mod old_man;

extern crate elvwasm;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;
use crate::old_man::S_OLD_MAN;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

use elvwasm::ErrorKinds;
use snailquote::unescape;

extern crate image;
use base64::engine::general_purpose;
use base64::Engine;
use image::GenericImageView;
use image::{
    error::{DecodingError, ImageFormatHint},
    jpeg::JpegEncoder,
};

use elvwasm::{
    implement_bitcode_module, jpc, register_handler, BitcodeContext, NewStreamResult,
    ReadStreamResult, WriteResult,
};

implement_bitcode_module!("crawl", do_crawl, "proxy", do_proxy, "image", do_img);

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WatermarkJson {
    #[serde(default)]
    pub x: String,
    #[serde(default)]
    pub y: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub opacity: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageWatermark {
    #[serde(default)]
    pub image_watermark: WatermarkJson,
}
fn parse_asset(path: &str) -> String {
    let mut pos: Vec<&str> = path.split('/').collect();
    if pos.len() > 2 {
        pos = pos[3..].to_owned();
        return pos.join("/");
    }
    "".to_owned()
}

fn get_offering(bcc: &BitcodeContext, input_path: &str) -> CallResult {
    let v: Vec<&str> = input_path.split('/').collect();
    let mut s = "";
    if v.len() > 1 {
        s = v[2];
    }
    let json_path = format!("/public/image/offerings/{s}");
    // input_path should just be offering
    bcc.sqmd_get_json(&json_path)
}

fn fab_file_to_image(
    bcc: &&mut elvwasm::BitcodeContext,
    stream_id: &str,
    asset_path: &str,
) -> image::ImageResult<image::DynamicImage> {
    let f2s = match bcc.q_file_to_stream(stream_id, asset_path, &bcc.request.q_info.hash) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    let written: WriteResult = match serde_json::from_slice(&f2s) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    BitcodeContext::log(&format!("written = {}", &written.written));
    let read_res = match bcc.read_stream(stream_id.to_owned(), written.written) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    let read_data: ReadStreamResult = match serde_json::from_slice(&read_res) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    let base = read_data.result;
    let buffer = match general_purpose::STANDARD.decode(base) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::Decoding(
                DecodingError::from_format_hint(ImageFormatHint::Name(format!("{x}"))),
            ))
        }
    };
    BitcodeContext::log(&format!("bytes read = {}", read_data.retval));
    image::load_from_memory_with_format(&buffer, image::ImageFormat::Jpeg)
}

fn do_img(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let offering = get_offering(bcc, &http_p.path)?;
    BitcodeContext::log(&format!(
        "json={}",
        std::str::from_utf8(&offering).unwrap_or_default()
    ));
    let offering_json: WatermarkJson = serde_json::from_slice(&offering)?;
    let asset_path = parse_asset(&http_p.path);
    BitcodeContext::log(&format!(
        "offering = {:?} asset_path = {}",
        &offering_json, &asset_path
    ));
    let res = bcc.new_stream()?;
    let stream_main: NewStreamResult = serde_json::from_slice(&res)?;
    defer! {
      BitcodeContext::log("Closing main stream");
      let _ = bcc.close_stream(stream_main.stream_id.clone());
    }
    let img = &mut fab_file_to_image(&bcc, &stream_main.stream_id, &asset_path)?;
    let (h, w) = img.dimensions();
    let height_str = &http_p.query["height"];
    let height: usize = height_str[0].parse().unwrap_or(0);
    let width_factor: f32 = h as f32 / w as f32;
    let new_width: usize = (width_factor * height as f32) as usize;
    if !offering_json.image.is_empty() {
        BitcodeContext::log("WATERMARK");
        let res = bcc.new_stream()?;
        let stream_wm: NewStreamResult = serde_json::from_slice(&res)?;
        defer! {
          BitcodeContext::log(&format!("Closing watermark stream {}", &stream_wm.stream_id));
          let _ = bcc.close_stream(stream_wm.stream_id.clone());
        }
        let wm = fab_file_to_image(&bcc, &stream_wm.stream_id, &offering_json.image)?;
        let wm_height = offering_json.height.parse::<f32>().unwrap_or_default();
        let _opacity = offering_json.opacity.parse::<f32>().unwrap_or_default();
        let wm_thumb = image::imageops::thumbnail(
            &wm,
            (height as f32 * wm_height * width_factor) as u32,
            (height as f32 * wm_height) as u32,
        );
        BitcodeContext::log("THUMBNAIL");
        image::imageops::overlay(
            img,
            &wm_thumb,
            offering_json.x.parse::<u32>().unwrap_or(10),
            offering_json.y.parse::<u32>().unwrap_or(10),
        );
        BitcodeContext::log("OVERLAY");
    } else {
        BitcodeContext::log("NO WATERMARK!!!");
    }
    let br = img.resize(
        new_width as u32,
        height as u32,
        image::imageops::FilterType::Lanczos3,
    );
    let mut bytes: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new(&mut bytes);
    encoder.encode(&br.to_bytes(), new_width as u32, height as u32, br.color())?;
    bcc.callback(200, "image/jpeg", bytes.len())?;
    BitcodeContext::write_stream_auto(bcc.request.id.clone(), "fos", &bytes)?;
    bcc.make_success_json(
        &json!(
        {
            "headers" : "application/json",
            "body" : "SUCCESS",
            "result" : 0,
        }),
        &bcc.request.id,
    )
}

fn do_proxy(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    BitcodeContext::log(&format!(
        "In DoProxy hash={} headers={:#?} query params={qp:#?}",
        &bcc.request.q_info.hash, &http_p.headers
    ));
    let res = bcc.sqmd_get_json("/request_parameters")?;
    let mut meta_str: String = match String::from_utf8(res) {
        Ok(m) => m,
        Err(e) => {
            return bcc.make_error_with_kind(ErrorKinds::Invalid(format!(
                "failed to parse request params error={e}"
            )))
        }
    };
    meta_str = meta_str
        .replace("${API_KEY}", &qp["API_KEY"][0].to_string())
        .replace("${QUERY}", &qp["QUERY"][0].to_string())
        .replace("${CONTEXT}", &qp["CONTEXT"][0].to_string());
    BitcodeContext::log(&format!("MetaData = {}", &meta_str));
    let req: serde_json::Map<String, serde_json::Value> =
        match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&meta_str) {
            Ok(m) => m,
            Err(e) => {
                return bcc.make_error_with_kind(ErrorKinds::Invalid(format!(
                    "serde_json::from_str failed error = {e}"
                )))
            }
        };
    let proxy_resp = bcc.proxy_http(Some(json!({ "request": req })))?;
    let proxy_resp_json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&proxy_resp).unwrap_or("{}"))?;
    let client_response = serde_json::to_vec(&proxy_resp_json["result"])?;
    let id = &bcc.request.id;
    bcc.callback(200, "application/json", client_response.len())?;
    BitcodeContext::write_stream_auto(id.clone(), "fos", &client_response)?;
    bcc.make_success_json(
        &json!(
        {
            "headers" : "application/json",
            "body" : "SUCCESS",
            "result" : 0,
        }),
        id,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

fn extract_body(v: Value) -> Option<Value> {
    let obj = match v.as_object() {
        Some(v) => v,
        None => return None,
    };
    let mut full_result = true;
    let res = match obj.get("result") {
        Some(m) => m,
        None => match obj.get("http") {
            Some(h) => {
                full_result = false;
                h
            }
            None => return None,
        },
    };
    if full_result {
        let http = match res.get("http") {
            Some(h) => h,
            None => return None,
        };
        return http.get("body").cloned();
    }
    res.get("body").cloned()
}

fn do_crawl(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let qp = &http_p.query;
    bcc.log_info(&format!(
        "In do_crawl hash={} headers={:#?} query params={qp:#?}",
        &bcc.request.q_info.hash, &http_p.headers
    ))?;
    let id = bcc.request.id.clone();
    let nib_res = serde_json::from_slice(&bcc.new_index_builder(json!({}))?)?;
    let dir = match extract_body(nib_res) {
        Some(v) => match v.get("dir") {
            Some(d) => match unescape(&d.to_string()) {
                Ok(u) => u,
                Err(e) => {
                    return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(format!(
                        "unescape failed on directory error={e}"
                    )))
                }
            },
            None => {
                return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(
                    "could not find dir in new_index_builder return".to_string(),
                ))
            }
        },
        None => {
            return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(
                "could not find body in new_index_builder return".to_string(),
            ))
        }
    };
    let ft_json: serde_json::Value = serde_json::from_slice(&bcc.builder_add_text_field(Some(
        json!({ "field_name": "title", "type": 2_u8, "stored": true}),
    ))?)?;
    let field_title = match extract_body(ft_json) {
        Some(o) => o.get("field").unwrap().as_u64(),
        None => {
            return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(
                "could not find key document-create-id".to_string(),
            ))
        }
    };
    let fb_json: serde_json::Value = serde_json::from_slice(&bcc.builder_add_text_field(Some(
        json!({ "field_name": "body", "type": 2_u8 , "stored": true}),
    ))?)?;
    let field_body = match extract_body(fb_json) {
        Some(o) => o.get("field").unwrap().as_u64(),
        None => {
            return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(
                "could not find key document-create-id".to_string(),
            ))
        }
    };
    bcc.builder_build(None)?;
    let doc_old_man: serde_json::Value = serde_json::from_slice(&bcc.document_create(None)?)?;
    console_log(&format!("obj_old = {:?}", &doc_old_man));
    let o_doc_id = match extract_body(doc_old_man) {
        Some(o) => o.get("document-create-id").unwrap().as_u64(),
        None => {
            return bcc.make_error_with_kind(ErrorKinds::BadHttpParams(
                "could not find key document-create-id".to_string(),
            ))
        }
    };
    let doc_id = o_doc_id.unwrap();
    bcc.log_info(&format!(
        "doc_id={doc_id}, field_title = {}, field_body={}",
        field_title.unwrap(),
        field_body.unwrap()
    ))?;
    bcc.document_add_text(Some(json!({ "field": field_title.unwrap(), "value": "The Old Man and the Sea", "doc_id": doc_id})))?;
    bcc.document_add_text(Some(
        json!({ "field": field_body.unwrap(), "value": S_OLD_MAN, "doc_id": doc_id}),
    ))?;
    bcc.document_create_index(None)?;
    bcc.index_create_writer(None)?;
    bcc.index_add_document(Some(json!({ "document_id": doc_id })))?;
    bcc.index_writer_commit(None)?;
    let part_u8 = bcc.archive_index_to_part(&dir)?;
    let part_hash: serde_json::Value = serde_json::from_slice(&part_u8)?;
    let b = extract_body(part_hash.clone());
    let body_hash = b.unwrap_or_else(|| json!({}));
    bcc.callback(200, "application/json", part_u8.len())?;
    BitcodeContext::write_stream_auto(id.clone(), "fos", &part_u8)?;
    bcc.log_info(&format!("part hash = {part_hash}, body = {body_hash}"))?;
    bcc.make_success_json(
        &json!(
        {
            "headers" : "application/json",
            "body" : "SUCCESS",
            "result" : 0,
        }),
        &id,
    )
}
