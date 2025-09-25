use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_HEAD_FILE, GILLTER_OBJECTS_DIR, GILLTTER_INDEX_FILE, GILLTTER_PATH},
    index::index::{Index, IndexEntry, IndexType},
    objects::{
        ObjectDump, ObjectPump,
        blob::Blob,
        commit::Commit,
        tree::{Tree, TreeObject},
    },
    utils,
};

pub fn revert(commit_hash: &Path) -> anyhow::Result<()> {
    let path = Path::new(GILLTTER_PATH)
        .join(GILLTER_OBJECTS_DIR)
        .join(commit_hash);
    if !path.exists() {
        return Err(anyhow!("Such commit object does not exist"));
    }

    // Get a commit
    let commit = Commit::from_file(&path)?;

    // Parse index and remove all files (and dirs) that are there (beware of unstaged deleted?)
    let mut index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE))?;
    for i_entry in &index.indices {
        println!("File: {:?}", i_entry.filename);
        let filepath_as_str = i_entry.filename.to_string_lossy();
        let dirs: Vec<&str> = filepath_as_str.split('/').collect();

        let root_path = Path::new(dirs[0]);
        if root_path.is_file() || root_path.is_symlink() {
            if let Err(why) = std::fs::remove_file(dirs[0]) {
                eprintln!("Could not delete file {}, because: {}", dirs[0], why);
                return Err(anyhow!(why));
            }
        } else {
            // if let Err(why) = std::fs::remove_dir_all(root_path) { // FIXME: what if there is an untracked file?
            //     eprintln!("Could not delete dir {}, because: {}", dirs[0], why);
            //     return Err(anyhow!(why));
            // }
        }
    }
    index.remove_all();

    // Create new files from that commit
    let root_tree_hash = commit.get_tree_sha().ok_or(anyhow!(
        "How there is no tree in {} commit",
        commit_hash.to_str().unwrap()
    ))?;
    let root_tree_path = Path::new(GILLTTER_PATH)
        .join(GILLTER_OBJECTS_DIR)
        .join(root_tree_hash);
    let root_tree = Tree::from_file(&root_tree_path)?;

    let mut root_tree_files = Vec::new();
    let mut current_path = PathBuf::new();
    // load files from this root tree into vector
    traverse_head_tree(&mut root_tree_files, &mut current_path, &root_tree)?;

    // itrate files and create dirs and files
    for object_file in root_tree_files.iter() {
        let prefix = object_file.filename.parent();
        if prefix.is_some() {
            std::fs::create_dir_all(prefix.unwrap())?;
        }

        let blob = Blob::from_file(
            &Path::new(GILLTTER_PATH)
                .join(GILLTER_OBJECTS_DIR)
                .join(&object_file.sha1_hash),
        )?;
        let mut file = std::fs::File::create(&object_file.filename)?;
        file.write_all(&blob.get_data())?;

        // Add file to index
        let file_metadata = file.metadata()?;
        index.add(IndexEntry::new(
            file_metadata.ctime(),
            file_metadata.mtime(),
            file_metadata.size(),
            IndexType::RegularFile,
            object_file.filename.clone(),
            // object_file.sha1_hash.clone(),
            utils::generate_hash(&blob.convert_to_bytes().unwrap()), // blob convert to bytes cant fail
        ));

        file.flush()?;
    }

    // Save index file
    index.dump_to_file()?;

    // Update head TODO
    let mut head_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(&Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE))?;
    head_file.write_all(commit_hash.to_string_lossy().as_bytes())?;
    head_file.flush()?;
    Ok(())
}

// TODO: Maybe factor this out in util
fn traverse_head_tree(
    head_files: &mut Vec<IndexEntry>,
    current_path: &mut PathBuf,
    tree: &Tree,
) -> anyhow::Result<()> {
    let tree_objects = tree.get_objects();
    for (path, object) in &tree_objects {
        if let TreeObject::Blob(blob_hash) = object {
            head_files.push(IndexEntry::new(
                0,
                0,
                0,
                IndexType::RegularFile,
                current_path.join(path),
                blob_hash.to_string(),
            ));
        } else if let TreeObject::Tree(tree) = object {
            let tree = Tree::from_file(
                &Path::new(GILLTTER_PATH)
                    .join(GILLTER_OBJECTS_DIR)
                    .join(tree.get_hash().unwrap()),
            )
            .map_err(|why| anyhow!("Such tree does not exist: {}", why))?;
            traverse_head_tree(head_files, &mut current_path.join(path), &tree)?;
        }
    }
    Ok(())
}
