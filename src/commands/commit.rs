use anyhow::anyhow;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    base::{GILLTER_CONFIG_FILE, GILLTER_HEAD_FILE, GILLTTER_PATH},
    config::{self, Config},
    index::index::Index,
    objects::{
        ObjectDump, ObjectPump,
        commit::Commit,
        tree::{self, Tree, TreeObject},
    },
};

pub fn commit(index: &Index, message: String) -> anyhow::Result<String> {
    assert!(!index.indices.is_empty());
    let mut base_tree = Tree::new();
    // Create a base tree, which all other objects are added to

    for entry in index.indices.iter() {
        let name = entry.filename.to_string_lossy().to_string();
        let paths: Vec<&str> = name.split('/').collect();

        // If it is only a file in a root dir, just add it...
        if paths.len() == 1 {
            base_tree.add_object(
                paths.first().unwrap(),
                TreeObject::Blob(entry.sha1_hash.clone()),
            );
            continue;
        }

        // Make sure src tree exists
        base_tree
            .add_object_if_not_exists(paths.first().unwrap(), || TreeObject::Tree(Tree::new()));

        // Current reference to tree so we can like traverse it
        let mut this_tree: &mut TreeObject =
            base_tree.get_object_mut(paths.first().unwrap()).unwrap();

        let dirs = &paths[1..paths.len() - 1];
        // Iterate until 1 before last, because the last one is a file
        for dir in dirs {
            let next = match this_tree {
                TreeObject::Tree(tree) => {
                    tree.add_object_if_not_exists(dir, || TreeObject::Tree(Tree::new()));
                    tree.get_object_mut(dir).unwrap()
                }
                _ => panic!("expected tree, found blob"),
            };
            this_tree = next;
        }

        if let TreeObject::Tree(tree) = this_tree {
            // println!("Dirs: {:?}, Tree: {:#?}, file: {}, exists: {}", dirs, tree.objects, paths.last().unwrap(), tree.object_exists(paths.last().unwrap()));
            tree.add_object(
                paths.last().unwrap(),
                TreeObject::Blob(entry.sha1_hash.clone()),
            );
        }
    }
    // Create commit object
    let config = Config::from_file(&Path::new(GILLTTER_PATH).join(GILLTER_CONFIG_FILE))?;
    let username = config
        .get(
            config::CONFIG_GENERAL_SECTION,
            config::CONFIG_USERNAME_FIELD,
        )
        .ok_or(anyhow!("Username should be set in config file"))?;
    let email = config
        .get(config::CONFIG_GENERAL_SECTION, config::CONFIG_EMAIL_FIELD)
        .ok_or(anyhow!("Email should be set in config file"))?;

    let base_tree_hash = base_tree.dump_to_file()?;

    let base_tree_objects = base_tree.get_objects();
    for object in base_tree_objects.values() {
        if let TreeObject::Tree(tree) = object {
            // Dump all subtrees
            tree::dump_tree_recursive(tree)?;
        }
    }

    // Open head file containing parent commit
    let mut head_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE))?;
    let mut file_bytes = Vec::new();
    head_file.read_to_end(&mut file_bytes)?;

    // Safe to use from_utf8_lossy, since no funny stuff should be in 'head'
    let parent_commit_hash = String::from_utf8_lossy(&file_bytes); // Empty, if first commit

    let mut commit = Commit::new();
    commit
        .set_tree_sha(base_tree_hash)
        .set_parent_commit_sha(if !parent_commit_hash.is_empty() {
            Some(parent_commit_hash.to_string())
        } else {
            None
        })
        .set_message(message)
        .set_username(username)
        .set_email(email);
    let commit_hash = commit.dump_to_file()?;

    // Write latest commit hash to the HEAD file
    head_file.seek(SeekFrom::Start(0))?; // Move cursor to the start of the file
    head_file.write_all(commit_hash.as_bytes())?;
    head_file.flush()?;

    Ok(commit_hash)
}
