extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate wapc_guest as guest;
extern crate scopeguard;

use serde::ser::{Serializer, SerializeStruct};
use serde::{Deserialize, Serialize};
use serde_json::json;

use std::error::Error;
use std::fmt;
use std::fmt::Debug;

use std::str;

use guest::prelude::*;
use std::collections::HashMap;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
  static ref CALLMAP: Mutex<HashMap<String, HandlerFunction>> = Mutex::new(HashMap::new());
}


trait BaseError<T>{
  fn fmt(f: &mut std::fmt::Formatter) -> std::fmt::Result;
}

macro_rules! define_error {
  // `()` indicates that the macro takes no argument.
  ($class_name:ident,$global_static:ident) => {
    #[derive(Copy, Clone, Serialize)]
    pub struct $class_name {
    }
    impl Debug for $class_name {
      fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(f, "{}", $class_name::get_description());
      }
    }
    impl $class_name{
      fn get_description() -> &'static str{
        return $global_static;
      }
    }
  };
}

define_error!{OtherError, OTHER_ERROR_DESC}
define_error!{InvalidError, INVALID_ERROR_DESC}
define_error!{PermissionError, PERMISSION_ERROR_DESC}
define_error!{NotImplemented, NOT_IMPLEMENTED_ERROR_DESC}
define_error!{IOError, IO_ERROR_DESC}
define_error!{BadHttpParams, BAD_HHTP_PARAMS_ERROR_DESC}
define_error!{ExistError, EXIST_ERROR_DESC}
define_error!{NotExistError, NOT_EXIST_ERROR_DESC}
define_error!{IsDirError, IS_DIR_ERROR_DESC}
define_error!{NotDirError, NOT_DIR_ERROR_DESC}
define_error!{FinalizedError, FINALIZED_ERROR_DESC}
define_error!{NotFinalizedError, NOT_FINALIZED_ERROR_DESC}

pub static INVALID_ERROR_DESC: &str = "invalid";                              // invalid operation for this type of item.
pub static OTHER_ERROR_DESC: &str = "other";                              // invalid operation for this type of item.
pub static PERMISSION_ERROR_DESC: &str = "permission denied";                 // Permission denied.
pub static IO_ERROR_DESC: &str = "I/O error";                                 // I/O denied.
pub static EXIST_ERROR_DESC: &str = "item already exists";                    // Item already exists.
pub static NOT_EXIST_ERROR_DESC: &str = "item does not exist";                 // Item does not exist.
pub static IS_DIR_ERROR_DESC: &str = "item is a directory";                    // Item is a directory.
pub static NOT_DIR_ERROR_DESC: &str = "item is not a directory";               // Item is not a directory.
pub static FINALIZED_ERROR_DESC: &str = "item is already finalized";          // Part or content is already finalized.
pub static NOT_FINALIZED_ERROR_DESC: &str ="item is not finalized";            // Part or content is not yet finalized.
pub static BAD_HHTP_PARAMS_ERROR_DESC: &str = "invalid Http params specified";  // Bitcode call with invalid HttpParams
pub static NOT_IMPLEMENTED_ERROR_DESC: &str = "not implemented";                // Function not implemented

#[derive(Debug, Clone, Serialize, Copy)]
pub enum ErrorKinds{
  Other,
  NotImplemented,
  Invalid,
  Permission,
  IO,
  Exist,
  NotExist,
  IsDir,
  NotDir,
  Finalized,
  NotFinalized,
  BadHttpParams,
}

impl fmt::Display for ErrorKinds {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    return write!(f, "{:?}", self);
  }
}

struct NoSubError{
}

impl Error for NoSubError {
  fn description(&self) -> &str {
      &"No Sub Error"
  }
}

impl std::fmt::Display for NoSubError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      return write!(f,"No Sub Error");
  }
}

impl std::fmt::Debug for NoSubError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    return write!(f,"No Sub Error");
  }
}

#[derive(Clone)]
pub struct ElvError<T> {
    details: String,
    kind: ErrorKinds,
    description : String,
    json_error : Option<T>,
}

impl<T> ElvError<T>{
      pub fn new_json(msg: &str, kind: ErrorKinds, sub_err : T) -> ElvError<T> {
      ElvError{
        details:  msg.to_string(),
        kind:     kind,
        description: format!("{{ details : {}, kind : {} }}", msg, kind),
        json_error : Some(sub_err),
      }
    }
}

impl<T> ElvError<T>{
  fn new(msg: &str, kind: ErrorKinds) -> ElvError<T>{
    ElvError{
      details:  msg.to_string(),
      kind:     kind,
      description: format!("{{ details : {}, kind : {} }}", msg, kind),
      json_error: None,
    }
  }

}

trait Kind {
  fn kind(&self) -> ErrorKinds;
  fn desc(&self) -> String;
}

impl<T:Error> Kind for ElvError<T>{
  fn kind(&self) -> ErrorKinds{
    return self.kind;
  }
  fn desc(&self) -> String{
    return self.description.clone();
  }
}


impl<T> std::error::Error for ElvError<T> where
T: fmt::Display + Debug {
  fn description(&self) -> &str {
      match self.kind {
        ErrorKinds::BadHttpParams => return BadHttpParams::get_description(),
        ErrorKinds::Other => return OtherError::get_description(),
        ErrorKinds::NotImplemented => return NotImplemented::get_description(),
        ErrorKinds::Invalid => return InvalidError::get_description(),
        ErrorKinds::Permission => return PermissionError::get_description(),
        ErrorKinds::IO => return IOError::get_description(),
        ErrorKinds::Exist => return ExistError::get_description(),
        ErrorKinds::NotExist => return NotExistError::get_description(),
        ErrorKinds::IsDir => return IsDirError::get_description(),
        ErrorKinds::NotDir => return NotDirError::get_description(),
        ErrorKinds::Finalized => return FinalizedError::get_description(),
        ErrorKinds::NotFinalized => return NotFinalizedError::get_description(),
      }
  }
}

impl<T> fmt::Display for ElvError<T> where
T: fmt::Display + Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match &self.json_error{
       Some(e) => {return write!(f,"{{ details : {}, kind : {}, sub_error : {:?} }}", self.details, self.kind, e)},
       None => {return write!(f,"{{ details : {}, kind : {}, sub_error : {{}} }}", self.details, self.kind)}
      };
    }
}

impl<T:fmt::Display + Debug> Serialize for ElvError<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where T:fmt::Display + Debug, S:Serializer,
  {
    let mut state = serializer.serialize_struct("ElvError", 3)?;
    state.serialize_field("description", &self.details)?;
    state.serialize_field("op", &self.kind)?;
    match &self.json_error{
      Some(e) => {
        state.serialize_field("message", &format!("{:?}", &e))?;
      },
      None => {
        state.serialize_field("message", "no extra error info")?;
      },
    };
    state.end()
  }
}

impl<T>  Debug for ElvError<T>  where
T: fmt::Display + Debug {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self.json_error{
     Some(e) => {return write!(f,"{{ details : {}, kind : {}, sub_error : {:?} }}", self.details, self.kind, e)},
     None => {return write!(f,"{{ details : {}, kind : {}, sub_error : {{}} }}", self.details, self.kind)}
    };
  }
}

impl<T:Error> From<T> for ElvError<T> {
  fn from(err: T) -> ElvError<T> {
    ElvError::<T> {
          details: err.to_string(),
          description:"".to_string(),
          json_error:None,
          kind:ErrorKinds::Invalid,
      }
  }
}

#[derive(Serialize, Deserialize,  Clone, Debug)]
pub struct FileStream {
  pub stream_id:String,
  pub file_name:String,
}

#[derive(Serialize, Deserialize,  Clone, Debug)]
pub struct FileStreamSize {
  pub file_size:usize,
}
#[derive(Serialize, Deserialize,  Clone, Debug)]
pub struct JpcParams {
  pub http: HttpParams
}

#[derive(Serialize, Deserialize,  Clone, Debug)]
pub struct HttpParams {
  #[serde(default)]
  pub headers: HashMap<String, Vec<String>>,
  pub path: String,
  #[serde(default)]
  pub query: HashMap<String, Vec<String>>,
  pub verb: String,
  #[serde(default)]
  pub fragment: String,
  #[serde(default)]
  pub content_length : String,
  #[serde(default)]
  pub client_ip : String,
  #[serde(default)]
  pub self_url : String,
  #[serde(default)]
  pub proto : String,
  #[serde(default)]
  pub host : String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QInfo {
  pub hash: String,
  pub id: String,
  pub qlib_id: String,
  #[serde(rename = "type")]
  pub qtype: String,
  pub write_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Request {
  pub id: String,
  pub jpc: String,
  pub method: String,
  pub params: JpcParams,
  #[serde(rename = "qinfo")]
  pub q_info: QInfo,
}


#[derive(Serialize, Deserialize)]
pub struct Response {
  pub jpc: String,
  #[serde(rename = "params")]
  pub params : serde_json::Value,
  pub id:String,
  pub module:String,
  pub method:String,
}

#[derive(Debug, Clone)]
pub struct BitcodeContext<'a> {
  pub request: &'a Request,
  pub return_buffer: Vec<u8>,
}

type HandlerFunction = fn(bcc: & mut BitcodeContext) -> CallResult;

pub fn make_json_error<T:Error>(err:ElvError<T>) -> CallResult {
  console_log(&format!("error={}", err));
  let msg = json!(
    {"error" :  err }
  );
  let vr = serde_json::to_vec(&msg).unwrap();
  console_log(&format!("returning a test {}", str::from_utf8(&vr).unwrap()));
  return Ok(vr);
}


impl<'a> BitcodeContext<'a> {
  fn new(request: &'a Request) -> BitcodeContext<'a> {
    BitcodeContext {
      request: request,
      return_buffer : vec![],
    }
  }

  pub fn write_stream(id:String, stream:String,  src:&'a [u8], len: usize) -> CallResult {
    let mut actual_len = src.len();
    if len != usize::MAX {
      actual_len = len
    }
    let v = serde_json::json!(src[..actual_len]);
    return host_call(&id, &stream, &"Write".to_string(), &serde_json::to_vec(&v).unwrap());
  }

  pub fn write_stream_auto(id:String, stream:String,  src:&'a [u8]) -> CallResult {
    return host_call(&id, &stream, &"Write".to_string(), src);
  }

  pub fn callback(&'a self, status:usize, content_type:&str, size:usize) -> CallResult{
    let v = json!(
      {"http" : {
        "status": status,
        "headers": {
          "Content-Type": content_type,
          "Content-Length": size,
        }
        }
      }
    );
    let method  = "Callback";
    return self.call_function(method, v, "ctx");
  }

  pub fn read_stream(&'a mut self, stream_to_read:String, sz:usize) -> CallResult {
        let input = serde_json::json![sz];
        let input_json = serde_json::to_vec(&input)?;
        return host_call(self.request.id.as_str(),stream_to_read.as_str(), &"Read", &input_json);
  }

  pub fn temp_dir(&'a mut self) -> CallResult {
    let temp_dir_res = self.call("TempDir", &"{}", &"ctx".as_bytes())?;
    return Ok(temp_dir_res);
  }

  pub fn close_stream(&'a self, sid : String) -> CallResult{
    return self.call_function(&"CloseStream", serde_json::Value::String(sid), &"ctx");
  }

  pub fn make_success(&'a self, msg:&str, id:&str) -> CallResult {
    let js_ret = json!({"jpc":"1.0", "id": id, "result" : msg});
    let v = serde_json::to_vec(&js_ret).unwrap();
    console_log(&format!("returning : {}", std::str::from_utf8(&v).unwrap()));
    return Ok(v);
  }

  pub fn make_success_json(&'a self, msg:&serde_json::Value, id:&str) -> CallResult {
    let js_ret = json!({"jpc":"1.0", "id": id, "result" : msg});
    let v = serde_json::to_vec(&js_ret).unwrap();
    console_log(&format!("returning : {}", std::str::from_utf8(&v).unwrap()));
    return Ok(v);
  }

  pub fn make_error(&'a self, msg:&str, _id:&str) -> CallResult {
    return make_json_error(ElvError::<NoSubError>::new(msg , ErrorKinds::Invalid));
  }

  pub fn make_success_bytes(&'a self, msg:&[u8], id:&str) -> CallResult {
    let res:serde_json::Value = serde_json::from_slice(msg).unwrap();
    let js_ret = json!({"jpc":"1.0", "id": id, "result" : res});
    let v = serde_json::to_vec(&js_ret)?;
    return Ok(v);
  }

  pub fn sqmd_get_json(&'a self, s:&'a str) -> CallResult {
    let sqmd_get = json!({"path": s});
    let method = "SQMDGet";
    return self.call_function(&method, sqmd_get, &"core");
  }

  pub fn proxy_http(&'a self, v:serde_json::Value) -> CallResult {
    let method = "ProxyHttp";
    let proxy_result = self.call_function(&method, v, &"ext")?;
    let id = self.request.id.clone();
    return self.make_success_bytes(&proxy_result, &id);
  }

  fn call(&'a mut self, ns: &str, op: &str, msg: &[u8]) -> CallResult{
    return host_call(self.request.id.as_str(),ns,op,msg);
  }

  pub fn call_function(&'a self, fn_name:&str , params:serde_json::Value, module:&str) -> CallResult {
    let response = &Response{
      jpc:"1.0".to_string(),
      id:self.request.id.clone(),
      module:module.to_string(),
      method : fn_name.to_string(),
      params:params,
    };
    let call_val = serde_json::to_vec(response)?;
    let call_str = serde_json::to_string(response)?;

    console_log(&format!("CALL STRING = {}", call_str));
    return host_call(self.request.id.as_str(),module, fn_name, &call_val);
  }
    // NewStream creates a new stream and returns its ID.
    pub fn new_stream(&'a self) -> String {
      let v = json!({});
      let strm = self.call_function("NewStream", v, "ctx").unwrap_or_default();
      let strm_json:serde_json::Value = serde_json::from_slice(&strm).unwrap_or_default();
      let sid:String = strm_json["stream_id"].to_string();
      return sid;
    }

    pub fn new_file_stream(&'a self) -> FileStream{
      let v = json!({});
      return serde_json::from_slice(&self.call_function("NewFileStream", v, "ctx").unwrap()).unwrap();
    }

    pub fn ffmpeg_run(&'a self, cmdline:Vec<&str>) -> CallResult {
      let params = json!({
        "stream_params" : cmdline
      });
      return self.call_function( "FFMPEGRun", params, "ext");
    }


    pub fn q_download_file(&'a mut self, path:&str, hash_or_token:&str) -> CallResult{
      console_log(&format!("q_download_file path={} token={}",path,hash_or_token));
      let sid = self.new_stream();
      if sid == ""{
        return self.make_error("Unable to find stream_id", "");
      }
      let j = json!({
        "stream_id" : sid,
        "path" : path,
        "hash_or_token": hash_or_token,
      });

      let ret = self.call_function("QFileToStream", j, "core");
      let v:serde_json::Value;
      match ret{
        Err(e) => return Err(e),
        Ok(e) => v = serde_json::from_slice(&e).unwrap_or_default()
      }

      let jtemp = v.to_string();
      console_log(&format!("json={}", jtemp));
      let written = v["written"].as_u64().unwrap();

      if written != 0 {
        return self.read_stream(sid, written as usize);
      }
      return self.make_error("failed to write data", "");

    }

    pub fn file_to_stream(&'a self, filename:&str, stream:&str) -> CallResult {
      let param = json!({ "stream_id" : stream, "path" : filename});
      return self.call_function("FileToStream", param, "core");
    }

    pub fn file_stream_size(&'a self,filename:&str) -> usize {
      console_log("file_stream_size");
      let ret:Vec<u8> = match self.call_function("FileStreamSize", json!({"file_name" : filename}), "ctx"){
         Ok(m) =>{ m }
         Err(_e) => {
           let j:FileStreamSize = serde_json::from_value(json!({"file_size" : 0})).unwrap();
           return j.file_size;
          }
      };

      match serde_json::from_slice::<FileStreamSize>(&ret){
        Ok(msize) => {
          console_log(&format!("FileStream returned={}", msize.file_size));
          return msize.file_size;
        }
        Err(_e) => {
          console_log("Err from FileStreamSize");
          return 0;
        }
      };
    }

}

#[no_mangle]
pub fn register_handler(name: &str, h: HandlerFunction) {
  CALLMAP.lock().unwrap().insert(name.to_string(), h);
}

#[no_mangle]
pub fn jpc<'a>(_msg: &'a [u8]) -> CallResult {
  console_log(&"In jpc");
  let input_string = str::from_utf8(_msg)?;
  console_log(&format!("parameters = {}", input_string));
  let json_params: Request = match serde_json::from_str(input_string){
    Ok(m) => {m},
    Err(err) => {
      return make_json_error(ElvError::new_json("parse failed for http" , ErrorKinds::Invalid, err));
    }
  };
  console_log(&"Request parsed");
  let mut bcc = BitcodeContext::new(&json_params);
  console_log(&"Parameters parsed");
  let split_path: Vec<&str> = bcc.request.params.http.path.as_str().split('/').collect();
  console_log(&format!("splitpath={:?}", split_path));
  match CALLMAP.lock().unwrap().get(split_path[1]) {
    Some(f) => {
      match f(& mut bcc){
        Ok(m) => {
          console_log(&format!("here and m={:?}", m));
          return Ok(m)
        },
        Err(err) => {
          return make_json_error(ElvError::new_json("parse failed for http" , ErrorKinds::Invalid, &*err));
        }
      }
    }
    None => {console_log("HERE!!!"); return Err(Box::new(ElvError::<NoSubError>::new("No valid path provided",  ErrorKinds::BadHttpParams)));}
  };
}


