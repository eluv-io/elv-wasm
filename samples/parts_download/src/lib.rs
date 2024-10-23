#![feature(try_trait_v2)]
#![feature(linked_list_cursors)]
extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;
const VERSION: &str = "1.1.3";

use std::collections::HashMap;

use elvwasm::{
    implement_bitcode_module, jpc, register_handler, BitcodeContext, NewStreamResult, QPartList,
    SystemTimeResult,
};
use serde_json::json;
use std::io::{BufWriter, ErrorKind, SeekFrom, Write};

implement_bitcode_module!(
    "parts_download",
    do_parts_download,
    "content",
    do_parts_download
);

#[derive(Debug)]
struct FabricWriter<'a> {
    bcc: &'a BitcodeContext,
    size: usize,
}

impl<'a> FabricWriter<'a> {
    fn new(bcc: &'a BitcodeContext, sz: usize) -> FabricWriter<'a> {
        FabricWriter { bcc, size: sz }
    }
}
impl<'a> std::io::Write for FabricWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self.bcc.write_stream("fos", buf) {
            Ok(s) => {
                self.bcc
                    .log_debug(&format!("Wrote {} bytes", buf.len()))
                    .unwrap_or_default(); // to gobble the log result
                let w: elvwasm::WritePartResult = serde_json::from_slice(&s)?;
                self.size += w.written;
                Ok(w.written)
            }
            Err(e) => Err(std::io::Error::new(ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        // Nothing to flush.  The BufWriter will handle its buffer independant using writes
        Ok(())
    }
}

impl<'a> std::io::Seek for FabricWriter<'a> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        match pos {
            SeekFrom::Start(s) => {
                self.bcc
                    .log_debug(&format!("SEEK from START {s}"))
                    .unwrap_or_default();
            }
            SeekFrom::Current(s) => {
                self.bcc
                    .log_debug(&format!("SEEK from CURRENT {s}"))
                    .unwrap_or_default();
            }
            SeekFrom::End(s) => {
                self.bcc
                    .log_debug(&format!("SEEK from END {s}"))
                    .unwrap_or_default();
            }
        }
        Ok(self.size as u64)
    }
}

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
        let data = bcc.read_stream(stream_wm.stream_id.clone(), usz)?;
        bcc.write_stream("fos", &data)?;
        bcc.callback_disposition(200, "application/octet-stream", usz, &content_disp, VERSION)?;
        return bcc.make_success_json(&json!({}));
    }
    let mut fw = FabricWriter::new(bcc, total_size.try_into()?);
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
            let data = bcc.read_stream(stream_wm.stream_id.clone(), usz)?;
            let mut header = tar::Header::new_gnu();
            header.set_size(usz as u64);
            header.set_cksum();
            header.set_mtime(time_cur.time);
            a.append_data(&mut header, part.hash.clone(), data.as_slice())?;
        }
        a.finish()?;
        let mut finished_writer = a.into_inner()?;
        finished_writer.flush()?;
    }
    bcc.log_debug(&format!("Callback size = {}", fw.size))?;
    bcc.callback_disposition(200, "application/x-tar", fw.size, &content_disp, VERSION)?;
    bcc.make_success_json(&json!({}))
}
