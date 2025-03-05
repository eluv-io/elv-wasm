#![feature(try_trait_v2)]
#![feature(linked_list_cursors)]
extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;
const VERSION: &str = "1.1.3.1";

use std::collections::HashMap;

use elvwasm::{
    bccontext_fabric_io::{FabricStreamReader, FabricStreamWriter},
    implement_bitcode_module, jpc, register_handler, NewStreamResult, QPartList, SystemTimeResult,
};
use serde_json::json;
use std::io::{BufWriter, Write};

implement_bitcode_module!(
    "parts_download",
    do_parts_download,
    "content",
    do_parts_download
);

fn get_set_content_disposition(
    headers: HashMap<String, Vec<String>>,
    query: &HashMap<String, Vec<String>>,
    default: &str,
) -> CallResult {
    let mut res = Vec::new();

    // Extract from headers
    if let Some(values) = headers.get("X-Content-Fabric-Set-Content-Disposition") {
        for value in values {
            res.push(value.clone());
        }
    }

    // Extract from query parameters
    if let Some(value) = query.get("header-x_set_content_disposition") {
        res.push(value[0].clone());
    }

    // Handle the results
    if res.is_empty() {
        return Ok(default.as_bytes().to_vec());
    }
    if res.len() > 1 {
        let first = &res[0];
        for s in &res[1..] {
            if s != first {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "GetSetContentDisposition: multiple values (inconsistent): {}",
                    res.join(",")
                )));
            }
        }
    }

    Ok(res[0].clone().as_bytes().to_vec())
}

#[no_mangle]
fn do_parts_download(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let empty_vec = Vec::<String>::new();
    let part_hash = match qp.get("part_hash") {
        Some(x) => x,
        None => &empty_vec,
    };
    let bdisp = get_set_content_disposition(http_p.headers.clone(), qp, "")?;
    let content_disp = match String::from_utf8(bdisp) {
        Ok(v) => v,
        Err(e) => {
            bcc.log_debug(&std::format!(
                "Error converting content disposition to String: {}",
                e
            ))?;
            return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Error converting content disposition to String: {}",
                e
            )));
        }
    };
    const DEF_CAP: usize = 50000000;
    let buf_cap = match qp.get("buffer_capacity") {
        Some(x) => {
            bcc.log_debug(&format!("new capacity of {x:?} set"))?;
            x[0].parse().unwrap_or(DEF_CAP)
        }
        None => DEF_CAP,
    };
    let mut total_size = 0;
    if !part_hash.is_empty() {
        let part = part_hash[0].clone();
        let stream_wm: NewStreamResult = bcc.new_stream().try_into()?;
        defer! {
            bcc.log_debug(&format!("Closing part stream {}", &stream_wm.stream_id)).unwrap_or_default();
            let _ = bcc.close_stream(stream_wm.stream_id.clone());
        }
        let _wprb = bcc.write_part_to_stream(
            stream_wm.stream_id.clone(),
            part.clone(),
            bcc.request.q_info.hash.clone(),
            0,
            -1,
            true,
        )?;
        let pl: QPartList = bcc
            .q_part_list(bcc.request.q_info.hash.to_string())
            .try_into()?;
        pl.part_list.parts.iter().for_each(|x| {
            if x.hash == part {
                total_size = x.size;
            }
        });
        let usz = total_size.try_into()?;
        let mut fsr = FabricStreamReader::new(stream_wm.stream_id.clone(), bcc);
        let mut fsw = FabricStreamWriter::new(bcc, "fos".to_string(), usz);
        std::io::copy(&mut fsr, &mut fsw)?;
        bcc.callback_disposition(200, "application/octet-stream", usz, &content_disp, VERSION)?;
        return bcc.make_success_json(&json!({}));
    }
    let mut fw = FabricStreamWriter::new(bcc, "fos".to_string(), total_size.try_into()?);
    {
        let bw = BufWriter::with_capacity(buf_cap, &mut fw);

        let pl: QPartList = bcc
            .q_part_list(bcc.request.q_info.hash.clone())
            .try_into()?;

        let mut a = tar::Builder::new(bw);
        let time_cur: SystemTimeResult = bcc.q_system_time().try_into()?;
        for part in pl.part_list.parts {
            let stream_wm: NewStreamResult = bcc.new_stream().try_into()?;
            defer! {
                bcc.log_debug(&format!("Closing part stream {}", &stream_wm.stream_id)).unwrap_or_default();
                let _ = bcc.close_stream(stream_wm.stream_id.clone());
            }
            let _wprb = bcc.write_part_to_stream(
                stream_wm.stream_id.clone(),
                part.hash.clone(),
                bcc.request.q_info.hash.clone(),
                0,
                -1,
                true,
            )?;
            let usz = part.size.try_into()?;
            let fsr = FabricStreamReader::new(stream_wm.stream_id.clone(), bcc);
            let mut header = tar::Header::new_gnu();
            header.set_size(usz);
            header.set_mode(0o644);
            header.set_mtime(time_cur.time);
            header.set_path(&part.hash)?;
            header.set_cksum();

            a.append(&header, fsr)?;
        }
        a.finish()?;
        let mut finished_writer = a.into_inner()?;
        finished_writer.flush()?;
    }
    bcc.log_debug(&format!("Callback size = {}", fw.size))?;
    bcc.callback_disposition(200, "application/x-tar", fw.size, &content_disp, VERSION)?;
    bcc.make_success_json(&json!({}))
}
