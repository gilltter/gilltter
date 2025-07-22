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
    panic!("Not supported, windows users fuck off for now");
}
