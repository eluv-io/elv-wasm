extern crate elvwasm;
extern crate serde_json;
extern crate serde;

use elvwasm::{implement_bitcode_module, jpc, register_handler, LROResult};
use serde_json::{json};

implement_bitcode_module!("lro", do_lro, "callback", do_lro_callback);


#[no_mangle]
fn do_lro(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let _qp = &http_p.query;
    let id = &bcc.request.id;
    let bhandle = bcc.start_bitcode_lro("callback", &json!({"arg1" : "test"}))?;
    let bhandle:LROResult = serde_json::from_slice(&bhandle)?;
    bcc.make_success_json(&json!(
        {
            "headers" : "application/json",
            "body" : bhandle,
            "result" : "complete",
        }), id)
}

#[no_mangle]
fn do_lro_callback(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let _qp = &http_p.query;
    bcc.log_info("IN CALLBACK!!!!!!!")?;
    let id = &bcc.request.id;
    bcc.make_success_json(&json!(
        {
            "headers" : "application/json",
            "body" : "SUCCESS",
            "result" : "complete",
        }), id)
}