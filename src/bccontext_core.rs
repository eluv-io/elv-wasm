//! Context core is a logical separation of portions of the BitcodeContext that result in calls to qfab's core interfaces <br>
//! Most documentation will appear in the BitcodeModule and there is nothing else of interest here
//! This module is to organize the fabric APIs via a grouping of their corresoding function on the server

extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate thiserror;
extern crate wapc_guest as guest;

use crate::BitcodeContext;

use serde_json::json;

use std::collections::HashMap;
use std::str;

use guest::CallResult;

#[cfg(doc)]
use crate::{CreatePartResult, QList, QPart, QPartInfo, QPartList, QRef, WriteResult};

impl<'a> BitcodeContext {
    // CORE functions

    /// q_create_content creates a new content object locally.  The content will have a write token but will
    /// not be comitted to the fabric until calls to finalize and commit are made
    /// # Arguments
    /// * `qtype`-   a hash for the content type. Can also be "builtin" for built in bitcode
    /// * `meta`-    a HashMap containing the initial meta data for the object to be set at '/'
    /// # Returns
    /// utf8 bytes stream containing json
    /// ```json
    /// { "qid" : "idObj", "qwtoken" : "writeToken"}
    /// ```
    ///
    pub fn q_create_content(
        &'a self,
        qtype: &str,
        meta: &HashMap<&str, serde_json::Value>,
    ) -> CallResult {
        let msg = json!({
          "qtype" : qtype,
          "meta"  : meta,
        });
        self.call_function("QCreateContent", msg, "core")
    }

    /// q_list_content calculates a content fabric QList for the context's libid
    /// # Returns
    /// slice of u8 parseable to [QList]
    /// e.g.
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let qlist:elvwasm::QList = bcc.q_list_content().try_into()?;
    ///   let res = vec![];
    ///
    ///   // do stuff with the qlist
    ///   Ok(res)
    /// }
    /// ```
    ///
    pub fn q_list_content(&'a self) -> CallResult {
        self.call_function("QListContent", json!({}), "core")
    }

    /// q_list_content_for calculates a content fabric QList for a given libid
    /// # Arguments
    /// * `qlibid`-    libid to be listed
    /// # Returns
    /// slice of u8 parseable to [QList]
    /// e.g.
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.q_list_content_for(&bcc.request.q_info.qlib_id)?;
    ///   let qlist:elvwasm::QList = serde_json::from_str(std::str::from_utf8(&res).unwrap()).unwrap();
    ///   // do stuff with the qlist
    ///   Ok(res)
    /// }
    /// ```
    ///
    pub fn q_list_content_for(&'a self, qlibid: &str) -> CallResult {
        let j = json!(
          {
            "external_lib" : qlibid,
          }
        );

        self.call_function("QListContentFor", j, "core")
    }

    /// q_finalize_content finalizes a given write token
    /// # Arguments
    /// * `qwtoken` - a write token to finalize
    /// # Returns
    /// utf8 bytes stream containing json
    /// ```json
    /// { "qid" : "idObj", "qhash" : "newHash"}
    /// ```
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let mr:elvwasm::ModifyResult = bcc.q_modify_content().try_into()?;
    ///   // ... process content
    ///   let res = vec![];
    ///   let hash = bcc.q_finalize_content(&mr.qwtoken)?;
    ///   Ok(res)
    /// }
    /// ```
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/external/src/lib.rs#L50)
    ///
    pub fn q_finalize_content(&'a self, qwtoken: &str) -> CallResult {
        let msg = json!(
          {
            "qwtoken" : qwtoken,
          }
        );
        self.call_function("QFinalizeContent", msg, "core")
    }

    /// q_commit_content finalizes a given write token
    /// # Arguments
    /// * `qhash` - a finalized hash
    /// # Returns
    /// * slice of [u8] that is empty
    /// e.g.
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.q_commit_content("hq__jd7sd655fffg7HrF76mHDolzzwe")?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    ///
    ///
    pub fn q_commit_content(&'a self, qhash: &str) -> CallResult {
        let msg = json!(
          {
            "qhash" : qhash,
          }
        );
        self.call_function("QFinalizeContent", msg, "core")
    }

    pub fn q_system_time(&'a self) -> CallResult {
        let msg = json!({});
        self.call_function("SystemTime", msg, "core")
    }

    /// q_modify_content enables edit on the implicit content of the context
    /// # Returns
    /// utf8 bytes stream containing json
    /// ```json
    /// { "qwtoken" : "writeTokenForEdit"}
    /// ```
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let mr:elvwasm::ModifyResult = bcc.q_modify_content().try_into()?;
    ///   let res = vec![];
    ///   Ok(res)
    /// }
    /// ```
    /// # Returns
    /// * slice of [u8]
    ///
    pub fn q_modify_content(&'a self) -> CallResult {
        self.call_function("QModifyContent", json!({"meta" : {}, "qtype" : ""}), "core")
    }
    /// q_part_list returns a list of parts in a given hash
    /// # Arguments
    /// * `String`-    object id or hash for the objects parts to be listed
    ///
    /// # Returns
    /// utf8 bytes containing json
    /// [QPartList]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/objtar/src/lib.rs#L93)
    ///
    pub fn q_part_list(&'a self, object_id_or_hash: String) -> CallResult {
        self.call_function(
            "QPartList",
            json!({ "object_id_or_hash": object_id_or_hash }),
            "core",
        )
    }

    /// write_part_to_stream writes the content of a part to to a fabric stream
    /// # Arguments
    /// * `stream_id`-    stream identifier from new_stream or the like
    /// * `off`-  offset into the file (0 based)
    /// * `len`-  length of part to write
    /// * `qphash` - part hash to write
    /// # Returns
    /// utf8 bytes stream containing json
    /// [WriteResult]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/objtar/src/lib.rs#L110)
    ///
    pub fn write_part_to_stream(
        &'a self,
        stream_id: String,
        qphash: String,
        qihot: String,
        offset: i64,
        length: i64,
        decrypt: bool,
    ) -> CallResult {
        let msg = json!(
          {
            "stream_id" :  stream_id,
            "off":offset,
            "len" : length,
            "qphash":qphash,
            "qihot" : qihot,
            "decrypt" : decrypt,
         }
        );
        self.call_function("QWritePartToStream", msg, "core")
    }

    /// write_qfile_to_stream writes the content of a fabric file to to a fabric stream
    /// # Arguments
    /// * `stream_id`-    stream identifier from new_stream or the like
    /// * `len`-  length of part to write
    /// * `qphash` - part hash to write
    /// # Returns
    /// utf8 bytes stream containing json
    /// [WriteResult]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/objtar/src/lib.rs#L110)
    ///
    pub fn write_qfile_to_stream(
        &'a self,
        stream_id: String,
        path: String,
        qihot: String,
    ) -> CallResult {
        let msg = json!(
          {
            "stream_id" :  stream_id,
            "path":path,
            "qihot" : qihot,
         }
        );
        self.call_function("QFileToStream", msg, "core")
    }

    /// q_create_part_from_stream creates a new part in a writeable object from a context stream.
    /// The content will be made locally but not published until finalized and committed
    /// # Arguments
    /// * `qwtoken`-   a write token to write the part
    /// * `stream_id`- the stream to write [BitcodeContext::new_stream]
    /// # Returns
    /// utf8 bytes stream containing json
    /// [CreatePartResult]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/external/src/lib.rs#L48)
    ///
    pub fn q_create_part_from_stream(&'a self, qwtoken: &str, stream_id: &str) -> CallResult {
        let msg = json!({
          "qwtoken" : qwtoken,
          "stream_id"  : stream_id,
        });
        self.call_function("QCreatePartFromStream", msg, "core")
    }

    /// q_file_to_stream writes the given qfile's content to the provided stream
    /// # Arguments
    /// * `stream_id`-  string identifier aquired from [BitcodeContext::new_stream]
    /// * `path` - string conatining the QFile path
    /// * `hash_or_token` - an optional string with a hash to operate on (defaults to the contexts content)
    /// # Returns
    /// utf8 bytes stream containing json
    /// [WriteResult]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/real-img/src/lib.rs#L63)
    ///
    pub fn q_file_to_stream(
        &'a self,
        stream_id: &str,
        path: &str,
        hash_or_token: &str,
    ) -> CallResult {
        let j = json!(
          {
            "stream_id" : stream_id,
            "path" : path,
            "hash_or_token" : hash_or_token
          }
        );

        self.call_function("QFileToStream", j, "core")
    }

    /// file_to_stream directs a fabric file (filename) to a fabric stream (stream)
    /// filename - name of the fabric file (see new_file_stream)
    /// stream - name of the stream that receives the file stream (see new_stream)
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let wr:elvwasm::WriteResult = bcc.file_to_stream("myfile", "someStreamId").try_into()?;
    ///   bcc.log_info(&format!("file written {}", wr.written));
    ///   let res = vec![];
    ///   Ok(res)
    /// }
    /// ```
    pub fn file_to_stream(&'a self, filename: &str, stream: &str) -> CallResult {
        let param = json!({ "stream_id" : stream, "path" : filename});
        self.call_function("FileToStream", param, "core")
    }

    /// q_create_file_from_stream creates a qfile from the cotents of a bitcode stream
    /// # Arguments
    /// * `stream_id`-    stream identifier from new_stream or the like
    /// * `qwtoken`-  write token (will use context's if "" is provided)
    /// * `path`-  qfile path
    /// * `mime` - MIME type of the file
    /// * `size` - size of the file in bytes
    /// # Returns
    /// slice of [u8] parseable to [QPartInfo]
    pub fn q_create_file_from_stream(
        &'a self,
        stream_id: &str,
        qwtoken: &str,
        path: &str,
        mime: &str,
        size: i64,
    ) -> CallResult {
        let msg = json!(
          {
            "stream_id" :  stream_id,
            "qwtoken":qwtoken,
            "path" : path,
            "mime": mime,
            "size": size,
         }
        );
        self.call_function("QCreateFileFromStream", msg, "core")
    }

    /// q_create_q_state_store creates a new state store in the fabric
    /// # Returns
    /// utf8 bytes stream containing a string with the state store id
    /// e.g.
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.q_create_q_state_store()?;
    ///   let ssID = std::str::from_utf8(&res)?;
    ///   Ok(res)
    /// }
    /// ```
    pub fn q_create_q_state_store(&'a self) -> CallResult {
        self.call_function("QCreateQStateStore", json!({}), "core")
    }

    /// q_get_versions lists the content versions with additional details for the given qid in context's libid.
    /// # Arguments
    /// * `qid`-                 id of object to get versions for
    /// * `with_details`-        whether to retrieve additional info (currently content type hash, qlib ID, and size stats)
    /// # Returns
    /// [Vec] parseable to [QRef]
    /// e.g.
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.q_get_versions("id_someQID", true)?;
    ///   let qVersions:elvwasm::QRef = serde_json::from_str(std::str::from_utf8(&res).unwrap()).unwrap();
    ///   // Do stuff with qVersions
    ///   Ok(res)
    /// }
    /// ```
    pub fn q_get_versions(&'a self, qid: &str, with_details: bool) -> CallResult {
        let j = json!({
          "qid": qid,
          "with_details": with_details
        });
        self.call_function("QGetVersions", j, "core")
    }

    /// q_checksum_part calculates a checksum of a given content part.
    /// # Arguments
    /// * `sum_method`-    checksum method ("MD5" or "SHA256")
    /// * `qphash`-        hash of the content part to checksum
    /// # Returns
    /// the checksum as hex-encoded string
    pub fn q_checksum_part(&'a self, sum_method: &str, qphash: &str) -> CallResult {
        let j = json!(
          {
            "method" : sum_method,
            "qphash" : qphash
          }
        );

        self.call_function("QCheckSumPart", j, "core")
    }

    /// q_checksum_file calculates a checksum of a file in a file bundle
    /// # Arguments
    /// * `sum_method`-    checksum method ("MD5" or "SHA256")
    /// * `file_path`-     the path of the file in the bundle
    /// # Returns
    /// the checksum as hex-encoded string
    pub fn q_checksum_file(&'a self, sum_method: &str, file_path: &str) -> CallResult {
        let j = json!(
          {
            "method" : sum_method,
            "file_path" : file_path,
          }
        );

        self.call_function("QCheckSumFile", j, "core")
    }

    /// The following sqmd_* based functions all work on the premise that a bitcode context represents a single content
    /// in the fabric.  As such, each content has meta data that is directly associated.  This meta forms a standard tree
    /// at the `/` root level.
    ///

    /// sqmd_set_json ets the metadata at path
    /// # Arguments
    /// * `path` : path to the meta data
    /// * `val` : serde_json::Value to set
    /// # Returns
    /// * error only no success return
    /// ```rust
    /// use serde_json::json;
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   bcc.sqmd_set_json("/some_key", &json!({"foo" : "bar"}))?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn sqmd_set_json(&'a self, path: &'a str, val: &serde_json::Value) -> CallResult {
        let sqmd_set = json!
        (
          {
            "meta": val,
            "path": path,
          }
        );
        self.call_function("SQMDSet", sqmd_set, "core")
    }

    /// sqmd_merge_json merges the metadata at path
    /// # Arguments
    /// * `path` : path to the meta data
    /// * `val` : serde_json::Value to merge
    /// # Returns
    /// * error only no success return
    /// ```rust
    /// use serde_json::json;
    ///
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   bcc.sqmd_merge_json("/some_key", r#"{{"foo" : "bar"}}"#)?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn sqmd_merge_json(&'a self, path: &'a str, json_str: &'a str) -> CallResult {
        let sqmd_merge = json!
        (
          {
            "meta":json_str,
            "path": path,
          }
        );
        self.call_function("SQMDMerge", sqmd_merge, "core")
    }

    /// sqmd_delete_json deletes the metadata at path
    /// # Arguments
    /// * `path` : path to the meta data
    /// # Returns
    /// * error only no success return
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   bcc.sqmd_delete_json("/some_key")?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn sqmd_delete_json(&'a self, path: &'a str) -> CallResult {
        let sqmd_delete = json!
        (
          {
            "path": path,
          }
        );
        self.call_function("SQMDDelete", sqmd_delete, "core")
    }

    /// sqmd_clear_json clears the metadata at path
    /// # Arguments
    /// * `path` : path to the meta data
    /// # Returns
    /// * nothing only error on failure
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   bcc.sqmd_clear_json("/some_key")?; // this will blast some_key and all its descendants
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn sqmd_clear_json(&'a self, path: &'a str) -> CallResult {
        let sqmd_clear = json!
        (
          {
            "path": path,
          }
        );
        self.call_function("SQMDClear", sqmd_clear, "core")
    }

    /// sqmd_get_json gets the metadata at path
    /// # Arguments
    /// * `path` : path to the meta data
    /// # Returns
    /// * UTF8 [u8] slice containing json
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.sqmd_get_json("/some_key")?;
    ///   let v:serde_json::Value = serde_json::from_slice(&res.clone()).unwrap();
    ///   let mut meta = v.as_object().unwrap();
    ///   Ok(res)
    /// }
    /// ```
    pub fn sqmd_get_json(&'a self, path: &'a str) -> CallResult {
        let sqmd_get = json!({ "path": path });
        self.call_function("SQMDGet", sqmd_get, "core")
    }

    /// sqmd_get_json_resolve gets the metadata at path resolving all links
    /// # Arguments
    /// * `path` : path to the meta data
    /// # Returns
    /// * UTF8 [u8] slice containing json
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.sqmd_get_json_resolve("/some_key")?;
    ///   let v:serde_json::Value = serde_json::from_slice(&res.clone()).unwrap();
    ///   let mut meta = v.as_object().unwrap();
    ///   Ok(res)
    /// }
    /// ```
    pub fn sqmd_get_json_resolve(&'a self, path: &'a str) -> CallResult {
        let sqmd_get = json!({ "path": path });
        self.call_function("SQMDGetJSONResolve", sqmd_get, "core")
    }

    /// sqmd_get_json_external gets the metadata at path from another content
    /// # Arguments
    /// * `path` : path to the meta data
    /// * `qhash`: hash of external content
    /// # Returns
    /// * UTF8 [u8] slice containing json
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.sqmd_get_json_external("libid4556", "hq_bad2a1ac0a2923ad85e1489736701c06320242a9", "/some_key")?;
    ///   let mut meta_str: String = String::from_utf8(res.clone())?;
    ///   Ok(res)
    /// }
    /// ```
    pub fn sqmd_get_json_external(&'a self, qlibid: &str, qhash: &str, path: &str) -> CallResult {
        let sqmd_get = json!
        (
          {
            "path": path,
            "qlibid":qlibid,
            "qhash":qhash,
          }
        );
        self.call_function("SQMDGetExternal", sqmd_get, "core")
    }

    /// fetch_link_reader resolves the fabric file link
    /// # Arguments
    /// * `link` : fabric link
    /// # Returns
    /// * UTF8 [u8] slice containing the resolved link
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.fetch_link_reader(serde_json::from_str("/qfab/hq_somehash/file/assets/foo.jpg")?)?;
    ///   Ok(res)
    /// }
    /// ```
    pub fn fetch_link_reader(&'a self, link: serde_json::Value) -> CallResult {
        let fetch_params = json!
        (
          {
            "link": link,
            "use_reader": true,
          }
        );
        self.call_function("FetchLink", fetch_params, "core")
    }

    /// fetch_link resolves the fabric file link
    /// # Arguments
    /// * `link` : fabric link
    /// # Returns
    /// * UTF8 [u8] slice containing the resolved link
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.fetch_link(serde_json::from_str("/qfab/hq_somehash/file/assets/foo.jpg")?)?;
    ///   Ok(res)
    /// }
    /// ```
    pub fn fetch_link(&'a self, link: serde_json::Value) -> CallResult {
        let fetch_params = json!
        (
          {
            "link": link,
            "use_reader": false,
          }
        );
        self.call_function("FetchLink", fetch_params, "core")
    }

    //
    /// sqmd_query queries the meta-data with the given JSONPath query expression.
    /// # Arguments
    /// * `query` : JSONPath query expression
    /// # Returns
    /// * UTF8 [u8] slice containing json
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.sqmd_query("$['some'].value[0].description")?;
    ///   let v:serde_json::Value = serde_json::from_slice(&res.clone()).unwrap();
    ///   let mut meta = v.as_object().unwrap();
    ///   Ok(res)
    /// }
    /// ```
    pub fn sqmd_query(&'a self, query: &'a str) -> CallResult {
        let sqmd_query = json!({ "query": query });
        self.call_function("SQMDQuery", sqmd_query, "core")
    }

    /// qss_set sets data into the Q state store
    /// # Arguments
    /// * `qssid`- string identifier aquired from [BitcodeContext::q_create_q_state_store]
    /// * `key` - string
    /// * `val` - string value to store
    /// # [Vec]Returns
    /// Nothing error only
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.q_create_q_state_store()?;
    ///   let ssID = std::str::from_utf8(&res)?;
    ///   bcc.qss_set(ssID, "akey", "avalue")?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn qss_set(&'a self, qssid: &str, key: &str, val: &str) -> CallResult {
        let j = json!(
          {
            "qssid" : qssid,
            "key" : key,
            "val" : val
          }
        );

        self.call_function("QSSSet", j, "core")
    }

    /// qss_get gets data from the Q state store
    /// # Arguments
    /// * `qssid`- string identifier aquired from [BitcodeContext::q_create_q_state_store]
    /// * `key` - string
    /// # Returns
    /// [Vec] containing string value
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   let res = bcc.qss_get("sid_648nfjfh5666nmjejh", "akey")?;
    ///   let strVal = std::str::from_utf8(&res)?;
    ///   Ok(res)
    /// }
    /// ```
    pub fn qss_get(&'a self, qssid: &str, key: &str) -> CallResult {
        let j = json!(
          {
            "qssid" : qssid,
            "key" : key,
          }
        );

        self.call_function("QSSGet", j, "core")
    }

    /// qss_get deletes data from the Q state store
    /// # Arguments
    /// * `qssid`- string identifier aquired from [BitcodeContext::q_create_q_state_store]
    /// * `key` - string
    /// # Returns
    /// Nothing error only
    /// ```rust
    /// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
    ///   bcc.qss_delete("sid_648nfjfh5666nmjejh", "akey")?;
    ///   Ok("SUCCESS".to_owned().as_bytes().to_vec())
    /// }
    /// ```
    pub fn qss_delete(&'a self, qssid: &str, key: &str) -> CallResult {
        let j = json!(
          {
            "qssid" : qssid,
            "key" : key,
          }
        );

        self.call_function("QSSDelete", j, "core")
    }
}
