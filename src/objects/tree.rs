use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
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

pub const TREE_TYPE_STRING: &'static str = "tree";

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
    pub objects: BTreeMap<String, TreeObject>, // path (local) -> Object
}

impl Tree {
    pub fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
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

    pub fn get_objects(&self) -> BTreeMap<String, TreeObject> {
        self.objects.clone()
    }
    pub fn object_exists(&self, filepath: &str) -> bool {
        self.objects.contains_key(filepath)
    }
}

impl ObjectDump for Tree {
    fn convert_to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        if self.objects.is_empty() {
            return Err(anyhow!("Can't convert an empty tree to bytes"));
        }

        let mut tree_bytes = Vec::new();
        write!(&mut tree_bytes, "{} ", TREE_TYPE_STRING)?;

        let mut bytes = Vec::new();
        for (path, value) in &self.objects {
            let mut type_bytes = Vec::new();
            match value {
                TreeObject::Blob(_) => {
                    type_bytes = FileType::RegularFile.to_bytes();
                }
                TreeObject::Tree(_) => {
                    type_bytes = FileType::Directory.to_bytes();
                }
            }

            let mut obj_hash = String::new();
            match value {
                TreeObject::Blob(hash) => {
                    obj_hash = hash.to_string();
                }
                TreeObject::Tree(tree) => {
                    let tree_hash = utils::generate_hash(&tree.convert_to_bytes()?);
                    obj_hash = tree_hash;
                }
            }

            write!(
                &mut bytes,
                "{} {} {}\n",
                std::str::from_utf8(&type_bytes)?,
                path,
                obj_hash
            )?;
        }

        write!(&mut tree_bytes, "{}\n", bytes.len())?;
        tree_bytes.extend(bytes.iter());

        Ok(tree_bytes)
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let tree_content = self.convert_to_bytes()?;
        // let filedata = utils::compress(&tree_content)?;
        let filedata = tree_content.clone(); // TODO: Remove this after testing
        let filename = utils::generate_hash(&tree_content);

        let path = Path::new(GILLTTER_PATH)
            .join(GILLTER_OBJECTS_DIR)
            .join(filename.as_str());
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

        let mut reader = BufReader::new(data);

        let mut tree_info = String::new();
        reader.read_line(&mut tree_info)?;
        let tree_info_parts: Vec<&str> = tree_info.split(' ').collect();
        if tree_info_parts.len() != 2 {
            return Err(anyhow!("Expected 2 values in tree header"));
        }
        let object_type = tree_info_parts.first().unwrap(); // safe
        if *object_type != "tree" {
            return Err(anyhow!("Could not parse obj header"));
        }

        let tree_bytes_size = tree_info_parts.last().unwrap().trim().parse::<usize>()?;
        let mut tree_bytes_cnt: usize = 0;
        for line in reader.lines() {
            if let Err(why) = line {
                return Err(anyhow!("Could not read line: {}", why));
            }
            let line = line?;
            tree_bytes_cnt += line.len() + 1; // for new line

            let line_parts = line.split(' ').collect::<Vec<&str>>();
            if line_parts.len() != 3 {
                return Err(anyhow!("Format error in object lines in tree"));
            }
            let obj_type_str = line_parts[0];
            let obj_path_str = line_parts[1];
            let obj_hash_str = line_parts[2];

            let obj_type = FileType::from_bytes(obj_type_str.as_bytes())
                .ok_or(anyhow!("Invalid object type"))?;
            match obj_type {
                FileType::RegularFile | FileType::ExecutableFile | FileType::SymbolicLink => {
                    tree.add_object(obj_path_str, TreeObject::Blob(obj_hash_str.to_string()));
                }
                FileType::Directory => {
                    let mut to_be_loaded_tree = Tree::new();
                    to_be_loaded_tree.set_hash(obj_hash_str)?; // Can't possibly fucking panic
                    tree.add_object(obj_path_str, TreeObject::Tree(to_be_loaded_tree)); // TODO: How to load this tree properly
                }
            }
        }

        if tree_bytes_cnt != tree_bytes_size {
            return Err(anyhow!("Tree size is not correct"));
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
    use super::*;

    #[test]
    #[should_panic]
    fn dump_empty_tree() {
        let tree = Tree::new();
        tree.convert_to_bytes().unwrap();
    }

    #[test]
    fn dump_nonempty_tree() {
        let mut tree = Tree::new();
        tree.add_object(
            "ddd.txt",
            TreeObject::Blob(String::from_utf8_lossy(&[87u8; 40]).to_string()),
        );
        tree.convert_to_bytes().unwrap();
    }

    #[test]
    fn add_if_not_exists() {
        let mut tree = Tree::new();
        let obj = TreeObject::Blob(String::from_utf8_lossy(&[87u8; 40]).to_string());
        tree.add_object("ddd.txt", obj);

        let obj = TreeObject::Blob(String::from_utf8_lossy(&[89u8; 40]).to_string());
        tree.add_object_if_not_exists("ddd.txt", || obj);

        let obj = tree.get_object("ddd.txt").unwrap();
        if let TreeObject::Blob(data) = obj {
            let a = String::from_utf8_lossy(&[87u8; 40]).to_string();
            let b = data.to_string();
            assert!(a == b)
        }
    }

    #[test]
    fn dump_and_pump() {
        let mut tree = Tree::new();
        let obj = TreeObject::Blob(String::from_utf8_lossy(&[87u8; 40]).to_string());
        tree.add_object("ddd.txt", obj);

        let obj = TreeObject::Blob(String::from_utf8_lossy(&[89u8; 40]).to_string());
        tree.add_object("ttt.txt", obj);

        let obj = TreeObject::Blob(String::from_utf8_lossy(&[84u8; 40]).to_string());
        tree.add_object("zz.txt", obj);

        let tree_bytes = tree.convert_to_bytes().unwrap();
        let hash_dumped = utils::generate_hash(&tree_bytes);

        let tree = Tree::from_raw_data(&tree_bytes).unwrap();
        let tree_bytes = tree.convert_to_bytes().unwrap();
        let hash_pumped = utils::generate_hash(&tree_bytes);
        assert_eq!(hash_dumped, hash_pumped)
    }
}
