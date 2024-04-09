#![feature(try_trait_v2)]
#![feature(linked_list_cursors)]
extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use std::convert::TryInto;

use base64::{engine::general_purpose, Engine as _};
use elvwasm::{
    implement_bitcode_module, jpc, register_handler, BitcodeContext, FetchResult, ReadStreamResult,
    SystemTimeResult,
};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufWriter, ErrorKind, SeekFrom, Write};
use std::path::Path;

use elvwasm::ErrorKinds;

implement_bitcode_module!(
    "download",
    do_download,
    "bulk_download",
    do_bulk_download,
    "preview",
    do_preview,
    "thumbnail",
    do_preview
);

const VERSION: &str = "1.1.3";
const MANIFEST: &str = ".download.info";

fn compute_image_url(
    operation: &str,
    meta: &serde_json::Value,
    qp: &HashMap<String, Vec<String>>,
) -> CallResult {
    let content_type = meta
        .get("attachment_content_type")
        .ok_or(ErrorKinds::NotExist(
            "attachment_content_type not found in meta".to_string(),
        ))?
        .as_str()
        .ok_or(ErrorKinds::NotExist(
            "attachment_content_type not convertible to string".to_string(),
        ))?;

    let ct: Vec<&str> = content_type.split('/').collect();
    let url: &str;
    let mut surl: String;
    let offering: &str;
    if ct[0] == "video" {
        offering = "implied";
        let down = meta
            .get("download")
            .ok_or(ErrorKinds::NotExist(format!("download not found in meta")))?;
        let def = down.get("default").ok_or(ErrorKinds::NotExist(format!(
            "default not found in download"
        )))?;
        url = def
            .get("/")
            .ok_or(ErrorKinds::NotExist("/ not found in default".to_string()))?
            .as_str()
            .ok_or(ErrorKinds::NotExist(
                "url not convertible to string".to_string(),
            ))?;
    } else {
        let v_represenations = meta.get("representations").ok_or(ErrorKinds::NotExist(
            "representations not found in meta".to_string(),
        ))?;

        offering = v_represenations
            .get(operation)
            .ok_or(ErrorKinds::NotExist(
                "operation not found in representations".to_string(),
            ))?
            .as_str()
            .ok_or(ErrorKinds::NotExist(
                "operation not convertible to string".to_string(),
            ))?;

        let binding = meta
            .get("file")
            .ok_or(ErrorKinds::NotExist(
                "fabric_file not found in meta".to_string(),
            ))?
            .get("/")
            .ok_or(ErrorKinds::NotExist("failed to find /".to_string()))?
            .as_str()
            .ok_or(ErrorKinds::NotExist(
                "fabric_file not convertible to string".to_string(),
            ))?
            .to_string();
        let fabric_file: Vec<&str> = binding.split('/').collect();

        let file_path = &fabric_file[2..].join("/");

        let v = &vec!["-1".to_string()];
        surl = format!("/image/{offering}/files/{file_path}");
        let height_str = qp.get("height").unwrap_or(v);
        if &height_str[0] != "-1" {
            let height = height_str[0].parse::<i32>().unwrap_or(-1);
            if height > 0 {
                surl = format!("{surl}?height={height}");
            }
        }
        url = &surl;
    }

    let jret = json!({
        "url": url,
        "content_type": content_type,
        "offering": offering,
    });
    Ok(jret.to_string().as_bytes().to_vec())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SummaryElement {
    asset: String,
    status: String,
}

fn pre_processs_link(link: &str) -> String {
    let mut path_vec: Vec<&str> = link.split('/').collect();
    if path_vec.len() < 4 {
        return link.to_string();
    }

    if path_vec[3] == "meta" && path_vec[4] == "assets" {
        path_vec[3] = "bc";
        path_vec.insert(5, "download");
    }
    let path_string = path_vec.join("/");
    path_string
}

#[test]
fn test_pre_process_link() {
    let link = "/qfab/hq_someverylonghash53336444VVEDDDDDD/meta/assets/11e1e-45d4a-06e3-6efc76.jpg";
    assert_eq!(
        pre_processs_link(link),
        "/qfab/hq_someverylonghash53336444VVEDDDDDD/bc/assets/download/11e1e-45d4a-06e3-6efc76.jpg"
    );
    let link = "/qfab/hq_someverylonghash53336444VVEDDDDDD/bc/assets/download/files/assets/11e1e-45d4a-06e3-6efc76.jpg";
    assert_eq!(pre_processs_link(link), link);

    let link = "/qfab/hq_someverylonghash53336444VVEDDDDDD/meta/assets/11e1e-45d4a-06e3-6efc76.jpg";
    assert_eq!(
        pre_processs_link(link),
        "/qfab/hq_someverylonghash53336444VVEDDDDDD/bc/assets/download/11e1e-45d4a-06e3-6efc76.jpg"
    );
}

fn process_multi_entry(bcc: &BitcodeContext, link: &str) -> CallResult {
    let path_string = pre_processs_link(link);
    bcc.log_debug(&std::format!(
        "process_multi_entry path_string = {path_string}"
    ))?;
    bcc.fetch_link(json!(path_string))
}

#[no_mangle]
fn do_bulk_download(bcc: &mut BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let path_vec: Vec<&str> = bcc.request.params.http.path.split('/').collect();
    bcc.log_debug(&format!(
        "In Assets path_vec = {path_vec:?} http params = {http_p:?}"
    ))?;

    bcc.log_debug("do_bulk_download")?;
    const DEF_CAP: usize = 50000000;
    let buf_cap = match qp.get("buffer_capacity") {
        Some(x) => {
            bcc.log_debug(&format!("new capacity of {x:?} set"))?;
            x[0].parse().unwrap_or(DEF_CAP)
        }
        None => DEF_CAP,
    };
    let total_size = 0;
    let mut fw = FabricWriter::new(bcc, total_size);
    {
        let bw = BufWriter::with_capacity(buf_cap, &mut fw);

        //let zip = GzEncoder::new(bw, flate2::Compression::default());
        let mut a = tar::Builder::new(bw);
        let time_cur: SystemTimeResult = bcc.q_system_time().try_into()?;
        let rsr: ReadStreamResult = bcc.read_stream("fis".to_string(), 0).try_into()?;

        let params: Vec<String> = if !rsr.result.is_empty() {
            let b64_decoded = general_purpose::STANDARD.decode(&rsr.result)?;

            let p: serde_json::Value = serde_json::from_slice(&b64_decoded)?;
            p.as_array()
                .ok_or(ErrorKinds::Invalid("params not an array".to_string()))?
                .iter()
                .map(|value| value.as_str().unwrap_or_default().to_string())
                .collect()
        } else {
            bcc.request
                .params
                .http
                .body
                .as_array()
                .map(|array| {
                    array
                        .iter()
                        .map(|value| value.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        };
        bcc.log_debug(&format!("Bulk download params: {params:?}"))?;
        let mut v_file_status: Vec<SummaryElement> = vec![];

        for p in &params {
            let exr: FetchResult = match process_multi_entry(bcc, p) {
                Ok(exr) => exr.try_into()?,
                Err(e) => {
                    v_file_status.push(SummaryElement {
                        asset: format!("{0} Error={e}", p),
                        status: "failed".to_string(),
                    });
                    bcc.log_error(&format!("Error processing {p} : {e}"))?;
                    continue;
                }
            };

            let mut header = tar::Header::new_gnu();
            let b64_decoded = general_purpose::STANDARD.decode(&exr.body)?;
            header.set_size(b64_decoded.len() as u64);
            header.set_cksum();
            header.set_mtime(time_cur.time);
            header.set_mode(0o644);
            let filename: String = exr
                .headers
                .get("Content-Disposition")
                .ok_or(ErrorKinds::NotExist(
                    "Content-Disposition not found".to_string(),
                ))?
                .iter()
                .find(|s| s.contains("filename="))
                .and_then(|s| s.split("filename=").nth(1))
                .map(|s| s.trim_matches(|c| c == '"' || c == '\''))
                .ok_or(ErrorKinds::NotExist("filename= not found".to_string()))?
                .to_string();
            a.append_data(&mut header, &filename, b64_decoded.as_slice())?;
            v_file_status.push(SummaryElement {
                asset: filename.to_string(),
                status: "success".to_string(),
            });
        }
        let mut header = tar::Header::new_gnu();
        let contents = v_file_status
            .iter()
            .map(|x| format!("{0} {1}", x.asset, x.status))
            .collect::<Vec<String>>()
            .join("\n");
        header.set_size(contents.len() as u64);
        header.set_cksum();
        header.set_mtime(time_cur.time);
        header.set_mode(0o644);
        a.append_data(&mut header, MANIFEST, std::io::Cursor::new(contents))?;
        a.finish()?;
        let mut finished_writer = a.into_inner()?;
        finished_writer.flush()?;
    }
    bcc.log_debug(&format!("Callback size = {}", fw.size))?;
    let disp = format!("attachment; filename=\"{}\"", "download.tar");
    bcc.callback_disposition(200, "application/tar", fw.size, &disp, VERSION)?;
    bcc.make_success_json(&json!({}))
}

fn do_single_asset(
    bcc: &BitcodeContext,
    qp: &HashMap<String, Vec<String>>,
    operation: &str,
    asset: &str,
    is_download: bool,
) -> CallResult {
    bcc.log_debug("do_single_asset")?;
    let asset_path = Path::new("/assets")
        .join(Path::new(asset).strip_prefix("/").unwrap())
        .to_string_lossy()
        .into_owned();
    let meta: serde_json::Value = serde_json::from_slice(&bcc.sqmd_get_json(&asset_path)?)?;
    let result: ComputeCallResult = compute_image_url(operation, &meta, qp).try_into()?;
    let is_video = result.offering == "implied";

    let exr: FetchResult = get_single_offering_image(bcc, &result.url, is_video).try_into()?;
    let imgbits = &general_purpose::STANDARD.decode(&exr.body)?;
    bcc.log_debug(&format!(
        "imgbits decoded size = {} fout size = {}",
        imgbits.len(),
        exr.body.len()
    ))?;
    let mut filename = meta
        .get("title")
        .ok_or(ErrorKinds::NotExist("title not found in meta".to_string()))?
        .as_str()
        .ok_or(ErrorKinds::Invalid(
            "title not convertible to string".to_string(),
        ))?
        .to_string();
    let ct = meta
        .get("attachment_content_type")
        .ok_or(ErrorKinds::NotExist(
            "attachment_content_type not found in meta".to_string(),
        ))?
        .as_str()
        .ok_or(ErrorKinds::NotExist(
            "attachment_content_type not convertible to string".to_string(),
        ))?
        .to_string();
    let is_document = ct == "application/pdf";
    let content_returned = exr
        .headers
        .get("Content-Type")
        .ok_or(ErrorKinds::NotExist("Content-Type not found".to_string()))?;
    if ct != "image/jpeg" && !is_document && content_returned[0] == "image/jpeg" {
        filename += ".jpg"
    }
    bcc.log_debug(&format!(
        "RepAssets op={operation} asset={asset} isDoc={is_document} ct={ct} filename={filename}, rep image path={0} version={VERSION}, rep_image format={1}",result.url, &content_returned[0]
    ))?;
    if is_download {
        let content_disp = format!("attachment; filename=\"{}\"", filename);
        bcc.callback_disposition(
            200,
            &content_returned[0],
            imgbits.len(),
            &content_disp,
            VERSION,
        )?;
    } else if is_document {
        bcc.callback(200, &ct, imgbits.len())?;
    } else {
        bcc.callback(200, &content_returned[0], imgbits.len())?;
    }

    bcc.write_stream("fos", imgbits)?;
    bcc.make_success_json(&json!({}))
}

fn get_single_offering_image(bcc: &BitcodeContext, url: &str, is_video: bool) -> CallResult {
    if is_video {
        return bcc.fetch_link(json!(url));
    }
    bcc.fetch_link(json!(format!("./rep{url}")))
}

#[no_mangle]
fn do_download(bcc: &mut BitcodeContext) -> CallResult {
    let req = &bcc.request;
    do_single_asset(
        bcc,
        &req.params.http.query,
        &req.method,
        &req.params.http.path,
        true,
    )
}

#[no_mangle]
fn do_preview(bcc: &mut BitcodeContext) -> CallResult {
    let req = &bcc.request;
    do_single_asset(
        bcc,
        &req.params.http.query,
        &req.method,
        &req.params.http.path,
        false,
    )
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ComputeCallResult {
    pub url: String,
    pub content_type: String,
    pub offering: String,
}

impl TryFrom<CallResult> for ComputeCallResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<ComputeCallResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

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
            SeekFrom::Start(_s) => {}
            SeekFrom::Current(_s) => {}
            SeekFrom::End(_s) => {}
        }
        Ok(self.size as u64)
    }
}

#[test]
fn test_image_url_generation() {
    let meta = json!({
      "asset_type": "image",
      "attachment_content_type": "image/jpeg",
      "attachment_file_name": "AquarionEVOL_ShowHeroTablet.jpg",
      "attachment_file_size": 1285603,
      "attachment_updated_at": "2024-02-29T02:54:03.510Z",
      "file": {
        ".": {
          "auto_update": {
            "tag": "latest"
          },
          "container": "hq__J6aApMvBZPCkikMMMh9yZ69ymkr6c2DcmFj39Wk5GcQXJTd5z4VG7RgbiKHehPvHc3Q92naTQg"
        },
        "/": "./files/assets/11e1e-45d4a-06e3-6efc76.jpg"
      },
      "image": {
        "height": 2394,
        "orientation": "portrait",
        "width": 1728
      },
      "original_access": 400,
      "representations": {
        "download": "tier1",
        "preview": "none",
        "thumbnail": "tier1"

      },
      "title": "AquarionEVOL_ShowHeroTablet.jpg",
      "uuid": "11e1e-45d4a-06e3-6efc76",
      "version": "2",
    });
    let operation = "download";
    let result: ComputeCallResult =
        compute_image_url(operation, &meta, &HashMap::<String, Vec<String>>::default())
            .try_into()
            .unwrap();
    assert_eq!(
        result.url,
        "/image/tier1/files/assets/11e1e-45d4a-06e3-6efc76.jpg"
    );
    let operation = "preview";
    let result: ComputeCallResult =
        compute_image_url(operation, &meta, &HashMap::<String, Vec<String>>::default())
            .try_into()
            .unwrap();
    assert_eq!(
        result.url,
        "/image/none/files/assets/11e1e-45d4a-06e3-6efc76.jpg"
    );
    let operation = "thumbnail";
    let result: ComputeCallResult =
        compute_image_url(operation, &meta, &HashMap::<String, Vec<String>>::default())
            .try_into()
            .unwrap();
    assert_eq!(
        result.url,
        "/image/tier1/files/assets/11e1e-45d4a-06e3-6efc76.jpg"
    );
    let mut qp = HashMap::<String, Vec<String>>::default();
    qp.insert("height".to_string(), vec!["100".to_string()]);

    let result_with_height: ComputeCallResult =
        compute_image_url(operation, &meta, &qp).try_into().unwrap();
    assert_eq!(
        result_with_height.url,
        "/image/tier1/files/assets/11e1e-45d4a-06e3-6efc76.jpg?height=100"
    );

    assert_eq!(result.content_type, "image/jpeg")
}
