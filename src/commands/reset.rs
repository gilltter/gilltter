use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_HEAD_FILE, GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump, commit::Commit},
    utils,
};

pub fn reset(value: i32) -> anyhow::Result<()> {
    // Read sha of the current commit
    let head_file_path = Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE);
    let mut head_file = File::open(&head_file_path)?;
    let mut head_bytes = Vec::new();
    head_file.read_to_end(&mut head_bytes)?;

    // Read current commit
    let commit_sha = String::from_utf8_lossy(&head_bytes)
        .to_string()
        .trim()
        .to_string();
    let commit_path = Path::new(GILLTTER_PATH)
        .join(GILLTER_OBJECTS_DIR)
        .join(commit_sha);
    let mut commit_file = Commit::from_file(&commit_path)?;
    // Iterate commits until we get to the one we need
    for _ in 0..value {
        // Iterate until the commit we need
        if let Some(parent_commit_sha) = commit_file.get_parent_commit_sha() {
            let parent_commit_path = Path::new(GILLTTER_PATH)
                .join(GILLTER_OBJECTS_DIR)
                .join(parent_commit_sha);
            commit_file = Commit::from_file(&parent_commit_path)?;
        } else {
            return Err(anyhow!("Value is too big, commit history is not that long"));
        }
    }

    let commit_bytes = commit_file.convert_to_bytes()?;
    let commit_sha = utils::generate_hash(&commit_bytes);
    // Set current commit in HEAD to value, this way it is a soft reset
    let path = Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE);
    let mut head_file = OpenOptions::new().truncate(true).write(true).open(&path)?;
    head_file.write_all(commit_sha.as_bytes())?;
    Ok(())
}
