use crate::{
    base::{self, GILLTER_CONFIG_FILE, GILLTER_HEAD_FILE, GILLTTER_INDEX_FILE, GILLTTER_PATH},
    config::Config,
    objects::{
        commit::Commit, tree::{self, Tree, TreeObject}, ObjectDump, ObjectPump
    }, utils,
};
use anyhow::anyhow;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Clone)]
pub enum IndexType {
    RegularFile,
    SymbolicLink,
}

impl IndexType {
    pub fn to_bytes(&self) -> Vec<u8> {
        // TODO: Make it take a byte, not 6
        match self {
            Self::RegularFile => b"100644".to_vec(),
            Self::SymbolicLink => b"120000".to_vec(),
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"100644" => Some(Self::RegularFile),
            b"120000" => Some(Self::SymbolicLink),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub index_type: IndexType,
    pub ctime: i64,     // metadata last changed time
    pub mtime: i64, // file contents last changed time (used for comparing working tree with index, if differs, then file not staged), also used for comparing index file with last commit,
    pub file_size: u64, // in bytes
    pub filename: PathBuf,
    pub sha1_hash: String, // goes last, cuz it is fixed
}

impl IndexEntry {
    pub fn new(
        ctime: i64,
        mtime: i64,
        file_size: u64,
        index_type: IndexType,
        filename: PathBuf,
        sha1_hash: String,
    ) -> Self {
        Self {
            ctime,
            mtime,
            file_size,
            index_type,
            filename,
            sha1_hash,
        }
    }
    pub fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.index_type.to_bytes());
        bytes.extend_from_slice(
            format!(
                " {} {} {} {} {}\n",
                self.ctime,
                self.mtime,
                self.file_size,
                self.filename.to_str().expect("No filename"),
                self.sha1_hash
            )
            .as_bytes(),
        ); // TODO: Get rid of strings and make it compact and optimized (binary format)
        bytes
    }
}

pub struct Index {
    pub indices: Vec<IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: IndexEntry) {
        self.indices.push(entry);
    }

    pub fn remove(&mut self, filepath: &Path) -> bool {
        let pos = self.indices.iter().position(|val| val.filename.as_path() == Path::new(filepath));
        if let Some(pos) = pos {
            self.indices.remove(pos);
            return true
        }
        false
    }

    pub fn commit(&self, message: String) -> anyhow::Result<String> {
        assert!(!self.indices.is_empty());
        let mut base_tree = Tree::new();
        // Create a base tree, which all other objects are added to

        for entry in self.indices.iter() {
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
                .add_object_if_not_exists(paths.first().unwrap(), || { 
                    println!("Tree name: '{}'", paths.first().unwrap());
                    TreeObject::Tree(Tree::new()) 
            });
            
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
            .get("General", "Username")
            .ok_or(anyhow!("Username should be set in config file"))?;
        let email = config
            .get("General", "Email")
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
}

impl ObjectPump for Index {
    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut index = Index::new();

        let reader = BufReader::new(Cursor::new(data));
        for line in reader.lines() {
            let line = line?;
            let elements_vec: Vec<&str> = line.split(' ').collect();

            // Extract fields
            let idx_type = IndexType::from_bytes(elements_vec[0].as_bytes())
                .ok_or(anyhow!("There is no index type bytes"))?;
            let ctime = elements_vec[1].parse::<i64>()?;
            let mtime = elements_vec[2].parse::<i64>()?;
            let file_size = elements_vec[3].parse::<u64>()?;
            let filename = PathBuf::from_str(elements_vec[4])?;
            let sha1_hash = elements_vec[5].to_owned();

            let entry = IndexEntry::new(ctime, mtime, file_size, idx_type, filename, sha1_hash);
            index.add(entry);
        }

        Ok(index)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                let data = utils::decompress(&file_contents)?;
                return Self::from_raw_data(&data);
            }
            Err(why) => {
                return Err(anyhow!("Could not open '{}': {}", filepath.to_string_lossy(), why));
            }
        }
    }
}

impl ObjectDump for Index {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for index in &self.indices {
            bytes.extend_from_slice(&index.convert_to_bytes());
        }
        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let index_content = self.convert_to_bytes();
        // let compressed_content = utils::compress(&index_content)?;
        let compressed_content = index_content; // TODO: Remove after testing

        let path = Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE);
        let mut index_file = OpenOptions::new().write(true).open(&path)?; // No point in using 'create(true)', since files are there at this point

        index_file.write_all(&compressed_content)?;
        index_file.flush()?;
        Ok(path.to_string_lossy().to_string())
    }
}

pub fn add_one_in_index(filepath: &Path) -> anyhow::Result<()> {
    
    let mut index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE)).unwrap();
    // if index
    //     .indices
    //     .iter()
    //     .find(|element| element.filename == filepath)
    //     .is_some()
    // {
    //     return Err(anyhow!(
    //         "File '{}' already exists",
    //         filepath.to_string_lossy()
    //     ));
    // }

    // Notice: It is OK if the file already exists, means it is probably unstaged and user wanna stage it
    // If the file already exists, find it and update its metadata, or just delete and add again
    // First way:
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
