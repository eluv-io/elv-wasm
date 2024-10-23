extern crate elvwasm;
extern crate serde_json;
#[macro_use(defer)]
extern crate scopeguard;
use std::collections::HashMap;

use elvwasm::ErrorKinds;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

extern crate image;
use image::jpeg::JpegEncoder;
use image::GenericImageView;

use elvwasm::{
    implement_bitcode_module, jpc, register_handler, BitcodeContext, NewStreamResult, WriteResult,
};

implement_bitcode_module!("image", do_img, "content", do_img);

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WatermarkJson {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub image: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub opacity: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageWatermark {
    #[serde(default)]
    pub image_watermark: WatermarkJson,
}

fn parse_asset(path: &str) -> String {
    let pos: Vec<&str> = path.split('/').collect();
    if pos.len() > 2 {
        let new_pos: Vec<&str> = pos[3..].to_vec();
        return "/".to_string() + &new_pos.join("/");
    }
    "".to_owned()
}

fn get_offering(bcc: &BitcodeContext, input_path: &str) -> CallResult {
    let v: Vec<&str> = input_path.split('/').collect();
    let mut s = "";
    if v.len() > 1 {
        s = v[2];
    }
    let json_path = format!("/image/offerings/{s}");
    // input_path should just be offering
    bcc.sqmd_get_json(&json_path)
}

fn fab_file_to_image(
    bcc: &&mut elvwasm::BitcodeContext,
    stream_id: &str,
    asset_path: &str,
) -> image::ImageResult<image::DynamicImage> {
    let written: WriteResult = match bcc
        .q_file_to_stream(stream_id, asset_path, &bcc.request.q_info.hash)
        .try_into()
    {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    let read_data = match bcc.read_stream(stream_id.to_owned(), written.written) {
        Ok(v) => v,
        Err(x) => {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                x,
            )))
        }
    };
    let buffer = read_data;
    image::load_from_memory_with_format(&buffer, image::ImageFormat::Jpeg)
}

fn do_img(bcc: &mut elvwasm::BitcodeContext) -> CallResult {
    let http_p = &bcc.request.params.http;
    let offering_json: ImageWatermark = elvwasm::convert(&get_offering(bcc, &http_p.path))?;
    let asset_path = parse_asset(&http_p.path);
    bcc.log_info(&format!(
        "offering = {:?} asset_path = {} http_path= {}",
        &offering_json, &asset_path, &http_p.path
    ))?;
    let stream_main: NewStreamResult = bcc.new_stream().try_into()?;
    defer! {
      let _ = bcc.close_stream(stream_main.stream_id.clone());
    }
    let qp = &http_p.query;
    let v_none = vec!["".to_string()];

    let content_disp = qp
        .get("header-x_set_content_disposition")
        .unwrap_or(&v_none);

    let img = &mut fab_file_to_image(&bcc, &stream_main.stream_id, &asset_path)?;
    let (w, h) = img.dimensions();
    let v = &vec![h.to_string()];
    let height_str = &http_p.query.get("height").unwrap_or(v);
    let outer_height: usize = height_str[0].parse().unwrap_or(h as usize);
    let width_factor: f32 = w as f32 / h as f32;
    let outer_width: usize = (width_factor * outer_height as f32) as usize;
    bcc.log_info(&format!(
        "x={} y={} outerh = {} outerw = {}",
        h, w, outer_height, outer_width
    ))?;
    let mut br = img.resize(
        outer_width as u32,
        outer_height as u32,
        image::imageops::FilterType::Lanczos3,
    );
    if !offering_json.image_watermark.image.is_empty() {
        bcc.log_info("WATERMARK")?;
        let stream_wm: NewStreamResult = bcc.new_stream().try_into()?;
        defer! {
          let _ = bcc.close_stream(stream_wm.stream_id.clone());
        }
        let wm_filename = match offering_json.image_watermark.image.get("/") {
            Some(f) => f
                .as_str()
                .ok_or(ErrorKinds::Invalid("Invalid link type".to_string()))?,
            None => {
                return Err(Box::new(ErrorKinds::Invalid(
                    "Invalid link type, no link provided".to_string(),
                )))
            }
        };
        bcc.log_info(&format!("watermark image {}", &wm_filename[7..]))?;
        let wm = fab_file_to_image(&bcc, &stream_wm.stream_id, &wm_filename[7..])?;
        let wm_height_scale = offering_json.image_watermark.height;
        let opacity = offering_json.image_watermark.opacity;
        let mut wm_thumb = image::imageops::thumbnail(
            &wm,
            (outer_width as f32 * wm_height_scale) as u32,
            (outer_height as f32 * wm_height_scale) as u32,
        );
        wm_thumb
            .as_flat_samples_mut()
            .samples
            .chunks_mut(4)
            .for_each(|channels: &mut [u8]| channels[3] = (channels[3] as f32 * opacity) as u8);
        bcc.log_info("THUMBNAIL")?;
        image::GenericImage::copy_from(
            &mut br,
            &wm_thumb,
            (outer_width as f32 * wm_height_scale / 2.0) as u32,
            (outer_height as f32 * wm_height_scale / 2.0) as u32,
        )?;
    } else {
        bcc.log_info("NO WATERMARK!!!")?;
    }

    bcc.log_info(&format!("DynImage {:?}", br.bounds()))?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut encoder = JpegEncoder::new(&mut bytes);
    encoder.encode(&br.to_bytes(), br.width(), br.height(), br.color())?;
    bcc.callback_disposition(200, "image/jpeg", bytes.len(), &content_disp[0], "1.0.0")?;
    bcc.write_stream("fos", &bytes)?;
    bcc.make_success_json(&json!({}))
}
