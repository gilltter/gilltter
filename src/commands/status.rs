use anyhow::anyhow;
use colored::Colorize;
use std::{
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use crate::{
    base::{GILLTER_HEAD_FILE, GILLTER_OBJECTS_DIR, GILLTTER_INDEX_FILE, GILLTTER_PATH},
    index::index::{Index, IndexEntry, IndexType},
    objects::{
        ObjectPump,
        commit::Commit,
        tree::{Tree, TreeObject},
    },
    utils,
};

fn traverse_dirs_impl(
    entries: &mut Vec<IndexEntry>,
    path: std::path::PathBuf,
) -> anyhow::Result<()> {
    let dir = path;
    let root_path = std::env::current_dir()?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_file() {
            let meta = std::fs::metadata(entry.path())?;

            let content = utils::get_file_contents_as_blob(&entry.path())?;
            let sha1 = utils::generate_hash(&content);

            // TODO: it is unix only
            entries.push(IndexEntry::new(
                meta.ctime(),
                meta.mtime(),
                meta.size(),
                crate::index::index::IndexType::RegularFile,
                entry.path().strip_prefix(&root_path)?.to_path_buf(),
                sha1,
            ));
        } else if filetype.is_dir() {
            // TODO: Get rid of this shit
            if entry.file_name() == "target"
                || entry.file_name() == ".gilltter"
                || entry.file_name() == ".git"
            {
                continue;
            }
            traverse_dirs_impl(entries, entry.path())?;
        }
    }
    Ok(())
}

pub fn traverse_dirs(path: std::path::PathBuf) -> anyhow::Result<Vec<IndexEntry>> {
    let mut entries: Vec<IndexEntry> = Vec::new();
    if let Err(why) = traverse_dirs_impl(&mut entries, path) {
        return Err(why);
    }
    Ok(entries)
}

fn get_untracked(work_tree_files: &Vec<IndexEntry>, index: &Index) -> Vec<IndexEntry> {
    let mut untracked_files: Vec<IndexEntry> = Vec::new();

    for worktree_entry in work_tree_files.iter() {
        if !index
            .indices
            .iter()
            .any(|val| worktree_entry.filename == val.filename)
        {
            untracked_files.push(worktree_entry.clone());
        }
    }
    untracked_files
}

fn get_unstaged(work_tree_files: &Vec<IndexEntry>, index: &Index) -> Vec<IndexEntry> {
    let mut unstaged_files: Vec<IndexEntry> = Vec::new();
    for index_entry in index.indices.iter() {
        let worktree_entry = work_tree_files
            .iter()
            .find(|val| val.filename == index_entry.filename);
        if let Some(worktree_entry) = worktree_entry {
            if worktree_entry.sha1_hash != index_entry.sha1_hash {
                unstaged_files.push(worktree_entry.clone());
            }
        }
    }
    unstaged_files
}

fn get_deleted_files(
    work_tree_files: &Vec<IndexEntry>,
    head_files: &Vec<IndexEntry>,
    index: &Index,
) -> Vec<IndexEntry> {
    let mut deleted_files: Vec<IndexEntry> = Vec::new();

    // если файл удален, но не staged, то добавляем в deleted_files Entry с метаданными = изменение не закомичено
    // если файл удален, и staged (gilltter add <deleted-file>), то добавляем в deleted_files Entry без метаданных = изменение закомичено
    for index_entry in index.indices.iter() {
        // Find unstaged deleted files
        let worktree_entry = work_tree_files
            .iter()
            .find(|val| val.filename == index_entry.filename);
        if worktree_entry.is_none() {
            deleted_files.push(index_entry.clone());
        }
    }

    for head_entry in head_files.iter() {
        let index_entry = index
            .indices
            .iter()
            .find(|val| val.filename == head_entry.filename);
        if index_entry.is_none() {
            println!("HEREJKLJKLS:JKL:FSLJK:DFSDJKL:FDJKL:DFJKL:");
            // File is deleted and staged
            deleted_files.push(head_entry.clone());
        }
    }

    deleted_files
}

fn traverse_head_get_files() -> anyhow::Result<Vec<IndexEntry>> {
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
        let commit_sha = String::from_utf8_lossy(&head_commit_bytes)
            .trim()
            .to_string();

        let commit = Commit::from_file(
            &Path::new(GILLTTER_PATH)
                .join(GILLTER_OBJECTS_DIR)
                .join(commit_sha.to_string()),
        )
        .map_err(|why| anyhow!("Commit file does not exist: {}", why))?;

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
        .map_err(|why| anyhow!("Such tree does not exist: {}", why))?;
        traverse_head_tree(&mut head_files, &mut current_path, &base_tree)?; // traverse from head untiil the end
    }
    Ok(head_files)
}

fn get_staged_and_commited(
    head_files: &Vec<IndexEntry>,
    work_tree_files: &Vec<IndexEntry>,
    index: &Index,
) -> anyhow::Result<(Vec<IndexEntry>, Vec<IndexEntry>)> {
    let mut staged_files: Vec<IndexEntry> = Vec::new();
    let mut commited_files: Vec<IndexEntry> = Vec::new();

    if !head_files.is_empty() {
        for index_entry in index.indices.iter() {
            let worktree_entry = work_tree_files
                .iter()
                .find(|val| val.filename == index_entry.filename);
            if let Some(worktree_entry) = worktree_entry {
                let head_file_opt = head_files
                    .iter()
                    .find(|el| el.filename == worktree_entry.filename);

                let hash_comparison = worktree_entry.sha1_hash == index_entry.sha1_hash;
                if hash_comparison {
                    // if index == working tree
                    if let Some(head_file) = head_file_opt {
                        if head_file.sha1_hash != index_entry.sha1_hash {
                            // Index matches Worktree but differs from HEAD => staged
                            staged_files.push(worktree_entry.clone());
                        } else {
                            // All three match => committed
                            commited_files.push(worktree_entry.clone());
                        }
                    } else {
                        // File not in HEAD but staged
                        staged_files.push(worktree_entry.clone());
                    }
                }
            }
        }
    } else {
        for index_entry in index.indices.iter() {
            let worktree_entry = work_tree_files
                .iter()
                .find(|val| val.filename == index_entry.filename);
            if let Some(worktree_entry) = worktree_entry {
                if worktree_entry.sha1_hash == index_entry.sha1_hash {
                    staged_files.push(worktree_entry.clone());
                }
            }
        }
    }
    Ok((staged_files, commited_files))
}

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

// TODO: Better error handling, more transparent errors, check edge cases like when one of the tree doesn't exist
// idea n1: use HashMaps instead of Vec to optimize lookup
// idead n2: Store references instead of clones()
pub(crate) fn gilltter_status() -> anyhow::Result<()> {
    // Parse index
    let index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE))?;

    // Parse working tree
    let dir = std::env::current_dir()?;
    // FInd all work tree files and put into work_tree_files
    let work_tree_files = traverse_dirs(dir)?;

    // Сначала найдем untracked файлы => untracked файл это значит он есть в ворк три но нет в index
    // let untracked_files = get_untracked(&work_tree_files, &index);

    // Теперь с оставшимися нужно сделать unstaged
    let unstaged_files = get_unstaged(&work_tree_files, &index);

    // Get head files
    let head_files = traverse_head_get_files()?;

    let (staged_files, commited_files) =
        get_staged_and_commited(&head_files, &work_tree_files, &index)?;

    let deleted_files = get_deleted_files(&work_tree_files, &head_files, &index);

    println!("{}", "==== Deleted Files ====".red().bold());
    for entry in &deleted_files {
        if entry.ctime > 0 {
            println!("  {} {:?}", "deleted:".red(), entry.filename);
        } else {
            println!("  {} {:?}", "deleted:".green(), entry.filename);
        }
    }

    println!("{}", "==== Unstaged Files ====".red().bold());
    for entry in &unstaged_files {
        println!("  {} {:?}", "modified:".yellow(), entry.filename);
    }
    println!();

    println!("{}", "==== Staged Files ====".green().bold());
    for entry in &staged_files {
        println!("  {} {:?}", "staged:".green(), entry.filename);
    }
    println!();

    // println!("{}", "==== Untracked Files ====".magenta().bold());
    // for entry in &untracked_files {
    //     println!("  {} {:?}", "untracked:".magenta(), entry.filename);
    // }
    // println!();

    Ok(())
}
