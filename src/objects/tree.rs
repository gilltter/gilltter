use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump, SPACE_STR},
    utils,
};

#[derive(Clone)]
pub enum FileType {
    RegularFile,
    ExecutableFile,
    SymbolicLink,
    Directory,
}

pub const TREE_TYPE_STRING: &'static [u8] = b"tree";

impl FileType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::RegularFile => b"100644".to_vec(),
            Self::ExecutableFile => b"100755".to_vec(),
            Self::SymbolicLink => b"120000".to_vec(),
            Self::Directory => b"040000".to_vec(),
        }
    }
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"100644" => Some(Self::RegularFile),
            b"100755" => Some(Self::ExecutableFile),
            b"120000" => Some(Self::SymbolicLink),
            b"040000" => Some(Self::Directory),
            _ => None,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct Object {
    pub obj_type: FileType,
    pub filepath: String,
    pub sha1_pointer: String,
}
impl Object {
    #[allow(dead_code)]
    pub fn new(obj_type: FileType, filepath: String, sha1_pointer: String) -> Self {
        Self {
            obj_type,
            filepath,
            sha1_pointer,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TreeObject {
    Tree(Tree),
    Blob(String), // sha1-hash
}

#[derive(Clone, Debug)]
pub struct Tree {
    sha1_hash: String, // if it is a loaded object from tree parser, just set this field, kinda stupid, but will work for now
    pub objects: HashMap<String, TreeObject>, // path (local) -> Object
}

impl Tree {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            sha1_hash: String::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_hash(&self) -> anyhow::Result<String> {
        if !self.objects.is_empty() {
            return Err(anyhow!(
                "Can't get sha1-hash when tree is not used in load context"
            ));
        }
        Ok(self.sha1_hash.clone())
    }
    pub fn set_hash(&mut self, sha1_hash: &str) -> anyhow::Result<()> {
        if !self.objects.is_empty() {
            return Err(anyhow!(
                "Can't set sha1-hash when tree is not used in load context"
            ));
        }
        self.sha1_hash = sha1_hash.to_string();
        Ok(())
    }

    pub fn add_object(&mut self, filepath: &str, object: TreeObject) {
        self.objects.insert(filepath.to_owned(), object);
    }

    pub fn add_object_if_not_exists<F: FnOnce() -> TreeObject>(
        &mut self,
        filepath: &str,
        object_lambda: F,
    ) {
        self.objects
            .entry(filepath.to_string())
            .or_insert_with(object_lambda);
    }

    #[allow(dead_code)]
    pub fn get_object(&self, filepath: &str) -> Option<&TreeObject> {
        self.objects.get(filepath)
    }
    pub fn get_object_mut(&mut self, filepath: &str) -> Option<&mut TreeObject> {
        self.objects.get_mut(filepath)
    }

    pub fn get_objects(&self) -> HashMap<String, TreeObject> {
        self.objects.clone()
    }
    pub fn object_exists(&self, filepath: &str) -> bool {
        self.objects.contains_key(filepath)
    }
}

impl ObjectDump for Tree {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(TREE_TYPE_STRING);
        bytes.extend_from_slice(SPACE_STR);

        let mut bytes_count = 0;

        for (path, _) in self.objects.iter() {
            bytes_count += 6; // file type
            bytes_count += 1; // space
            bytes_count += path.len();
            bytes_count += 1; // \0 after path
            bytes_count += 40; // sha-1 size in hex
        }

        bytes.extend_from_slice(format!("{}\0", bytes_count).as_bytes());

        for (path, value) in self.objects.iter() {
            match value {
                TreeObject::Blob(_) => {
                    bytes.extend_from_slice(&FileType::RegularFile.to_bytes());
                }
                TreeObject::Tree(_) => {
                    bytes.extend_from_slice(&FileType::Directory.to_bytes());
                }
            }
            bytes.extend_from_slice(SPACE_STR);
            bytes.extend_from_slice(path.as_bytes());
            bytes.extend_from_slice(b"\0");
            match value {
                TreeObject::Blob(hash) => {
                    bytes.extend_from_slice(hash.as_bytes());
                }
                TreeObject::Tree(tree) => {
                    let tree_hash = utils::generate_hash(&tree.convert_to_bytes());
                    bytes.extend_from_slice(tree_hash.as_bytes());
                }
            }
        }

        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let tree_content = self.convert_to_bytes();
        // let filedata = utils::compress(&tree_content)?;
        let filedata = tree_content.clone(); // TODO: Remove this after testing
        let filename = utils::generate_hash(&tree_content);

        let path = Path::new(GILLTTER_PATH).join(GILLTER_OBJECTS_DIR).join(filename.as_str());
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;
        Ok(filename)
    }
}

pub fn dump_tree_recursive(tree: &Tree) -> anyhow::Result<()> {
    tree.dump_to_file()?;

    let base_tree_objects = tree.get_objects();
    for object in base_tree_objects.values() {
        if let TreeObject::Tree(tree) = object {
            // Dump all subtrees
            tree.dump_to_file()?;
            dump_tree_recursive(tree)?;
        }
    }
    Ok(())
}

impl ObjectPump for Tree {
    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut tree = Tree::new();
        let null_pos = data
            .iter()
            .position(|element| *element == "\0".as_bytes()[0])
            .ok_or(anyhow!("No null terminator in file"))?;
        let header = &data[0..null_pos];
        let content = &data[null_pos + 1..];

        if &header[0..TREE_TYPE_STRING.len()] != TREE_TYPE_STRING {
            return Err(anyhow!("Object type is incorrect"));
        }

        // TODO: Incorrectly counted
        let size_tree_bytes = &header[TREE_TYPE_STRING.len() + 1..null_pos];
        let _size_tree: u32 = String::from_utf8_lossy(size_tree_bytes)
            .trim()
            .parse::<u32>()?;

        // may wanna check if data == size tree bytes
        let mut data = content;
        while !data.is_empty() {
            let obj_type_bytes = &data[0..6];
            let obj_type =
                FileType::from_bytes(obj_type_bytes).ok_or(anyhow!("Weird file type"))?;

            if data.len() <= 2 {
                return Err(anyhow!("Format error"));
            }
            data = &data[2..];
            let null_pos = data
                .iter()
                .position(|element| *element == *"\0".as_bytes().first().unwrap())
                .ok_or(anyhow!("No null terminator in file"))?;

            let filepath = String::from_utf8_lossy(&data[0..null_pos]).to_string();

            data = &data[null_pos + 1..]; // We skipped \0 now we at sha1
            if data.len() < 40 {
                return Err(anyhow!("Tree is weirdly formatted"));
            }

            let sha1_pointer = String::from_utf8_lossy(&data[0..40]).to_string(); // sha1 is 40 bytes
            match obj_type {
                FileType::RegularFile | FileType::ExecutableFile | FileType::SymbolicLink => {
                    tree.add_object(&filepath, TreeObject::Blob(sha1_pointer));
                }
                FileType::Directory => {
                    let mut to_be_loaded_tree = Tree::new();
                    to_be_loaded_tree.set_hash(&sha1_pointer).unwrap(); // Can't possibly fucking panic
                    tree.add_object(&filepath, TreeObject::Tree(to_be_loaded_tree)); // TODO: How to load this tree
                }
            }

            data = &data[40..];
        }

        Ok(tree)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                // let data = utils::decompress(&file_contents)?; // TODO: corrupt deflate stream if empty 
                let data = file_contents;
                return Tree::from_raw_data(&data);
            }
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open the file"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::objects::{self, blob::Blob};

    use super::*;

    fn gilltter_add(filepath: &str) -> String {
        let mut file = File::open(filepath).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let mut blob = Blob::new();
        blob.set_data(&contents);

        let filename = blob.dump_to_file().unwrap();
        filename
    }

    #[test]
    fn create_tree_dump_then_load() {
        let mut index_mock: HashMap<String, Object> = HashMap::new();

        // Imagine git add
        {
            let utils_filepath = String::from("src/utils.rs");
            let utils_sha1 = gilltter_add(&utils_filepath);
            index_mock.insert(
                utils_filepath.clone(),
                Object::new(
                    objects::tree::FileType::RegularFile,
                    utils_filepath.to_string(),
                    utils_sha1.clone(),
                ),
            );

            let base_filepath = String::from("src/base.rs");
            let base_sha1 = gilltter_add(&base_filepath);
            index_mock.insert(
                base_filepath.clone(),
                Object::new(
                    objects::tree::FileType::RegularFile,
                    base_filepath.to_string(),
                    base_sha1.clone(),
                ),
            );

            // Build a tree
            let mut tree = Tree::new();

            for (position, value) in index_mock.into_iter() {
                tree.add_object(&position, TreeObject::Blob(value.sha1_pointer));
            }

            let name = tree.dump_to_file().unwrap();

            Tree::from_file(Path::new(&format!(".gilltter/objects/{}", name))).unwrap();
        }
    }
}
