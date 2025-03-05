#![feature(try_trait_v2)]
#![feature(linked_list_cursors)]
extern crate base64;
extern crate elvwasm;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use elvwasm::{
    bccontext_fabric_io::FabricStreamReader, bccontext_fabric_io::FabricStreamWriter,
    implement_bitcode_module, jpc, register_handler, BitcodeContext, FetchResult, SystemTimeResult,
};

use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufWriter, Write};
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

// This function is used to compute the image url based on the operation and meta data
// The content type is aquired from meta and is used to determine if the file is a video or image
// if video the url is returned as is from meta download default
// if image the url is constructed from the representations and file path
//      The operation is used to determine the representation to use
//      The meta data is used to determine the file path
//      The query parameters are used to determine the height of the image
// The function returns a json string with the url, content_type, and offering
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
        let down = meta.get("download").ok_or(ErrorKinds::NotExist(
            "download not found in meta".to_string(),
        ))?;
        let def = down.get("default").ok_or(ErrorKinds::NotExist(
            "default not found in download".to_string(),
        ))?;
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

// This function is used to pre process the link
// The function takes a link and returns a string
// The function splits the link by '/' and checks if the 4th and 5th elements are meta and assets
// If they are the function replaces the 3rd element with bc and inserts download after assets
// The function then joins the elements back together with '/' and returns the string
fn pre_processs_link(link: &str) -> String {
    let mut path_vec: Vec<&str> = link.split('/').collect();
    if path_vec.len() < 4 {
        return link.to_string();
    }

    if path_vec[3] == "meta" && path_vec[4] == "assets" {
        path_vec[3] = "bc";
        path_vec.insert(5, "download");
    }
    path_vec.join("/")
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
    bcc.fetch_link_reader(json!(path_string))
}

// This function is used to download multiple assets
// The function takes a list of assets from a POST body and downloads them
// The body is of the form ["/qfab/hash/meta/assets/asset1", "/qfab/hash/meta/assets/asset2]
// This function is used to download multiple assets at once
// The function creates a tar file with the assets and a manifest file
#[no_mangle]
fn do_bulk_download(bcc: &mut BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    let path_vec: Vec<&str> = bcc.request.params.http.path.split('/').collect();
    bcc.log_debug(&format!(
        "In Assets path_vec = {path_vec:?} http params = {http_p:?}"
    ))?;

    bcc.log_debug("do_bulk_download")?;

    // set the headers BEFORE writing any data - otherwise they will be ignored.
    let disp = format!("attachment; filename=\"{}\"", "download.tar");
    bcc.callback_disposition(200, "application/tar", 0, &disp, VERSION)?;

    const DEF_CAP: usize = 50000000;
    let buf_cap = match qp.get("buffer_capacity") {
        Some(x) => {
            bcc.log_debug(&format!("new capacity of {x:?} set"))?;
            x[0].parse().unwrap_or(DEF_CAP)
        }
        None => DEF_CAP,
    };
    let mut fw = FabricStreamWriter::new(bcc, "fos".to_string(), 0);
    {
        let bw = BufWriter::with_capacity(buf_cap, &mut fw);

        //let zip = GzEncoder::new(bw, flate2::Compression::default());
        let mut a = tar::Builder::new(bw);
        let time_cur: SystemTimeResult = bcc.q_system_time().try_into()?;
        let mut fir = FabricStreamReader::new("fis".to_string(), bcc);
        let mut buffer = Vec::new();
        std::io::copy(&mut fir, &mut buffer)?;
        let rsr: Vec<u8> = buffer;

        let params: Vec<String> = if !rsr.is_empty() {
            let p: serde_json::Value = serde_json::from_slice(&rsr)?;
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
            let sz_file: i32 = exr
                .headers
                .get("Content-Length")
                .unwrap_or(&vec!["0".to_string()])[0]
                .parse::<i32>()
                .unwrap_or(0);
            bcc.log_debug(&std::format!("Bulk download asset {p} size = {sz_file}"))?;
            if sz_file < 0 {
                v_file_status.push(SummaryElement {
                    asset: format!("{0} Error=Size is negative", p),
                    status: "failed".to_string(),
                });
                continue;
            }

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
            let fsr = FabricStreamReader::new(exr.body.clone(), bcc);
            header.set_size(sz_file as u64);
            header.set_mtime(time_cur.time);
            header.set_mode(0o644);
            header.set_path(&filename)?;
            header.set_cksum();
            a.append(&header, fsr)?;
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
    bcc.make_success_json(&json!({}))
}

// This function is used to download or preview/thu8mbnail a single asset
// The function gets the meta data for the asset and the content type via compute_image_url
// The function then fetches the image bits from the url calling get_single_offering_image
// which in actuality is a fetch_link
// The function then decodes the image or video bits and writes them to the stream
// The function then returns a success json
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

    let sid = exr.body;
    let mut fsr = FabricStreamReader::new(sid.clone(), bcc);
    let mut fsw = FabricStreamWriter::new(bcc, "fos".to_string(), 0);
    let body_size = std::io::copy(&mut fsr, &mut fsw)? as usize;
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
        bcc.callback_disposition(200, &content_returned[0], body_size, &content_disp, VERSION)?;
    } else if is_document {
        bcc.callback(200, &ct, body_size)?;
    } else {
        bcc.callback(200, &content_returned[0], body_size)?;
    }
    bcc.make_success_json(&json!({}))
}

fn get_single_offering_image(bcc: &BitcodeContext, url: &str, is_video: bool) -> CallResult {
    if is_video {
        return bcc.fetch_link_reader(json!(url));
    }
    bcc.fetch_link_reader(json!(format!("./rep{url}")))
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
