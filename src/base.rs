use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::path::Path;

use crate::objects::ObjectDump;
use crate::objects::blob::Blob;
use crate::utils;

pub const GILLTTER_PATH: &'static str = ".gilltter";

pub const GILLTER_OBJECTS_DIR: &'static str = "objects";
pub const GILLTER_HEAD_FILE: &'static str = "head";
pub const GILLTER_STATE_FILE: &'static str = "state"; // A.k.a git INDEX file
pub const GILLTER_BRANCHES_DIR: &'static str = "branches";
pub const GILLTER_CONFIG_FILE: &'static str = "config";
pub const GILLTTER_INDEX_FILE: &'static str = "index";
pub const GILLTTER_IGNORE_FILE: &'static str = ".gignore";

pub fn create_gilltter_project() -> anyhow::Result<()> {
    if !fs::exists(GILLTTER_PATH)?
        || !fs::exists(String::from(GILLTTER_PATH) + utils::get_separator() + GILLTER_OBJECTS_DIR)?
        || !fs::exists(String::from(GILLTTER_PATH) + utils::get_separator() + GILLTER_HEAD_FILE)?
        || !fs::exists(String::from(GILLTTER_PATH) + utils::get_separator() + GILLTER_STATE_FILE)?
        || !fs::exists(String::from(GILLTTER_PATH) + utils::get_separator() + GILLTER_BRANCHES_DIR)?
        || !fs::exists(String::from(GILLTTER_PATH) + utils::get_separator() + GILLTER_CONFIG_FILE)?
    {
        if let Err(why) = fs::create_dir(GILLTTER_PATH) {
            if why.kind() == ErrorKind::PermissionDenied {
                eprintln!("Could not create Gilltter project directory: {}", why)
            }
        }

        let objects_dir = Path::new(GILLTTER_PATH).join(GILLTER_OBJECTS_DIR);
        fs::create_dir(objects_dir).unwrap_or(()); // At this point we should be allowed to create files/dirs (in terms of permissions)

        let head_file = Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE);
        if let Err(_) = fs::File::create(head_file) {} // Drops here therefore closing file

        let idx_file = Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE);
        if let Err(_) = fs::File::create(idx_file) {} // Drops here therefore closing file

        let index_file = Path::new(GILLTTER_PATH).join(GILLTER_STATE_FILE);
        if let Err(_) = fs::File::create(index_file) {}

        let local_config_file = Path::new(GILLTTER_PATH).join(GILLTER_CONFIG_FILE);
        if let Err(_) = fs::File::create(local_config_file) {}

        let branches_dir = Path::new(GILLTTER_PATH).join(GILLTER_BRANCHES_DIR);
        fs::create_dir(branches_dir).unwrap_or(());
    }
    Ok(())
}

pub(crate) fn gilltter_init() -> anyhow::Result<()> {
    create_gilltter_project()
}

pub(crate) fn gilltter_add(filepath: &Path) -> anyhow::Result<String> {
    let mut file = File::open(filepath)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let mut blob = Blob::new();
    blob.set_data(&contents);

    let sha_hash = blob.dump_to_file()?;
    Ok(sha_hash)
}
