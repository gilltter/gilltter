use std::{ffi::CString, fs, path};

const GILLTTER_PATH: &str = ".gilltter";
const GILLTER_OBJECTS_DIR: &str = "objects";
const GILLTER_HEAD_FILE: &str = "head";
const GILLTER_STATE_FILE: &str = "state"; // A.k.a git INDEX file

#[cfg(target_os = "linux")]
fn get_file_info(path: &str) -> libc::stat {
    println!("Path: {}", path);
    let mut file_info: libc::stat = unsafe { std::mem::zeroed() };
    let file_info_result = unsafe {
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
fn get_file_info(path: &str) {
    panic!("Not supported, windows users fuck off for now");
}

fn make_sure_gilltter_dir_exists() -> anyhow::Result<()> {
    if !fs::exists(GILLTTER_PATH)?
        && !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR)?
        || !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_HEAD_FILE)?
        || !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_STATE_FILE)?
    {
        if let Err(why) = fs::create_dir(GILLTTER_PATH) {
            eprintln!("Could not create Gilltter project directory: {}", why);
        }
        let objects_dir =
            path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR);
        fs::create_dir(objects_dir)?; // At this point we should be allowed to create files/dirs (in terms of permissions)

        let head_file = path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_HEAD_FILE);
        fs::File::create(head_file)?; // Drops here therefore closing file

        let index_file =
            path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_STATE_FILE);
        fs::File::create(index_file)?;
    }
    Ok(())
}

fn main() {
    if let Err(why) = make_sure_gilltter_dir_exists() {
        eprintln!(
            "Could not verify, that gilltter directory and its files exist: {}",
            why
        );
    }

    let file_info = get_file_info(".gilltter");
    if (file_info.st_mode & libc::S_IFMT) == libc::S_IFDIR {
        println!("{} is a directory, all good 👍", GILLTTER_PATH);
    }

    println!("Hello, Dear gilltter users!");
}
