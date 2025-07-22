use std::fs;
use std::io::ErrorKind;
use std::path;

pub const GILLTTER_PATH: &str = ".gilltter";
pub const GILLTER_OBJECTS_DIR: &str = "objects";
pub const GILLTER_HEAD_FILE: &str = "head";
pub const GILLTER_STATE_FILE: &str = "state"; // A.k.a git INDEX file

pub fn make_sure_gilltter_dir_exists() -> anyhow::Result<()> {
    if !fs::exists(GILLTTER_PATH)?
        || !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR)?
        || !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_HEAD_FILE)?
        || !fs::exists(String::from(GILLTTER_PATH) + "/" + GILLTER_STATE_FILE)?
    {
        if let Err(why) = fs::create_dir(GILLTTER_PATH) {
            if why.kind() == ErrorKind::PermissionDenied {
                eprintln!("Could not create Gilltter project directory: {}", why)
            }
        }
        let objects_dir =
            path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR);
        fs::create_dir(objects_dir).unwrap_or(()); // At this point we should be allowed to create files/dirs (in terms of permissions)

        let head_file = path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_HEAD_FILE);
        if let Err(_) = fs::File::create(head_file) {} // Drops here therefore closing file

        let index_file =
            path::PathBuf::from(String::from(GILLTTER_PATH) + "/" + GILLTER_STATE_FILE);
        if let Err(_) = fs::File::create(index_file) {}
    }
    Ok(())
}
