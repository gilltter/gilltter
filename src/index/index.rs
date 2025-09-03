use crate::{
    base::{GILLTER_CONFIG_FILE, GILLTER_HEAD_FILE, GILLTER_OBJECTS_DIR, GILLTTER_INDEX_FILE, GILLTTER_PATH}, config::Config, gilltter_add, objects::{
        blob::Blob, commit::Commit, tree::{self, FileType, Object, Tree, TreeObject}, ObjectDump, ObjectPump
    }, utils
};
use anyhow::anyhow;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    str::FromStr, time::{SystemTime, UNIX_EPOCH},
};

pub enum IndexType {
    RegularFile,
    SymbolicLink,
}

impl IndexType {
    pub fn to_bytes(&self) -> Vec<u8> {
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

    pub fn commit(&self, message: String) -> anyhow::Result<String> {
        assert!(!self.indices.is_empty());

        let mut base_tree = Tree::new();

        for entry in self.indices.iter() {
            let name = entry.filename.to_string_lossy().to_string();
            let paths: Vec<&str> = name.split('/').collect();

            if paths.len() == 1 {
                base_tree.add_object(
                    paths.first().unwrap(),
                    TreeObject::Blob(entry.sha1_hash.clone()),
                );
                continue;
            }

            if !base_tree.object_exists(paths.first().unwrap()) {
                base_tree.add_object(paths.first().unwrap(), TreeObject::Tree(Tree::new()));
            }

            let mut this_tree: &mut TreeObject =
                base_tree.get_object_mut(paths.first().unwrap()).unwrap();

            let dirs = &paths[1..paths.len() - 1];
            for dir in dirs {
                let next = match this_tree {
                    TreeObject::Tree(tree) => {
                        if !tree.object_exists(dir) {
                            tree.add_object(dir, TreeObject::Tree(Tree::new()));
                        }
                        tree.get_object_mut(dir).unwrap()
                    }
                    _ => panic!("expected tree, found blob"),
                };
                this_tree = next;
            }

            if let TreeObject::Tree(tree) = this_tree {
                tree.add_object(
                    paths.last().unwrap(),
                    TreeObject::Blob(entry.sha1_hash.clone()),
                );
            }
        }

        // Create commit object
        let config = Config::from_file(&Path::new(GILLTTER_PATH).join(GILLTER_CONFIG_FILE)).expect("Config must be set up");
        let username = config.get("General", "Username").ok_or(anyhow!("Username should be set in config file"))?;
        let email = config.get("General", "Email").ok_or(anyhow!("Email should be set in config file"))?;


        let base_tree_hash = base_tree.dump_to_file()?;

      
        let base_tree_objects = base_tree.get_objects();
        for object in base_tree_objects.values() {
            if let TreeObject::Tree(tree) = object { // Dump all subtrees
                tree::dump_tree_recursive(tree)?;
            }
        }

        // open head file containing parent commit
        let mut head_file = OpenOptions::new().read(true).write(true).open(&Path::new(GILLTTER_PATH).join(GILLTER_HEAD_FILE))?;
        let mut file_bytes = Vec::new();
        head_file.read_to_end(&mut file_bytes)?;
        // Safe to use from_utf8_lossy, since no funny stuff should be in 'head'
        let parent_commit = String::from_utf8_lossy(&file_bytes); // Empty, if first commit 

        let mut commit = Commit::new();
        commit
            .set_tree_sha(base_tree_hash)
            .set_parent_commit_sha(if !parent_commit.is_empty() { Some(parent_commit.to_string()) } else { None }) 
            .set_message(message)
            .set_username(username)
            .set_email(email);
        let commit_hash = commit.dump_to_file()?;
        println!("commit dmped");

        // Write latest commit hash to the HEAD file
        head_file.seek(SeekFrom::Start(0))?; // Move cursor to the start of the file
        head_file.write_all(commit_hash.as_bytes())?;
        head_file.flush()?;

        Ok(commit_hash)
    }
}


impl ObjectPump for Index {
    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut index = Index::new();

        let reader = BufReader::new(Cursor::new(data));
        for line in reader.lines() {
            let line = line?;
            let elements_vec: Vec<&str> = line.split(' ').collect();

            let idx_type = IndexType::from_bytes(elements_vec[0].as_bytes())
                .expect("Index type should be here");
            let ctime = elements_vec[1].parse::<i64>()?;
            let mtime = elements_vec[2].parse::<i64>()?;
            let file_size = elements_vec[3].parse::<u64>()?;

            let filename = PathBuf::from_str(elements_vec[4])?;

            let sha1_hash = elements_vec[5].to_owned();

            index.indices.push(IndexEntry::new(
                ctime, mtime, file_size, idx_type, filename, sha1_hash,
            ));
        }

        Ok(index)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                return Self::from_data(&file_contents);
            }
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open the file"));
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

        let path = Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE);
        let mut index_file = OpenOptions::new().write(true).create(true).open(&path)?;

        index_file.write_all(&index_content)?;
        index_file.flush()?;
        Ok(path.to_string_lossy().to_string())
    }
}

pub fn add_one_in_index(filepath: &Path) -> anyhow::Result<()> {
    let mut index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE))?;
    if index
        .indices
        .iter()
        .find(|element| element.filename == filepath)
        .is_some()
    {
        eprintln!("Such file already exists, fuck you!");
        return Err(anyhow!("Such file already exists"));
    }

    let file_sha1 = gilltter_add(filepath.to_str().unwrap()).unwrap();
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
        PathBuf::from_str(&filepath.to_string_lossy())?,
        file_sha1,
    );
    index.indices.push(entry);

    index.dump_to_file()?;
    Ok(())
}

/*
{
    // Split dirs and path
    let filename_as_string = filename.to_string_lossy().to_string();
    let mut dirs: Vec<&str> = filename_as_string.split(utils::get_separator()).collect();
    let filename = dirs.last().expect("Cant be empty").to_string();
    let _ = dirs.pop();

    // Open file and set blob's contents to it
    let mut file = File::open(&filename).expect("file must exist");
    let mut file_bytes = Vec::new();
    file.read_to_end(&mut file_bytes)?;
    file.flush()?;

    // Fill blob with data
    let mut file_blob = Blob::new();
    file_blob.set_data(&file_bytes);
    let file_sha1 = file_blob.dump_to_file()?;

    // Create necessary trees
    let mut trees = Vec::new();
    for dir in dirs.iter() {
        trees.push(Tree::new());
    }
    // Add blob to the last tree
    trees.last_mut().unwrap().add_object(Object::new(FileType::RegularFile, filename, file_sha1));

    for i in (0..trees.len() - 1).rev() {
        // Dump next tree
        let tree_sha1 = trees[i + 1].dump_to_file()?;
        // Add it to current tree
        trees[i].add_object(Object::new(FileType::Directory, dirs[i + 1].to_owned(), tree_sha1));
    }

    // At this point
}

*/
