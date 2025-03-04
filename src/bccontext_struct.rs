extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate thiserror;
extern crate wapc_guest as guest;

use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str;
use wapc_guest::CallResult;

/// Q is a bitcode representation of an individual piece of content from the fabric
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Q {
    pub id: String,
    pub hash: String,
    #[serde(default)]
    pub write_token: String,
    #[serde(rename = "type")]
    pub q_type: String,
    pub qlib_id: String,
    #[serde(default)]
    pub meta: serde_json::Value,
    #[serde(default)]
    pub size_stats: SizeStats,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SizeStats {
    pub parts: i32,
    #[serde(default)]
    pub size: String,
    pub size_bytes: i64,
}

/// Bitcode representation of a fabric size error
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SeekResult {
    pub offset: u64,
}

/// Bitcode representation of a fabric size error
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QError {
    pub error: String,
    #[serde(default)]
    pub item: Q,
}

/// QRef is a bitcode representation of versioned content from the fabric
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QRef {
    pub id: String,
    pub versions: Vec<Q>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WritePartResult {
    pub written: usize,
}

impl TryFrom<CallResult> for WritePartResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<WritePartResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateResult {
    pub qid: String,
    pub qwtoken: String,
}

impl TryFrom<CallResult> for CreateResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<CreateResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreatePartResult {
    pub qphash: String,
    pub size: i64,
}

impl TryFrom<CallResult> for CreatePartResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<CreatePartResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SystemTimeResult {
    pub time: u64,
}

impl TryFrom<CallResult> for SystemTimeResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<SystemTimeResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ExternalCallResult {
    pub function_return: serde_json::Value,
    pub fout: String,
    pub format: Vec<String>,
}

impl TryFrom<CallResult> for ExternalCallResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<ExternalCallResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

impl TryFrom<Vec<u8>> for ExternalCallResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: Vec<u8>,
    ) -> Result<ExternalCallResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FinalizeCallResult {
    pub qid: String,
    pub qhash: String,
}

impl TryFrom<CallResult> for FinalizeCallResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<FinalizeCallResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

/// Bitcode representation of a full content listing given an optional filter
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QList {
    #[serde(default)]
    pub filter: String,
    pub contents: Vec<QRef>,
    #[serde(default)]
    pub errors: Vec<QError>,
}

impl TryFrom<CallResult> for QList {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<QList, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

/// Bitcode representation of a fabric FileStream
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileStream {
    /// bitcode stream handle
    pub stream_id: String,
    /// fabric file path
    pub file_name: String,
}

impl TryFrom<CallResult> for FileStream {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<FileStream, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}
/// Bitcode representation of the size of a fabric stream
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileStreamSize {
    pub file_size: usize,
}

impl TryFrom<CallResult> for FileStreamSize {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<FileStreamSize, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

/// Bitcode representation of the JPC (JSON Procedure Call) parameters from a client request
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct JpcParams {
    pub http: HttpParams,
}

/// Bitcode representation of the http parameters from a client request
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HttpParams {
    #[serde(default)]
    pub headers: HashMap<String, Vec<String>>,
    pub path: String,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    pub query: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub verb: String,
    #[serde(default)]
    pub fragment: String,
    #[serde(default)]
    pub content_length: usize,
    #[serde(default)]
    pub client_ip: String,
    #[serde(default)]
    pub self_url: String,
    #[serde(default)]
    pub proto: String,
    #[serde(default)]
    pub host: String,
}

/// Bitcode representation of a content sans meta data
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QInfo {
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub id: String,
    pub qlib_id: String,
    #[serde(rename = "type")]
    pub qtype: String,
    #[serde(default)]
    pub write_token: String,
}

impl QInfo {
    pub fn qhot(&self) -> String {
        let s: String = if !self.write_token.is_empty() {
            self.write_token.to_string()
        } else {
            self.hash.to_string()
        };
        s
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QPart {
    #[serde(default)]
    pub write_token: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub size: i64,
}

impl TryFrom<CallResult> for QPart {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<QPart, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QPartListContents {
    #[serde(default)]
    pub content: Q,
    #[serde(default)]
    pub parts: Vec<QPart>,
}

impl TryFrom<CallResult> for QPartListContents {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<QPartListContents, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QPartList {
    pub part_list: QPartListContents,
}

impl TryFrom<CallResult> for QPartList {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<QPartList, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QPartInfo {
    pub content: Q,
    pub part: QPart,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QFileToStreamResult {
    #[serde(default)]
    pub written: usize,
    #[serde(default)]
    pub mime_type: String,
}

impl TryFrom<CallResult> for QFileToStreamResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<QFileToStreamResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WriteResult {
    #[serde(default)]
    pub written: usize,
}

impl TryFrom<CallResult> for WriteResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<WriteResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FetchResult {
    #[serde(default)]
    pub status: usize,
    #[serde(default)]
    pub headers: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub body: String,
}

impl TryFrom<CallResult> for FetchResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<FetchResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

impl TryFrom<Vec<u8>> for FetchResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: Vec<u8>,
    ) -> Result<FetchResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModifyResult {
    #[serde(default)]
    pub qwtoken: String,
}
impl TryFrom<CallResult> for ModifyResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<ModifyResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

/// Bitcode representation of a incomming client request
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Request {
    pub id: String,
    pub jpc: String,
    pub method: String,
    pub params: JpcParams,
    #[serde(rename = "qinfo")]
    pub q_info: QInfo,
}

/// Bitcode representation of a request back to the fabric as a consequnce of processing the request
/// In order for bitcode to respond to a primary request from a client, the bitcode must gather info
/// from the fabric for processing.  This structure represents the data to such a call.
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub jpc: String,
    pub params: serde_json::Value,
    pub id: String,
    pub module: String,
    pub method: String,
}

/// Bitcode representation of a result from new_stream
/// ```
/// fn do_something<'s>(bcc: &'s mut elvwasm::BitcodeContext) -> wapc_guest::CallResult {
///   let res = bcc.new_stream()?;
///   let stream1:elvwasm::NewStreamResult = serde_json::from_slice(&res)?;
///   // stream1.stream_id has new id
///   Ok(res)
/// }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewStreamResult {
    pub stream_id: String,
}

impl TryFrom<CallResult> for NewStreamResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<NewStreamResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadCount {
    pub read: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LROResult {
    pub lro_handle: String,
}

impl TryFrom<CallResult> for LROResult {
    type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
    fn try_from(
        cr: CallResult,
    ) -> Result<LROResult, Box<dyn std::error::Error + Sync + Send + 'static>> {
        Ok(serde_json::from_slice(&cr?)?)
    }
}
