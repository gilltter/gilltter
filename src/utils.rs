use std::io::{Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use sha1::{Digest, Sha1};

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub fn get_file_info(path: &str) -> libc::stat {
    println!("Path: {}", path);
    let mut file_info: libc::stat = unsafe { std::mem::zeroed() };
    let file_info_result = unsafe {
        use std::ffi::CString;

        libc::lstat(
            CString::new(path).unwrap().as_ptr(),
            &mut file_info as *mut libc::stat,
        )
    };
    if file_info_result < 0 {
        eprintln!("Could not get file stats");
    }
    file_info
}

#[cfg(target_os = "windows")]
pub fn get_file_info(path: &str) {
    panic!("Not supported, windows users fuck off");
}

#[cfg(target_family = "unix")]
pub const fn get_separator() -> &'static str {
    "/"
}
#[cfg(target_family = "windows")]
pub const fn get_separator() -> &'static str {
    "\\"
}

pub fn generate_hash(content: &[u8]) -> String {
    // uncompressed (raw) bytes
    let mut hasher = Sha1::new();
    hasher.update(content);
    let result = hasher.finalize();
    let str = format!("{:x}", result);
    str
}
// penis
#[allow(dead_code)]
pub fn compress(contents: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(contents)?;
    let compressed = e.finish()?;
    Ok(compressed)
}

pub fn decompress(contents: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut d = ZlibDecoder::new(contents);
    let mut buf = Vec::new();
    d.read_to_end(&mut buf)?;
    Ok(buf)
}
