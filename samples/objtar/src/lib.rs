#![feature(try_trait_v2)]
#![feature(linked_list_cursors)]
extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;

use elvwasm::{
    bccontext_fabric_io::{FabricStreamReader, FabricStreamWriter},
    implement_bitcode_module, jpc, register_handler, FileStream, NewStreamResult, QPartList,
    SystemTimeResult,
};
use flate2::write::GzEncoder;
use serde_json::json;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};

implement_bitcode_module!(
    "tar",
    do_tar_from_obj,
    "content",
    do_tar_from_obj,
    "seeker",
    do_seeker
);

#[no_mangle]
fn do_tar_from_obj(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let vqhot = &vec![bcc.request.q_info.qhot()];
    let obj_id = match qp.get("object_id_or_hash") {
        Some(x) => x,
        None => vqhot,
    };
    const DEF_CAP: usize = 50000000;
    let buf_cap = match qp.get("buffer_capacity") {
        Some(x) => {
            bcc.log_debug(&format!("new capacity of {x:?} set"))?;
            x[0].parse().unwrap_or(DEF_CAP)
        }
        None => DEF_CAP,
    };
    let total_size = 0;
    let mut fw = FabricStreamWriter::new(bcc, "fos".to_string(), total_size);
    {
        let bw = BufWriter::with_capacity(buf_cap, &mut fw);

        let pl: QPartList = bcc.q_part_list(obj_id[0].to_string()).try_into()?;

        let zip = GzEncoder::new(bw, flate2::Compression::default());
        let mut a = tar::Builder::new(zip);
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
                false,
            )?;
            let usz = part.size as u64;
            let fsr = FabricStreamReader::new(stream_wm.stream_id.clone(), bcc);
            let mut header = tar::Header::new_gnu();
            header.set_size(usz);
            header.set_cksum();
            header.set_mtime(time_cur.time);
            a.append_data(&mut header, part.hash.clone(), fsr)?;
        }
        a.finish()?;
        let mut finished_writer = a.into_inner()?;
        finished_writer.flush()?;
    }
    bcc.log_debug(&format!("Callback size = {}", fw.size))?;
    bcc.callback(200, "application/zip", fw.size)?;

    bcc.make_success_json(&json!({}))
}

#[no_mangle]
pub fn do_seeker(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let fstream: FileStream = bcc.new_file_stream().try_into()?;
    defer! {
        bcc.log_debug(&format!("Closing part stream {}", &fstream.stream_id)).unwrap_or_default();
        let _ = bcc.close_stream(fstream.stream_id.clone());
    }
    let mut fsw = FabricStreamWriter::new(bcc, fstream.stream_id.clone(), 0);

    // Write a known string into the fabric file
    let known_string = "Hello, world! This is a test string for seeking.";
    fsw.write_all(known_string.as_bytes())?;
    fsw.flush()?;
    let expected = ["Hello, wor", "world! Thi", " This is a", "est string"];

    // Use a reader to seek to different positions and read the data
    let mut fsr = FabricStreamReader::new(fstream.stream_id.clone(), bcc);
    let positions = [0, 7, 13, 25];
    for (iter, &pos) in positions.iter().enumerate() {
        let seek_return = fsr.seek(SeekFrom::Start(pos))?;
        if seek_return != pos {
            return bcc.make_error(&format!(
                "error unexpected seek return at position {} expected {} got {}",
                iter, pos, seek_return
            ));
        }
        let mut buffer = [0; 10];
        let bytes_read = fsr.read(&mut buffer)?;
        if &buffer[..bytes_read] != expected[iter].as_bytes() {
            return bcc.make_error(&format!(
                "error unexpected data read at position {} expected {} got {}",
                iter,
                expected[iter],
                String::from_utf8_lossy(&buffer[..bytes_read]),
            ));
        }
    }

    // Close the stream
    bcc.close_stream(fstream.stream_id.clone())?;
    bcc.make_success_json(&json!({}))
}
