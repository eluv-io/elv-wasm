//! elvwasm contains and collects the bitcode extension API for the Eluvio content fabric. <br>
//! The library is intended to be built as wasm and the resultant part uploaded to the content fabric.
//! The main entry point for each client module is implemented by [jpc] which automatically creates and dispatches
//! requests to the [BitcodeContext] <br>
//! Example
/*!
  ```rust
    extern crate elvwasm;
    extern crate serde_json;
    use serde_json::json;

    use elvwasm::{implement_bitcode_module, jpc, register_handler, BitcodeContext, ErrorKinds};

    implement_bitcode_module!("proxy", do_proxy);

    fn do_proxy(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
        let http_p = &bcc.request.params.http;
        let qp = &http_p.query;
        bcc.log_debug(&format!("In DoProxy hash={} headers={:#?} query params={qp:#?}",&bcc.request.q_info.hash, &http_p.headers))?;
        let res = bcc.sqmd_get_json("/request_parameters")?;
        let mut meta_str: String = match String::from_utf8(res){
          Ok(m) => m,
          Err(e) => {return bcc.make_error_with_kind(ErrorKinds::Invalid(format!("failed to parse request params err = {e}")))}
        };
        meta_str = meta_str.replace("${API_KEY}", &qp["API_KEY"][0].to_string()).
          replace("${QUERY}", &qp["QUERY"][0].to_string()).
          replace("${CONTEXT}", &qp["CONTEXT"][0].to_string());
        bcc.log_debug(&format!("MetaData = {}", &meta_str))?;
        let req:serde_json::Map<String,serde_json::Value> = match serde_json::from_str::<serde_json::Map<String,serde_json::Value>>(&meta_str){
          Ok(m) => m,
          Err(e) => return bcc.make_error_with_kind(ErrorKinds::Invalid(format!("serde_json::from_str failed error = {e}"))),
        };
        let proxy_resp =  bcc.proxy_http(Some(json!({"request": req})))?;
        let proxy_resp_json:serde_json::Value = serde_json::from_str(std::str::from_utf8(&proxy_resp).unwrap_or("{}"))?;
        let client_response = serde_json::to_vec(&proxy_resp_json["result"])?;
        let id = &bcc.request.id;
        bcc.callback(200, "application/json", client_response.len())?;
        bcc.write_stream("fos", &client_response)?;
        bcc.make_success_json(&json!(
          {
              "headers" : "application/json",
              "body" : "SUCCESS",
              "result" : 0,
          }))
    }
  ```

  To Build binaries <br>
    *cargo build --all --features "host-wasm"* <br>
  To Build samples <br>
    *cd samples* <br>
    *cargo build --target wasm32-unknown-unknown* <br>
  <br>
  test <br>
    *target/debug/mock ./samples/target/wasm32-unknown-unknown/debug/deps/rproxy.wasm ./samples/fabric.json*
*/

extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate wapc_guest as guest;
#[macro_use(defer)]
extern crate scopeguard;

pub mod bccontext;
pub mod bccontext_core;
pub mod bccontext_error;
pub mod bccontext_ext;
pub mod bccontext_fabric_io;
pub mod bccontext_search;
pub mod bccontext_struct;

pub use self::bccontext::*;
pub use self::bccontext_error::*;
pub use self::bccontext_struct::*;

use std::str;

use guest::prelude::*;
use std::collections::HashMap;

use lazy_static::lazy_static;
use std::sync::Mutex;

#[derive(Clone)]
struct HandlerData<'a> {
    pub hf: HandlerFunction<'a>,
    pub req: Option<BitcodeContext>,
}

lazy_static! {
    static ref CALLMAP: Mutex<HashMap<String, HandlerData<'static>>> = Mutex::new(HashMap::new());
}

mod version {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

fn get_git_version() -> &'static str {
    version::COMMIT_HASH
}
fn get_cargo_version() -> &'static str {
    version::CARGO_VERSION
}

pub fn get_library_version() -> String {
    format!("cargo:{}--git:{}", get_cargo_version(), get_git_version())
}

#[macro_export]
macro_rules! register_handlers {
  () => {};
  ($handler_name:literal, $handler_func:ident $(,$more_name:literal, $more_func:ident )*) => {

    register_handler($handler_name, $handler_func);
    register_handlers!($( $more_name, $more_func ),* );
  }
}

/// This macro delivers the required initializtion of the eluvio wasm module
/// In addition the macro also registers a handler of the form
/// ```ignore
/// fn fn_name<'s, 'r>(bcc: &'s mut elvwasm::BitcodeContext<'r>) -> CallResult
///
/// implement_bitcode_module!("proxy", do_proxy, "image", do_image);
/// fn do_proxy<'s, 'r>(bcc: &'s mut elvwasm::BitcodeContext<'r>) -> CallResult {
///   return bcc.make_success("SUCCESS");
/// }
/// fn do_image<'s, 'r>(bcc: &'s mut elvwasm::BitcodeContext<'r>) -> CallResult {
///   return bcc.make_success("SUCCESS");
/// }
/// ```
#[macro_export]
macro_rules! implement_bitcode_module {
  ($handler_name:literal, $handler_func:ident $(, $more_lit:literal, $more:ident)*) => {
    extern crate wapc_guest as guest;

    use guest::{register_function, CallResult, console_log};
    use std::panic;
    use std::io;
    use elvwasm::register_handlers;

    #[no_mangle]
    pub extern "C" fn wapc_init() {
      register_handlers!($handler_name, $handler_func $(, $more_lit, $more)*);
      register_function("_JPC", jpc);
      panic::set_hook(Box::new(|panic_info| {
            if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                let _ = console_log(&format!("Panic in WASM!! {0}", s));
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                let _ = console_log(&format!("Panic in WASM!! {0}", s));
            } else {
                let _ = console_log(&format!("Panic in WASM!!"));
            }
        }));
    }
  }
}
#[cfg(target_os = "linux")]
mod c_exports {
    macro_rules! output_raw_pointers {
        ($raw_ptr:ident, $raw_len:ident) => {
            unsafe {
                std::str::from_utf8(std::slice::from_raw_parts($raw_ptr, $raw_len))
                    .unwrap_or("unable to convert")
            }
        };
    }

    #[no_mangle]
    pub extern "C" fn __console_log(ptr: *const u8, len: usize) {
        let out_str = output_raw_pointers!(ptr, len);
        println!("console output : {}", out_str);
    }
}

#[cfg(target_os = "macos")]
mod c_exports {
    macro_rules! output_raw_pointers {
        ($raw_ptr:ident, $raw_len:ident) => {
            unsafe {
                std::str::from_utf8(std::slice::from_raw_parts($raw_ptr, $raw_len))
                    .unwrap_or("unable to convert")
            }
        };
    }

    #[no_mangle]
    pub extern "C" fn __console_log(ptr: *const u8, len: usize) {
        let out_str = output_raw_pointers!(ptr, len);
        println!("console output : {}", out_str);
    }
    #[no_mangle]
    pub extern "C" fn __host_call(
        bd_ptr: *const u8,
        bd_len: usize,
        ns_ptr: *const u8,
        ns_len: usize,
        op_ptr: *const u8,
        op_len: usize,
        ptr: *const u8,
        len: usize,
    ) -> usize {
        let out_bd = output_raw_pointers!(bd_ptr, bd_len);
        let out_ns = output_raw_pointers!(ns_ptr, ns_len);
        let out_op = output_raw_pointers!(op_ptr, op_len);
        let out_ptr = output_raw_pointers!(ptr, len);
        println!(
            "host call bd = {} ns = {} op = {}, ptr={}",
            out_bd, out_ns, out_op, out_ptr
        );
        0
    }
    #[no_mangle]
    pub extern "C" fn __host_response(ptr: *const u8) {
        println!("host __host_response ptr = {:?}", ptr);
    }

    #[no_mangle]
    pub extern "C" fn __host_response_len() -> usize {
        println!("host __host_response_len");
        0
    }

    #[no_mangle]
    pub extern "C" fn __host_error_len() -> usize {
        println!("host __host_error_len");
        0
    }

    #[no_mangle]
    pub extern "C" fn __host_error(ptr: *const u8) {
        println!("host __host_error ptr = {:?}", ptr);
    }

    #[no_mangle]
    pub extern "C" fn __guest_response(ptr: *const u8, len: usize) {
        let out_resp = output_raw_pointers!(ptr, len);
        println!("host  __guest_response ptr = {}", out_resp);
    }

    #[no_mangle]
    pub extern "C" fn __guest_error(ptr: *const u8, len: usize) {
        let out_error = output_raw_pointers!(ptr, len);
        println!("host  __guest_error ptr = {}", out_error);
    }

    #[no_mangle]
    pub extern "C" fn __guest_request(op_ptr: *const u8, ptr: *const u8) {
        println!("host __guest_request op_ptr = {:?} ptr = {:?}", op_ptr, ptr);
    }
}

// The following are mearly intended to verify internal consistency.  There are no actual calls made
// but the tests verify that the json parsing of the http message is correct
#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    pub use self::bccontext::*;
    use super::*;
    use serde_json::*;

    fn handler_for_test(bcc: &mut BitcodeContext) -> CallResult {
        bcc.make_success("DONE")
    }

    #[test]
    fn test_basic_http() {
        register_handler("testing", handler_for_test);
        let test_json = json!({
          "id" : "dummydummy",
          "jpc" : "1.0",
          "method" : "testing",
          "params" : {
            "http" : {
              "path" : "/testing",
              "verb" : "GET",
            },
          },
          "qinfo" : {
            "qlib_id" : "idlib1234",
            "type" : "some_type",
          },
        });
        match serde_json::to_vec(&test_json) {
            Ok(x) => {
                let res = jpc(&x);
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        panic!("failed test_http err = {:?}", err);
                    }
                }
            }
            Err(err) => {
                panic!("failed test_http err = {:?}", err);
            }
        };
    }

    #[test]
    fn test_basic_http_failure() {
        register_handler("test_handler", handler_for_test);
        // path missing
        let test_json = json!({
          "id" : "dummydummy",
          "jpc" : "1.0",
          "method" : "GET",
          "params" : {
            "http" : {
            },
          },
        });
        match serde_json::to_vec(&test_json) {
            Ok(x) => {
                let res = jpc(&x);
                match res {
                    Ok(k) => {
                        let mut res_json: serde_json::Map<String, serde_json::Value> =
                            serde_json::from_slice(&k).unwrap();
                        let mut err_json: serde_json::Map<String, serde_json::Value> =
                            serde_json::from_value(res_json["error"].take()).unwrap();
                        println!("{:?}", err_json);
                        assert_eq!(err_json["op"], 2);
                        let err_json_data: serde_json::Map<String, serde_json::Value> =
                            serde_json::from_value(err_json["data"].take()).unwrap();
                        assert_eq!(err_json_data["op"], 2);
                    }
                    Err(err) => {
                        panic!("failed test_http err = {:?}", err);
                    }
                }
            }
            Err(err) => {
                panic!("failed test_http err = {:?}", err);
            }
        };
    }
}

type HandlerFunction<'a> = fn(bcc: &'a mut BitcodeContext) -> CallResult;

/// register_handler adjusts the global static call map to associate a bitcode module with a path
/// this map is used by jpc to implement bitcode calls
#[no_mangle]
pub fn register_handler(name: &str, h: HandlerFunction<'static>) {
    let hd = HandlerData { hf: h, req: None };
    match CALLMAP.lock().as_mut() {
        Ok(x) => {
            x.insert(name.to_string(), hd);
        }
        Err(e) => console_log(&format!("MutexGuard unable to aquire lock, error = {e}")),
    };
}

const ID_NOT_CALCULATED_YET: &str = "id not yet calculated";

fn do_bitcode(json_params: Request) -> CallResult {
    console_log("Parameters parsed");
    let method: &str = json_params.method.as_str();

    let mut v_leaks: Vec<Box<BitcodeContext>> = Vec::<Box<BitcodeContext>>::new();

    let cm = match CALLMAP.lock() {
        Ok(c) => c,
        Err(e) => {
            return make_json_error(
                ErrorKinds::BadHttpParams(format!("unable to gain access to callmap: error = {e}")),
                ID_NOT_CALCULATED_YET,
            )
        }
    };
    // let mut element = 1;
    // if json_params.method == "content"{
    //   element = 0;
    // }
    let mut bind = cm.get(method).into_iter();
    let cm_handler = match bind.find(|mut _x| true).as_mut() {
        Some(b) => b.to_owned(),
        None => {
            return Err(Box::new(ErrorKinds::Invalid(format!(
                "handler not found {}",
                method
            ))))
        }
    };
    match cm_handler.req {
        Some(f) => {
            let id = f.request.id.to_string();
            let bcc = Box::new(f);
            let l = Box::leak(bcc);
            unsafe {
                v_leaks.push(Box::from_raw(l));
            }
            match (cm_handler.hf)(l) {
                Ok(o) => Ok(o),
                Err(e) => make_json_error(ErrorKinds::Other(e.to_string()), &id),
            }
        }
        None => {
            let bcc = BitcodeContext {
                request: json_params.clone(),
            };
            let id = bcc.request.id.clone();
            let l = Box::leak(Box::new(bcc));
            unsafe {
                v_leaks.push(Box::from_raw(l));
            }
            match (cm_handler.hf)(l) {
                Ok(o) => Ok(o),
                Err(e) => make_json_error(ErrorKinds::Other(e.to_string()), &id),
            }
        }
    }
}

/// jpc is the main entry point into a wasm bitcode for the web assembly procedure calls
/// this function will
/// # Steps
///   * parse the input for the appropriately formatted json
///   * construct a BitcodeContext from the json
///   * attempt to call the method using the incomming path
///   * return results to the caller
#[no_mangle]
pub fn jpc(_msg: &[u8]) -> CallResult {
    console_log("In jpc");
    let input_string = str::from_utf8(_msg)?;
    console_log(&format!("parameters = {input_string}"));
    let json_params: Request = match serde_json::from_str(input_string) {
        Ok(m) => m,
        Err(err) => {
            return make_json_error(
                ErrorKinds::Invalid(format!("parse failed for http error = {err}")),
                ID_NOT_CALCULATED_YET,
            );
        }
    };

    console_log("Request parsed");
    do_bitcode(json_params)
}
