use anyhow::anyhow;
use colored::Colorize;
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::index::index::{Index, IndexEntry, IndexType};
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::tree::{Tree, TreeObject};
use crate::objects::{ObjectDump, ObjectPump};
use crate::utils;

pub const GILLTTER_PATH: &'static str = ".gilltter";

pub const GILLTER_OBJECTS_DIR: &'static str = "objects";
pub const GILLTER_HEAD_FILE: &'static str = "head";
pub const GILLTER_STATE_FILE: &'static str = "state"; // A.k.a git INDEX file
pub const GILLTER_BRANCHES_DIR: &'static str = "branches";
pub const GILLTER_CONFIG_FILE: &'static str = "config";
pub const GILLTTER_INDEX_FILE: &'static str = "index";

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

fn get_file_contents(path: &Path) -> anyhow::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut blob = Blob::new();
    blob.set_data(&bytes);

    let bytes = blob.convert_to_bytes();
    Ok(bytes)
}

fn traverse_dirs(entries: &mut Vec<IndexEntry>, path: std::path::PathBuf) -> anyhow::Result<()> {
    let dir = path;
    let root_path = std::env::current_dir()?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_file() {
            let meta = std::fs::metadata(entry.path())?;

            let content = get_file_contents(&entry.path())?;
            let sha1 = utils::generate_hash(&content);

            entries.push(IndexEntry::new(
                meta.ctime(),
                meta.mtime(),
                meta.size(),
                crate::index::index::IndexType::RegularFile,
                entry.path().strip_prefix(&root_path)?.to_path_buf(),
                sha1,
            ));
        } else if filetype.is_dir() {
            if entry.file_name() == "target"
                || entry.file_name() == ".gilltter"
                || entry.file_name() == ".git"
            {
                continue;
            }
            traverse_dirs(entries, entry.path())?;
        }
    }
    Ok(())
}

/*
- Спарсить индекс
- Спарсить воркинг три
- Сравнить => (получатся unstaged, untracked файлы)
- Спарсить хед
- Сравнить с индексом (work tree) => Получатся staged (index == work tree != head), commited (index == work tree == head)
*/

pub(crate) fn gilltter_status() -> anyhow::Result<()> {
    // Parse index
    let index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE))?;

    // Parse working tree
    let mut work_tree_files = Vec::<IndexEntry>::new();
    let dir = std::env::current_dir()?;
    traverse_dirs(&mut work_tree_files, dir)?;
    // println!("Worktree: {:#?}", work_tree_files);
    // Сначала найдем untracked файлы => untracked файл это значит он есть в ворк три но нет в index
    let mut untracked_files: Vec<IndexEntry> = Vec::new();

    let mut worktree_to_delete = Vec::new();
    for (idx, worktree_entry) in work_tree_files.iter().enumerate() {
        if !index
            .indices
            .iter()
            .any(|val| worktree_entry.filename == val.filename)
        {
            untracked_files.push(worktree_entry.clone());
            worktree_to_delete.push(idx);
        }
    }
    for idx in worktree_to_delete.iter().rev() {
        work_tree_files.remove(*idx);
    }

    println!("========");
    for entry in &untracked_files {
        println!("{}: {:?}", "Untracked".red(), entry.filename);
    }
    println!("========");

    // Untracked готовы
    // Теперь с оставшимися нужно сделать unstaged
    let mut unstaged_files: Vec<IndexEntry> = Vec::new();
    for (_, index_entry) in index.indices.iter().enumerate() {
        let worktree_entry = work_tree_files
            .iter()
            .find(|val| val.filename == index_entry.filename);
        if let Some(worktree_entry) = worktree_entry {
            if worktree_entry.sha1_hash != index_entry.sha1_hash {
                unstaged_files.push(worktree_entry.clone());
            } else {
            }
        }
    }

    println!("========");
    for entry in &unstaged_files {
        println!("{}: {:?}", "Unstaged:".yellow(), entry.filename);
    }
    println!("========");

    let mut head_file = match std::fs::File::open(Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE))
    {
        Ok(file) => file,
        Err(why) => {
            // fatal error, head file should always be there
            eprintln!("Could not open the head file, because: {}", why);
            return Err(anyhow!(why));
        }
    };

    let mut head_commit_bytes = Vec::new();
    head_file.read_to_end(&mut head_commit_bytes)?;

    let mut head_files: Vec<IndexEntry> = Vec::new();
    if !head_commit_bytes.is_empty() {
        // Нужно пройтись по хеду и добавить файлы в head_file, путь ставить относительно root_path
        let commit_sha = String::from_utf8_lossy(&head_commit_bytes);
        let commit = Commit::from_file(
            &Path::new(GILLTTER_PATH)
                .join(GILLTER_OBJECTS_DIR)
                .join(commit_sha.to_string()),
        )?;

        // Get a tree
        // println!("commit: {}", String::from_utf8_lossy(&commit.convert_to_bytes()));
        let tree_hash = commit
            .get_tree_sha()
            .expect("Should've had a tree hash inside");

        // Then traverse trees and add them to trees
        let mut current_path = PathBuf::new(); // Track current path, empty = base tree, commit always points to a base tree
        let base_tree = Tree::from_file(
            &Path::new(GILLTTER_PATH)
                .join(GILLTER_OBJECTS_DIR)
                .join(&tree_hash),
        )
        .expect("How the fuck there is no such tree");
        traverse_head_tree(&mut head_files, &mut current_path, &base_tree); // traverse from head untiil the end
    }

    let mut staged_files: Vec<IndexEntry> = Vec::new();
    let mut commited_files: Vec<IndexEntry> = Vec::new();

    if !head_files.is_empty() {
        println!("Here");
        // One way of figuring out staged, commited
        // worktree_to_delete.clear();
        //
        for (_, index_entry) in index.indices.iter().enumerate() {
            let worktree_entry = work_tree_files
                .iter()
                .find(|val| val.filename == index_entry.filename);
            if let Some(worktree_entry) = worktree_entry {
                if worktree_entry.sha1_hash == index_entry.sha1_hash {
                    staged_files.push(worktree_entry.clone());
                }
            }
        }

        let mut staged_to_remove_idxs = Vec::new();
        for (i, file) in staged_files.iter().enumerate() {
            let head_file_opt = head_files.iter().find(|el| el.filename == file.filename);
            if let Some(head_file) = head_file_opt {
                println!("{} {}", head_file.sha1_hash, file.sha1_hash);
                if head_file.sha1_hash == file.sha1_hash {
                    commited_files.push(head_file.clone());
                    staged_to_remove_idxs.push(i);
                }
            }
        }

        // TODO: use retain instead of this shit
        for idx in staged_to_remove_idxs.iter().rev() {
            staged_files.remove(*idx);
        }
    } else {
        for (_, index_entry) in index.indices.iter().enumerate() {
            let worktree_entry = work_tree_files
                .iter()
                .find(|val| val.filename == index_entry.filename);
            if let Some(worktree_entry) = worktree_entry {
                if worktree_entry.sha1_hash == index_entry.sha1_hash {
                    staged_files.push(worktree_entry.clone());
                }
            }
        }
        // Other way of doing the same thing
        // head doesnt exist
        // ir index_file == worktree and head doesn't exist, all files are staged
        // if index_file != worktree => unstaged
        // if work tree fle doesnt exist in index fie => untracked
    }

    println!("========");
    for entry in &staged_files {
        println!("{}: {:?}", "Staged:".green(), entry.filename);
    }
    println!("=========");

    // println!("========");
    // for entry in &commited_files {
    //     println!("{}: {:?}", "Commited:".blue(), entry.filename);
    // }
    // println!("=========");

    Ok(())
}

fn traverse_head_tree(head_files: &mut Vec<IndexEntry>, current_path: &mut PathBuf, tree: &Tree) {
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
            .expect("How the fuck there is no such tree");
            traverse_head_tree(head_files, &mut current_path.join(path), &tree);
        }
    }
}
