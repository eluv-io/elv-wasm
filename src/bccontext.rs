extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate thiserror;
extern crate wapc_guest as guest;

use crate::{get_cargo_version, make_json_error, ErrorKinds};
use crate::{FileStream, NewStreamResult, Request, Response};

use serde_json::json;

use std::fmt::Debug;

use std::collections::HashMap;
use std::str;

use guest::prelude::*;

pub fn convert<'b, T>(cr: &'b CallResult) -> Result<T, Box<dyn std::error::Error + Sync + Send>>
where
    T: serde::Deserialize<'b>,
{
    match cr {
        Ok(r) => Ok(serde_json::from_slice(r)?),
        Err(e) => Err(Box::new(ErrorKinds::Invalid(e.to_string()))),
    }
}

/// This structure encapsulates all communication with the Eluvio content fabric.  A new BitcodeContext
/// is automatically created during the processing of the http request.  During initialization, all context
/// data is acquired from the http request.
#[derive(Debug, Clone, Default)]
pub struct BitcodeContext {
    pub request: Request,
}

impl<'a> BitcodeContext {
    pub fn new(request: Request) -> BitcodeContext {
        BitcodeContext { request }
    }

    pub fn log_info(&'a self, s: &str) -> CallResult {
        self.call_function("Log", json!({"level" : "INFO", "msg" : s}), "ctx")
    }

    pub fn log_debug(&'a self, s: &str) -> CallResult {
        self.call_function("Log", json!({"level" : "DEBUG", "msg" : s}), "ctx")
    }

    pub fn log_warn(&'a self, s: &str) -> CallResult {
        self.call_function("Log", json!({"level" : "WARN", "msg" : s}), "ctx")
    }

    pub fn log_error(&'a self, s: &str) -> CallResult {
        self.call_function("Log", json!({"level" : "ERROR", "msg" : s}), "ctx")
    }

    /// write_stream writes a u8 slice of specified length to a fabric stream
    /// # Arguments
    /// * `id`-    a unique identifier (can use BitcodeContext's request id)
    /// * `stream`-  the fabric stream to write to [BitcodeContext::new_stream]
    /// * `src`-  a u8 slice to write
    /// * `len` -  length of the slice to write
    /// # Returns
    /// utf8 bytes stream containing json
    /// { "written" : bytes }
    ///
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/external/src/lib.rs#L111)
    ///
    pub fn write_stream(&'a self, stream: &str, src: &'a [u8]) -> CallResult {
        host_call(&self.request.id, stream, "Write", src)
    }

    /// read_stream_chunked reads usize bytes ub chunks of chunksize from a fabric stream returning a slice of [u8]
    /// # Arguments
    /// * `stream_to_read`-  the fabric stream to read from
    /// * `sz`-  usize size of bytes
    /// * `chunksize`-  usize size of bytes to read in each chunk
    /// # Returns
    ///   byte slice of the read stream
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/objtar/src/lib.rs#L112)
    ///
    pub fn read_stream_chunked(&'a self, stream_to_read: String, chunk_size: usize) -> CallResult {
        self.log_debug(&format!("imput len = {}", chunk_size))?;
        let mut data: Vec<u8> = Vec::new();
        loop {
            let partial = match self.read_stream(stream_to_read.clone(), chunk_size) {
                Ok(p) => p,
                Err(e) => {
                    self.log_error(&format!("Error reading stream: {e}"))?;
                    break;
                }
            };
            if partial.is_empty() {
                break;
            }
            data.extend_from_slice(&partial);
        }
        Ok(data)
        // Read was previously being called with a json object containing the length
        // resulting in a full read of the part and itsd subsequent return as a base64 encoded string
        // The new convention is to call Reader which will return a byte slice or error
    }

    /// read_stream reads usize bytes from a fabric stream returning a slice of [u8]
    /// # Arguments
    /// * `stream_to_read`-  the fabric stream to read from
    /// * `sz`-  usize size of bytes
    /// # Returns
    ///   byte slice of the read stream
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/objtar/src/lib.rs#L112)
    ///
    pub fn read_stream(&'a self, stream_to_read: String, sz: usize) -> CallResult {
        self.log_debug(&format!("imput len = {}", sz))?;
        // Read was previously being called with a json object containing the length
        // resulting in a full read of the part and itsd subsequent return as a base64 encoded string
        // The new convention is to call Reader which will return a byte slice or error
        host_call(
            self.request.id.as_str(),
            stream_to_read.as_str(),
            "Reader",
            serde_json::to_string(&json!({ "len": sz }))?.as_bytes(),
        )
    }

    /// read_stream_inline reads usize bytes from a fabric stream returning a slice of [u8]
    /// # Arguments
    /// * `stream_to_read`-  the fabric stream to read from
    /// * `sz`-  usize size of bytes
    /// # Returns
    /// utf8 bytes stream containing json
    /// {
    ///   "return" : { "read" : byte-count-read },
    ///   "result" : "base64 encoded string"
    ///  }
    ///  this api is provided for backward compatibility with the previous read_stream api
    /// and should be avoided in new code
    pub fn read_stream_inline(&'a self, stream_to_read: String, sz: usize) -> CallResult {
        self.log_debug(&format!("imput len = {}", sz))?;
        host_call(
            self.request.id.as_str(),
            stream_to_read.as_str(),
            "Read",
            serde_json::to_string(&json!({ "len": sz }))?.as_bytes(),
        )
    }

    /// callback issues a Callback on the fabric setting up an expectation that the output stream
    /// contains a specified sized buffer
    /// # Arguments
    /// * `status`-    the http status of the call
    /// * `content-type`-     output buffer contents
    /// * `size`-  size of the output contents
    /// # Returns
    /// the checksum as hex-encoded string
    ///
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/external/src/lib.rs#L133)
    ///
    pub fn callback(&'a self, status: usize, content_type: &str, size: usize) -> CallResult {
        let v = json!(
          {"http" : {
            "status": status,
            "headers": {
              "Content-Type": [content_type],
              "Content-Length": [size.to_string()],
              "X-Content-Fabric-Bitcode-Version": vec![get_cargo_version()],
            }
            }
          }
        );
        let method = "Callback";
        self.call_function(method, v, "ctx")
    }

    /// callback_disposition issues a Callback on the fabric setting up an expectation that the output stream
    /// contains a specified sized buffer
    /// # Arguments
    /// * `status`-    the http status of the call
    /// * `content-type`-     output buffer contents
    /// * `size`-  size of the output contents
    /// * `disp`-  content disposition
    /// # Returns
    /// the checksum as hex-encoded string
    ///
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/external/src/lib.rs#L133)
    ///
    pub fn callback_disposition(
        &'a self,
        status: usize,
        content_type: &str,
        size: usize,
        disp: &str,
        version: &str,
    ) -> CallResult {
        let mut v = json!(
          {"http" : {
            "status": status,
            "headers": {
              "Content-Type": vec![content_type],
              "Content-Length": vec![size.to_string()],
              "Content-Disposition": vec![disp],
              "X-Content-Fabric-Bitcode-Version": vec![version, get_cargo_version()],
            }
            }
          }
        );
        if size == 0 {
            v = json!(
              {"http" : {
                "status": status,
                "headers": {
                  "Content-Type": vec![content_type],
                  "Content-Disposition": vec![disp],
                  "X-Content-Fabric-Bitcode-Version": vec![version, get_cargo_version()],
                }
                }
              }
            );
        }
        let method = "Callback";
        self.call_function(method, v, "ctx")
    }

    pub fn make_success(&'a self, msg: &str) -> CallResult {
        self.make_success_json(&json!(msg))
    }

    pub fn make_success_json(&'a self, msg: &serde_json::Value) -> CallResult {
        let js_ret = json!({
          "result" : msg,
          "jpc" : "1.0",
          "id"  : self.request.id,
        });
        let v = serde_json::to_vec(&js_ret)?;
        Ok(v)
    }

    pub fn make_error(&'a self, msg: &'static str) -> CallResult {
        make_json_error(ErrorKinds::Invalid(msg.to_string()), &self.request.id)
    }

    pub fn make_error_with_kind(&'a self, kind: ErrorKinds) -> CallResult {
        make_json_error(kind, &self.request.id)
    }

    pub fn make_error_with_error<T>(&'a self, kind: ErrorKinds, _err: T) -> CallResult {
        make_json_error(kind, &self.request.id)
    }

    pub fn make_success_bytes(&'a self, msg: &[u8], id: &str) -> CallResult {
        let res: serde_json::Value = serde_json::from_slice(msg)?;
        let js_ret = json!({"jpc":"1.0", "id": id, "result" : res});
        let v = serde_json::to_vec(&js_ret)?;
        Ok(v)
    }

    pub fn call(&'a self, ns: &str, op: &str, msg: &[u8]) -> CallResult {
        host_call(self.request.id.as_str(), ns, op, msg)
    }
    /// call_function - enables the calling of fabric api's
    /// # Arguments
    /// * `fn_name` - the fabric api to call e.g. QCreateFileFromStream
    /// * `params` - a json block to pass as parameters to the function being called
    /// * `module` - one of {"core", "ctx", "ext"} see [fabric API]
    ///
    ///  This is the main workhorse function for the invoking of fabric bitcode APIs
    ///  wherein all the outer wrapper functions merely call this with the appropriate json parameters
    pub(crate) fn call_function(
        &'a self,
        fn_name: &str,
        params: serde_json::Value,
        module: &str,
    ) -> CallResult {
        let response = &Response {
            jpc: "1.0".to_string(),
            id: self.request.id.clone(),
            module: module.to_string(),
            method: fn_name.to_string(),
            params,
        };
        let call_val = serde_json::to_vec(response)?;

        let call_ret_val = host_call(self.request.id.as_str(), module, fn_name, &call_val)?;
        let j_res: serde_json::Value = serde_json::from_slice(&call_ret_val)?;
        if !j_res.is_object() {
            return Ok(call_ret_val);
        }
        match j_res.get("result") {
            Some(x) => {
                let r = serde_json::to_vec(&x)?;
                Ok(r)
            }
            None => match j_res.get("error") {
                Some(x) => {
                    let r = serde_json::to_vec(&x)?;
                    Ok(r)
                }
                None => Ok(call_ret_val),
            },
        }
    }

    /// call_external_bitcode - enables the calling of fabric api's
    /// # Arguments
    /// * `function` - the function to call on the external bitcode
    /// * `args` -  the argumaents to pass the external function
    /// * `object_hash`  - the content object containing the external bitcode part
    /// * `code_part_hash` - the code part for the external bitcode
    ///
    ///   [Example](https://github.com/eluv-io/elv-wasm/blob/019b88ac27635d5022c2211751f6af5957df2463/samples/external/src/lib.rs#L101)
    ///
    /// ```
    /// use elvwasm::ErrorKinds;
    /// extern crate wapc_guest as guest;
    /// use serde_json::{json, Value};
    /// use std::str;
    /// use guest::CallResult;
    /// use elvwasm::ExternalCallResult;
    ///
    /// fn do_something<'s>(bcc: &'s elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///     let http_p = &bcc.request.params.http;
    ///     let qp = &http_p.query;
    ///     let id = &bcc.request.id;
    ///     let img_hash = &qp.get("img_hash").ok_or(ErrorKinds::Invalid("img_hash not present".to_string()))?[0];
    ///     let img_obj= &qp.get("img_obj").ok_or(ErrorKinds::Invalid("img_hash not present".to_string()))?[0];
    ///     let tar_hash = &qp.get("tar_hash").ok_or(ErrorKinds::Invalid("tar_hash not present".to_string()))?[0];
    ///     bcc.log_info(&format!("img_hash ={img_hash:?} tar_hash = {tar_hash:?}"))?;
    ///     let params = json!({
    ///         "http" : {
    ///             "verb" : "some",
    ///             "headers": {
    ///                 "Content-type": [
    ///                     "application/json"
    ///                 ]
    ///             },
    ///             "path" : "/image/default/assets/birds.jpg",
    ///             "query" : {
    ///                 "height" : ["200"],
    ///             },
    ///         },
    ///     });
    ///     let img = bcc.call_external_bitcode("image", &params, img_obj, img_hash)?;
    ///     let exr:ExternalCallResult = serde_json::from_slice(&img)?;
    ///     let imgbits = base64::decode(&exr.fout)?;
    ///     Ok(imgbits)
    /// }
    /// ```
    pub fn call_external_bitcode(
        &'a self,
        function: &str,
        args: &serde_json::Value,
        object_hash: &str,
        code_part_hash: &str,
    ) -> CallResult {
        let params = json!({ "module": "".to_string() ,"function": function,  "params" : args, "object_hash" : object_hash, "code_part_hash" : code_part_hash});
        let call_val = serde_json::to_vec(&params)?;

        let call_ret_val = host_call(
            self.request.id.as_str(),
            "ctx",
            "CallExternalBitcode",
            &call_val,
        )?;
        let j_res: serde_json::Value = serde_json::from_slice(&call_ret_val)?;
        if !j_res.is_object() {
            return Ok(call_ret_val);
        }
        match j_res.get("result") {
            Some(x) => {
                let r = serde_json::to_vec(&x)?;
                Ok(r)
            }
            None => match j_res.get("error") {
                Some(x) => {
                    let r = serde_json::to_vec(&x)?;
                    Ok(r)
                }
                None => Ok(call_ret_val),
            },
        }
    }

    /// close_stream closes the fabric stream
    /// - sid:    the sream id (returned from one of the new_file_stream or new_stream)
    ///
    /// Returns the checksum as hex-encoded string
    ///
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/b6a5e5b79022d52138b29aa1779b44f29f65ef51/samples/external/src/lib.rs#L60)
    ///
    pub fn close_stream(&'a self, sid: String) -> CallResult {
        self.call_function("CloseStream", json!({ "stream_id": sid }), "ctx")
    }

    /// new_stream creates a new fabric bitcode stream.
    /// # Returns
    /// * output [u8] of format `{"stream_id" : id}` where id is a string
    ///
    /// [Example](https://github.com/eluv-io/elv-wasm/blob/b6a5e5b79022d52138b29aa1779b44f29f65ef51/samples/external/src/lib.rs#L57)
    ///
    pub fn new_stream(&'a self) -> CallResult {
        let v = json!({});
        self.call_function("NewStream", v, "ctx")
    }

    /// new_file_stream creates a new fabric file
    /// output [u8] of format where id and path are strings
    /// {
    ///   "stream_id": id,
    ///   "file_name": path
    /// }
    pub fn new_file_stream(&'a self) -> CallResult {
        let v = json!({});
        self.call_function("NewFileStream", v, "ctx")
    }

    /// q_download_file : downloads the file stored  at the fabric file location path for some content
    /// # Arguments
    /// *  `path` : fabric file location in the content
    /// *  `hash_or_token` : hash for the content containing the file
    ///
    pub fn q_download_file(&'a mut self, path: &str, hash_or_token: &str) -> CallResult {
        self.log_debug(&format!(
            "q_download_file path={path} token={hash_or_token}"
        ))?;
        let stream_main: NewStreamResult = self.new_stream().try_into()?;
        let sid = stream_main.stream_id.clone();
        if stream_main.stream_id.is_empty() {
            return self.make_error_with_kind(ErrorKinds::IO(format!(
                "Unable to find stream_id {}",
                &sid
            )));
        }
        defer! {
            let _ = self.close_stream(sid.clone());
        }
        let j = json!({
          "stream_id" : &sid,
          "path" : path,
          "hash_or_token": hash_or_token,
        });

        let v: serde_json::Value = match self.call_function("QFileToStream", j, "core") {
            Err(e) => {
                return self.make_error_with_kind(ErrorKinds::NotExist(format!(
                    "QFileToStream failed path={path}, hot={hash_or_token} sid={} e={e}",
                    stream_main.stream_id
                )))
            }
            Ok(e) => serde_json::from_slice(&e)?,
        };

        let written = match v["written"].as_u64() {
            Some(s) => s,
            None => return self.make_error("failed to unmarshal written count"),
        };

        if written != 0 {
            return self.read_stream(stream_main.stream_id, written as usize);
        }
        self.make_error_with_kind(ErrorKinds::NotExist(format!(
            "wrote 0 bytes, sid={} path={path}, hot={hash_or_token}",
            &stream_main.stream_id
        )))
    }

    /// q_upload_file : uploads the input data and stores it at the fabric file location as filetype mime
    /// # Arguments
    /// * `qwt` : a fabric write token
    /// *  `input_data` : a slice of u8 data
    /// *  `path` : fabric file location
    /// *  `mime` : MIME type to store the data as (eg gif)
    ///
    pub fn q_upload_file(
        &'a mut self,
        qwt: &str,
        input_data: &[u8],
        path: &str,
        mime: &str,
    ) -> CallResult {
        let sid = self.new_file_stream()?;
        let new_stream: FileStream = serde_json::from_slice(&sid)?;
        defer! {
          let _ = self.close_stream(new_stream.stream_id.clone());
        }
        let ret_s = self.write_stream(new_stream.clone().stream_id.as_str(), input_data)?;
        let written_map: HashMap<String, String> = serde_json::from_slice(&ret_s)?;
        let i: i32 = written_map["written"].parse()?;
        let j = json!({
          "qwtoken" : qwt,
          "stream_id": new_stream.stream_id,
          "path":path,
          "mime":mime,
          "size": i,
        });

        let method = "QCreateFileFromStream";
        self.call_function(method, j, "core")
    }

    /// file_stream_size computes the current size of a fabric file stream given its stream name
    ///     filename : the name of the file steam.  See new_file_stream.
    pub fn file_stream_size(&'a self, filename: &str) -> CallResult {
        self.log_debug("file_stream_size")?;
        self.call_function("FileStreamSize", json!({ "file_name": filename }), "ctx")
    }
}
