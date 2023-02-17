extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate thiserror;
extern crate wapc_guest as guest;

use crate::{BitcodeContext};

use serde_json::{json, Value};
use std::str;
use guest::CallResult;

#[macro_export]
macro_rules! implement_ext_func {
    (
      $(#[$meta:meta])*
      $handler_name:ident,
      $fabric_name:literal,
      $module_name:literal
    ) => {
      $(#[$meta])*
      pub fn $handler_name(&'a self, v:Option<serde_json::Value>) -> CallResult {
        let v_call = match v{
          Some(v) => v,
          None => Value::default(),
        };
        let method = $fabric_name;
        let impl_result = self.call_function(method, v_call, $module_name)?;
        let id = self.request.id.clone();
        self.make_success_bytes(&impl_result, &id)
      }
    }
  }

impl<'a> BitcodeContext{
    implement_ext_func!(
        /// proxy_http proxies an http request in case of CORS issues
        /// # Arguments
        /// * `v` : a JSON Value
        ///
        /// ```
        /// use serde_json::json;
        ///
        /// fn do_something<'s>(bcc: &'s elvwasm::BitcodeContext) -> wapc_guest::CallResult {
        ///   let v = json!({
        ///         "request_parameters" : {
        ///         "url": "https://www.googleapis.com/customsearch/v1?key=AIzaSyCppaD53DdPEetzJugaHc2wW57hG0Y5YWE&q=fabric&cx=012842113009817296384:qjezbmwk0cx",
        ///         "method": "GET",
        ///         "headers": {
        ///           "Accept": "application/json",
        ///           "Content-Type": "application/json"
        ///         }
        ///      }
        ///   });
        ///   bcc.proxy_http(Some(v))
        /// }
        /// ```
        ///
        /// # Returns
        /// * slice of [u8]
        ///
        /// [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/rproxy/src/lib.rs#L26)
        ///
        proxy_http,
        "ProxyHttp",
        "ext"
    );

    /// start_bitcode_lro initiates a long running operation on the fabric.  Currently the lro implementation
    /// constrains the callback to be in the same bitcode module as the initiator.
    /// # Arguments
    /// * `function` : &str the function to call in the current bitcode module
    /// * `args` : JSON value containing the arguments to pass the callback
    ///
    /// # Returns
    /// * slice of [u8]
    ///
    ///  [Example](https://github.com/eluv-io/elv-wasm/blob/d261ece2140e5fc498edc470c6495065d1643b14/samples/lro/src/lib.rs#L16)
    ///
    pub fn start_bitcode_lro(&'a self, function: &str, args: &serde_json::Value) -> CallResult {
        let params = json!({ "function": function,  "args" : args});
        self.call_function("StartBitcodeLRO", params, "lro")
    }

}
