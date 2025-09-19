use std::{os::unix::fs::MetadataExt, path::Path};

use crate::{
    base::{self, GILLTTER_INDEX_FILE, GILLTTER_PATH},
    index::index::{Index, IndexEntry, IndexType},
    objects::{ObjectDump, ObjectPump},
};

pub fn add(filepath: &Path) -> anyhow::Result<()> {
    let mut index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE))
        .expect("Index fucked up");

    // Notice: It is OK if the file already exists, means it is probably unstaged and user wanna stage it
    // If the file already exists, find it and update its metadata, or just delete and add again
    // First way:
    println!("Filepath: {:?}, exists: {}", filepath, filepath.exists());
    if !filepath.exists() {
        // If file is deleted, then just remove it from the index
        index.remove(filepath);
        let name = index.dump_to_file()?;

        println!("Dumped name: {}", name);
        return Ok(());
    }
    index.remove(filepath);
    // I'd consider this a nasty hack
    // TODO: Do it the second way i guess?

    let file_sha1 = base::gilltter_add(filepath)?;

    let add_file_metadata = std::fs::metadata(filepath)?;
    let entry: IndexEntry = IndexEntry::new(
        add_file_metadata.ctime(),
        add_file_metadata.mtime(),
        add_file_metadata.size(),
        if add_file_metadata.is_symlink() {
            IndexType::SymbolicLink
        } else {
            IndexType::RegularFile
        },
        filepath.to_owned(),
        file_sha1,
    );

    index.add(entry);

    index.dump_to_file()?;
    Ok(())
}
