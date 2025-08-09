use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{self, ObjectDump, ObjectPump, SPACE_STR},
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
pub struct Object {
    pub obj_type: FileType,
    pub filepath: String,
    pub sha1_pointer: String,
}
impl Object {
    pub fn new(obj_type: FileType, filepath: String, sha1_pointer: String) -> Self {
        Self {
            obj_type,
            filepath,
            sha1_pointer,
        }
    }
}

pub struct Tree {
    objects: HashMap<String, Object>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }
    pub fn add_object(&mut self, object: Object) {
        self.objects.insert(object.sha1_pointer.clone(), object);
    }
    pub fn get_object(&self, sha1_pointer: String) -> Option<&Object> {
        self.objects.get(&sha1_pointer)
    }
    pub fn get_objects(&self) -> HashMap<String, Object> {
        self.objects.clone()
    }
}

impl ObjectDump for Tree {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(TREE_TYPE_STRING);
        bytes.extend_from_slice(SPACE_STR);

        // Count bytes
        let mut bytes_cnt = 0;
        for (_, value) in self.objects.iter() {
            bytes_cnt += 6; // 6 bytes for file type
            bytes_cnt += 1; // Space after type
            bytes_cnt += value.filepath.len() + 1; // Filepath + 1 for null terminator
            bytes_cnt += 40; // Sha-1 in hex is 40 bytes TODO: Would be good to get rid of magic numbers
        }
        bytes.extend_from_slice(format!("{}\0", bytes_cnt).as_bytes());

        for (_, value) in self.objects.iter() {
            bytes.extend_from_slice(&value.obj_type.to_bytes());
            bytes.extend_from_slice(" ".as_bytes());
            bytes.extend_from_slice(
                format!("{}\0{}", value.filepath, value.sha1_pointer).as_bytes(),
            );
        }

        println!("Tree: {}", String::from_utf8_lossy(&bytes));
        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let tree_content = self.convert_to_bytes();
        let filedata = utils::compress(&tree_content)?;
        let filename = utils::generate_filename(&tree_content);

        let path = String::from(GILLTTER_PATH)
            + utils::get_separator()
            + GILLTER_OBJECTS_DIR
            + utils::get_separator()
            + filename.as_str();
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;
        Ok(filename)
    }
}

impl ObjectPump for Tree {
    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut tree = Tree::new();
        let data = utils::decompress(data)?;

        let null_pos = data
            .iter()
            .position(|element| *element == "\0".as_bytes()[0])
            .ok_or(anyhow!("No null terminator in file"))?;
        let header = &data[0..null_pos];
        let content = &data[null_pos + 1..];

        if &header[0..TREE_TYPE_STRING.len()] != TREE_TYPE_STRING {
            return Err(anyhow!("Object type is incorrect"));
        }

        let size_tree_bytes = &header[TREE_TYPE_STRING.len() + 1..null_pos];
        let _size_tree: u32 = String::from_utf8_lossy(size_tree_bytes)
            .trim()
            .parse::<u32>()?;

        let mut data = content;
        while !data.is_empty() {
            // Some cycle maybe
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
            tree.add_object(Object::new(
                obj_type,
                filepath.clone(),
                sha1_pointer.clone(),
            ));

            data = &data[40..];
        }

        Ok(tree)
    }
    fn from_file(filepath: &str) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                return Tree::from_data(&file_contents);
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
    use crate::objects::blob::Blob;

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
                utils_sha1.clone(),
                Object::new(
                    objects::tree::FileType::RegularFile,
                    utils_filepath.to_string(),
                    utils_sha1.clone(),
                ),
            );

            let base_filepath = String::from("src/base.rs");
            let base_sha1 = gilltter_add(&base_filepath);
            index_mock.insert(
                base_sha1.clone(),
                Object::new(
                    objects::tree::FileType::RegularFile,
                    base_filepath.to_string(),
                    base_sha1.clone(),
                ),
            );

            // Build a tree
            let mut tree = Tree::new();

            for (_, value) in index_mock.into_iter() {
                tree.add_object(Object::new(
                    FileType::RegularFile,
                    value.filepath,
                    value.sha1_pointer,
                ));
            }

            let name = tree.dump_to_file().unwrap();

            Tree::from_file(&format!(".gilltter/objects/{}", name)).unwrap();
        }
    }
}
