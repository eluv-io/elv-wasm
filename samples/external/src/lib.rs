extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;

use base64::{engine::general_purpose, Engine as _};
use elvwasm::bccontext_fabric_io::FabricStreamWriter;
use elvwasm::BitcodeContext;
use elvwasm::{
    implement_bitcode_module, jpc, register_handler, CreatePartResult, ErrorKinds,
    ExternalCallResult, FinalizeCallResult, NewStreamResult,
};
use serde_json::json;
use std::convert::TryInto;
use std::io::Write;

implement_bitcode_module!("external", do_external, "failme", do_external_fail);

#[no_mangle]
fn do_external_fail(bcc: &mut BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let img_hash = &qp
        .get("img_hash")
        .ok_or(ErrorKinds::Invalid("img_hash not present".to_string()))?[0];
    let img_obj = &qp
        .get("img_obj")
        .ok_or(ErrorKinds::Invalid("img_hash not present".to_string()))?[0];
    let tar_hash = &qp
        .get("tar_hash")
        .ok_or(ErrorKinds::Invalid("tar_hash not present".to_string()))?[0];
    bcc.log_debug(&format!("img_hash ={img_hash:?} tar_hash = {tar_hash:?}"))?;
    let params = json!({
        "http" : {
            "verb" : "some",
            "headers": {
                "Content-type": [
                    "application/json"
                ]
            },
            "path" : "/image/default/assets/birds.jpg",
            "query" : {
                "height" : ["200"],
            },
        },
    });
    let exr: ExternalCallResult = bcc
        .call_external_bitcode("image", &params, img_obj, img_hash)
        .try_into()?;
    let imgbits = &general_purpose::STANDARD.decode(&exr.fout)?;
    bcc.log_debug(&format!(
        "imgbits decoded size = {} fout size = {}",
        imgbits.len(),
        exr.fout.len()
    ))?;
    bcc.log_debug(&format!("fout {}", &exr.fout))?;
    let stream_img: NewStreamResult = bcc.new_stream().try_into()?;
    defer! {
        bcc.log_debug(&format!("Closing part stream {}", &stream_img.stream_id)).unwrap_or_default();
        let _ = bcc.close_stream(stream_img.stream_id.clone());
    }
    let mut fsw = FabricStreamWriter::new(bcc, stream_img.stream_id.clone(), imgbits.len());
    fsw.write_all(imgbits)?;
    let imgpart: CreatePartResult = bcc
        .q_create_part_from_stream(&bcc.request.q_info.write_token, &stream_img.stream_id)
        .try_into()?;
    bcc.log_debug(&format!(
        "imgpart hash {} size = {}",
        &imgpart.qphash, imgpart.size
    ))?;
    let fc: FinalizeCallResult = bcc
        .q_finalize_content(&bcc.request.q_info.write_token)
        .try_into()?;
    let tar_params = json!({
        "http" : {
            "verb" : "some",
            "headers": {
                "Content-type": [
                    "application/json"
                ]
            },
            "path" : "/tar",
            "query" : {
                "object_id_or_hash" : [fc.qhash],
            },
        },
    });
    let exr_tar: ExternalCallResult = bcc
        .call_external_bitcode("tar", &tar_params, &fc.qhash, tar_hash)
        .try_into()?;
    let _tarbits = &general_purpose::STANDARD.decode(exr_tar.fout)?;
    let _img = bcc.call_external_bitcode("image", &params, img_obj, img_hash)?;
    bcc.make_success_json(&json!(
    {
        "headers" : "application/json",
        "body" : "SUCCESS",
        "result" : "complete",
    }))
}

#[no_mangle]
fn do_external(bcc: &mut BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let img_hash = &qp
        .get("img_hash")
        .ok_or(ErrorKinds::Invalid("img_hash not present".to_string()))?[0];
    let img_obj = &qp
        .get("img_obj")
        .ok_or(ErrorKinds::Invalid("img_obj not present".to_string()))?[0];
    let tar_hash = &qp
        .get("tar_hash")
        .ok_or(ErrorKinds::Invalid("tar_hash not present".to_string()))?[0];
    bcc.log_debug(&format!("img_hash ={img_hash:?} tar_hash = {tar_hash:?}"))?;
    let params = json!({
        "http" : {
            "verb" : "GET",
            "headers": {
                "Content-type": [
                    "application/json"
                ]
            },
            "path" : "/image/default/files/assets/birds.jpg",
            "query" : {
                "height" : ["200"],
            },
        },
    });
    let exr: ExternalCallResult = bcc
        .call_external_bitcode("image", &params, img_obj, "builtin")
        .try_into()?;
    bcc.log_debug("do external")?;
    let imgbits = &general_purpose::STANDARD.decode(&exr.fout)?;
    console_log(&format!(
        "imgbits decoded size = {} fout size = {}",
        imgbits.len(),
        exr.fout.len()
    ));
    bcc.log_debug(&format!("fout {}", &exr.fout))?;
    let stream_img: NewStreamResult = bcc.new_stream().try_into()?;
    defer! {
        bcc.log_debug(&format!("Closing part stream {}", &stream_img.stream_id)).unwrap_or_default();
        let _ = bcc.close_stream(stream_img.stream_id.clone());
    }
    bcc.write_stream(&stream_img.stream_id, imgbits)?;
    let imgpart: CreatePartResult = bcc
        .q_create_part_from_stream(&bcc.request.q_info.write_token, &stream_img.stream_id)
        .try_into()?;
    bcc.log_debug(&format!(
        "imgpart hash {} size = {}",
        &imgpart.qphash, imgpart.size
    ))?;
    let fc: FinalizeCallResult = bcc
        .q_finalize_content(&bcc.request.q_info.write_token)
        .try_into()?;
    let tar_params = json!({
        "http" : {
            "verb" : "some",
            "headers": {
                "Content-type": [
                    "application/json"
                ]
            },
            "path" : "/tar",
            "query" : {
                "object_id_or_hash" : [fc.qhash],
            },
        },
    });
    let exr_tar: ExternalCallResult = bcc
        .call_external_bitcode("tar", &tar_params, &fc.qhash, tar_hash)
        .try_into()?;
    let tarbits = &general_purpose::STANDARD.decode(&exr_tar.fout)?;
    bcc.log_debug(&format!(
        "fout size = {} tar_ bit len = {}",
        &exr_tar.fout.len(),
        tarbits.len()
    ))?;
    bcc.callback(200, "application/zip", tarbits.len())?;
    FabricStreamWriter::new(bcc, "fos".to_string(), tarbits.len()).write_all(tarbits)?;
    bcc.make_success_json(&json!({}))
}
