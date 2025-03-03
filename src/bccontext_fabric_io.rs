extern crate thiserror;
extern crate wapc_guest as guest;

use crate::BitcodeContext;

use std::io::{ErrorKind, Read, SeekFrom};

pub struct FabricStreamReader<'a> {
    stream_id: String,
    bcc: &'a BitcodeContext,
}

impl<'a> FabricStreamReader<'a> {
    pub fn new(sid: String, bcc_in: &'a BitcodeContext) -> FabricStreamReader<'a> {
        FabricStreamReader {
            stream_id: sid,
            bcc: bcc_in,
        }
    }
}

impl Read for FabricStreamReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_bytes = match self.bcc.read_stream(self.stream_id.clone(), buf.len()) {
            Ok(rb) => rb,
            Err(e) => {
                let _ = self.bcc.log_error(&format!("Error reading stream: {e}"));
                Vec::new()
            }
        };
        if read_bytes.is_empty() {
            return Ok(0);
        }
        let len = std::cmp::min(buf.len(), read_bytes.len());
        buf[..len].copy_from_slice(&read_bytes[..len]);
        let _ = self
            .bcc
            .log_debug(&format!("Read {len} bytes in FabricStreamReader"));
        Ok(len)
    }
}

//FabricStreamWriter is a struct that implements the Write trait
//The struct is used to write the image bits to the qfab based stream
// The is no buffer in the struct as the BufWriter will write immediately to "fos" of qfab's context
#[derive(Debug)]
pub struct FabricStreamWriter<'a> {
    pub bcc: &'a BitcodeContext,
    stream_id: String,
    pub size: usize,
}

impl FabricStreamWriter<'_> {
    pub fn new(bcc: &BitcodeContext, sid: String, sz: usize) -> FabricStreamWriter<'_> {
        FabricStreamWriter {
            bcc,
            stream_id: sid,
            size: sz,
        }
    }
}
impl std::io::Write for FabricStreamWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match self.bcc.write_stream(&self.stream_id, buf) {
            Ok(s) => {
                let w: crate::WritePartResult = serde_json::from_slice(&s)?;
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

impl std::io::Seek for FabricStreamWriter<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        let (offset, whence) = match pos {
            SeekFrom::Start(offset) => (offset as i64, 0),
            SeekFrom::Current(offset) => (offset, 1),
            SeekFrom::End(offset) => (offset, 2),
        };
        match self.bcc.seek_stream(&self.stream_id, offset, whence) {
            Ok(_) => Ok(self.size as u64),
            Err(e) => Err(std::io::Error::new(ErrorKind::Other, e)),
        }
    }
}

impl std::io::Seek for FabricStreamReader<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        let (offset, whence) = match pos {
            SeekFrom::Start(offset) => (offset as i64, 0),
            SeekFrom::Current(offset) => (offset, 1),
            SeekFrom::End(offset) => (offset, 2),
        };
        match self.bcc.seek_stream(&self.stream_id, offset, whence) {
            Ok(_) => Ok(0),
            Err(e) => Err(std::io::Error::new(ErrorKind::Other, e)),
        }
    }
}
