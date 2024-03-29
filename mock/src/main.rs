extern crate wapc;

extern crate wasmer;
extern crate wasmtime_provider;

extern crate base64;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate json_dotpath;
extern crate snailquote;
use std::sync::{Arc};

use elvwasm::ErrorKinds;
use std::fs::File;
use std::io::BufReader;
use json_dotpath::DotPaths;
use std::path::PathBuf;
use structopt::StructOpt;
use wasmer::{imports, Store, Universal};
use wasmer_compiler_cranelift::Cranelift;
use wasmtime_provider::WasmtimeEngineProvider;
use wapc::WapcHost;


use serde_derive::{Deserialize, Serialize};

static mut QFAB: MockFabric = MockFabric{
    fab : None
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RootMockFabric {
  pub library:Library,
  pub call:serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Object {
  pub hash: String,
  pub id: String,
  pub qlib_id: String,
  #[serde(rename = "type")]
  pub qtype: String,
  pub write_token: String,
  pub meta : serde_json::Map<String, serde_json::Value>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Library {
  pub id: String,
  pub objects: std::vec::Vec<Object>,
}

#[derive(Serialize, Deserialize,  Clone, Debug)]
pub struct MockFabric{
    fab : Option<RootMockFabric>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JPCRequest {
  pub jpc: String,
  pub params: serde_json::Map<String, serde_json::Value>
}

impl MockFabric{
    pub fn init(& mut self, path_to_json:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path_to_json)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let json_rep:RootMockFabric = serde_json::from_reader(reader)?;
        self.fab = Some(json_rep);
        return Ok("DONE".as_bytes().to_vec())
    }
    pub fn write_stream(&self, _json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in WriteStream");
        Ok("Not Implemented".as_bytes().to_vec())
    }
    pub fn sqmd_delete(&self, json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in SQMD delete");
        let j:JPCRequest = serde_json::from_str(json_rep)?;
        let path = j.params["path"].to_string();
        if  !path.is_empty(){
            let mut fab = self.fab.clone().unwrap();
            let p = &snailquote::unescape(&path).unwrap();
            let pp:String = p.chars().map(|x| match x {
                '/' => '.',
                _ => x
            }).collect();
            fab.library.objects[0].meta.dot_remove(&pp[1..])?;//{
            return Ok("DONE".as_bytes().to_vec())
        }else{
            println!("failed to find path argument");
        }
        Ok("FAILED".as_bytes().to_vec())
    }
    pub fn sqmd_set(&self, json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in SQMD set");
        let j:JPCRequest = serde_json::from_str(json_rep)?;
        let path = j.params["path"].to_string();
        let meta = j.params["meta"].to_string();
        if !path.is_empty(){
            let mut fab = self.fab.clone().unwrap();
            let p = &snailquote::unescape(&path).unwrap();
            let pp:String = p.chars().map(|x| match x {
                '/' => '.',
                _ => x
            }).collect();
            fab.library.objects[0].meta.dot_set(&pp[1..], meta)?;
            return Ok("DONE".as_bytes().to_vec())

        }else{
            println!("failed to find path argument");
        }
        Ok("FAILED".as_bytes().to_vec())
    }
    pub fn sqmd_get(&self, json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in SQMD get");
        let j:JPCRequest = serde_json::from_str(json_rep)?;
        let path = j.params["path"].to_string();
        if !path.is_empty(){
            let fab = self.fab.clone().unwrap();
            let p = &snailquote::unescape(&path).unwrap();
            let pp:String = p.chars().map(|x| match x {
                '/' => '.',
                _ => x
            }).collect();
            let gotten:Option<serde_json::Value> = fab.library.objects[0].meta.dot_get(&pp[1..])?;
            let ret = gotten.unwrap();
            println!("sqmd_get returning = {}", ret);
            return Ok(ret.to_string().as_bytes().to_vec())
        }else{
            println!("failed to find path argument");
        }
        Ok("FAILED".as_bytes().to_vec())
    }
    pub fn proxy_http(&self, _json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in ProxyHttp");
        let to_encode = r#"{"url" : {"type" : "application/json"}} "#.as_bytes();
        let enc = base64::encode(to_encode);
        Ok(format!(r#"{{"result": "{}"}}"#, enc).as_bytes().to_vec())
    }
    pub fn callback(&self, _json_rep:&str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        println!("in callback");
        let to_encode = r#"{"url" : {"type" : "application/json"}} "#.as_bytes();
        let enc = base64::encode(to_encode);
        Ok(format!(r#"{{"result": "{}"}}"#, enc).as_bytes().to_vec())
    }
    pub fn host_callback(i_cb:u64, id:&str, context:&str, method:&str, pkg:&[u8])-> std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>{
        let s_pkg = std::str::from_utf8(pkg)?;
        println!("In host callback, values i_cb = {} id = {} method = {} context = {}, pkg = {}", i_cb, id,method,context, s_pkg);
        match method {
            "SQMDGet" =>{
               unsafe{ QFAB.sqmd_get(s_pkg) }
            }
            "SQMDSet" =>{
                unsafe{ QFAB.sqmd_set(s_pkg) }
             }
            "SQMDDelete" =>{
                unsafe{ QFAB.sqmd_delete(s_pkg) }
             }
            "Write" => {
                unsafe{ QFAB.write_stream(s_pkg) }
            }
            "Callback" => {
                unsafe{ QFAB.callback(s_pkg) }
            }
            "ProxyHttp" => {
                unsafe{ QFAB.proxy_http(s_pkg) }
            }
            _ => {
                Err(Box::new(ErrorKinds::NotExist("Method not handled")))
            }
        }
    }
}

struct WasmerHolder{
    _instance:wasmer::Instance
}

impl wapc::WebAssemblyEngineProvider for WasmerHolder{
    fn init(&mut self, _host: Arc<wapc::ModuleState>) -> std::result::Result<(), Box<dyn std::error::Error>>{
        Ok(())
    }
    fn call(&mut self, _op_length: i32, _msg_length: i32) -> std::result::Result<i32, Box<dyn std::error::Error>>{
        //.instance.store().engine.
        Ok(0)
    }
    fn replace(&mut self, _bytes: &[u8]) -> std::result::Result<(), Box<dyn std::error::Error>>{
        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-w, --wasmer) will be deduced from the field's name
    #[structopt(short, long)]
    mode: String,

    /// Input wasm
    #[structopt(parse(from_os_str))]
    input: PathBuf,


    /// Input fabric/call
    #[structopt(parse(from_os_str))]
    fabric: PathBuf,

}

pub fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("In main");
    let opt = Opt::from_args();
    unsafe{QFAB.init(&opt.fabric.into_os_string().into_string().unwrap())?;}
    let module_wat = std::fs::read(&opt.input.into_os_string().into_string().unwrap())?;
    let h;
    if opt.mode == "wasmer"{
        let compiler = Cranelift::new();
        // Put it into an engine and add it to the store
        let store = Store::new(&Universal::new(compiler).engine());
        let wasmer_mod = wasmer::Module::new(&store, &module_wat)?;
        let import_object = imports! {};
        let instance = wasmer::Instance::new(&wasmer_mod, &import_object)?;
        let wasm_holder = WasmerHolder{_instance:instance};
        let host = wapc::WapcHost::new(Box::new(wasm_holder), MockFabric::host_callback)?;
        h = Some(host);
    }else{
        let engine = WasmtimeEngineProvider::new(&module_wat, None);
        let host = WapcHost::new(Box::new(engine), MockFabric::host_callback)?;
        h = Some(host)
    }

    h.unwrap().call("_JPC", &serde_json::to_vec(unsafe{&QFAB.fab.clone().unwrap().call}).unwrap())?;
    Ok(())
}

