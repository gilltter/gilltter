use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{self, ObjectDump, ObjectPump},
    utils,
};

#[derive(Clone)]
pub enum ObjectType {
    Blob,
    Tree,
}

impl ObjectType {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Blob => "b".as_bytes().to_owned(),
            Self::Tree => "t".as_bytes().to_owned(),
        }
    }
    fn from_byte(byte: u8) -> Self {
        match byte {
            b'b' => Self::Blob,
            b't' => Self::Tree,
            _ => panic!("Wtf"),
        }
    }
}

#[derive(Clone)]
pub struct Object {
    pub obj_type: ObjectType,
    pub filepath: String,
    pub sha1_pointer: String,
}
impl Object {
    pub fn new(obj_type: ObjectType, filepath: String, sha1_pointer: String) -> Self {
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
        bytes.reserve(4 + 4 + self.objects.len() * (1 + 8 + 20)); // Approxim.

        bytes.extend_from_slice("tree ".as_bytes());

        // Count bytes
        let mut bytes_cnt = 0;
        for (_, value) in self.objects.iter() {
            bytes_cnt += 1; // 1 byte for file type
            bytes_cnt += 1; // Space after type
            bytes_cnt += value.filepath.len() + 1; // Filepath + 1 for null terminator
            bytes_cnt += 40; // Sha-1 in hex is 40 bytes
        }
        bytes.extend_from_slice(format!("{}\0", bytes_cnt).as_bytes());

        for (_, value) in self.objects.iter() {
            bytes.extend_from_slice(&value.obj_type.to_bytes());
            bytes.extend_from_slice(" ".as_bytes());
            bytes.extend_from_slice(
                format!("{}\0{}", value.filepath, value.sha1_pointer).as_bytes(),
            );
        }
        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let tree_content = self.convert_to_bytes();
        let filedata = utils::compress(&tree_content)?;
        let filename = utils::generate_filename(&tree_content);

        let path =
            String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR + "/" + filename.as_str();
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

        if &data[0..4] != "tree".as_bytes() {
            return Err(anyhow!("Object type is incorrect"));
        }

        let null_pos = data
            .iter()
            .position(|element| *element == *"\0".as_bytes().first().unwrap())
            .ok_or(anyhow!("No null terminator in file"))?;

        let size_tree_bytes = &data[5..null_pos];
        let _size_tree: u32 = String::from_utf8_lossy(size_tree_bytes)
            .trim()
            .parse::<u32>()?;

        let mut data = &data[null_pos + 1..];
        while !data.is_empty() {
            // Some cycle maybe
            let obj_type_byte = data[0];
            let obj_type = ObjectType::from_byte(obj_type_byte);

            data = &data[2..];
            let null_pos = data
                .iter()
                .position(|element| *element == *"\0".as_bytes().first().unwrap())
                .ok_or(anyhow!("No null terminator in file"))?;
            let filepath = String::from_utf8_lossy(&data[0..null_pos]).to_string();

            data = &data[null_pos + 1..]; // We skipped \0 now we at sha1
            let sha1_pointer = String::from_utf8_lossy(&data[0..40]).to_string(); // sha1 is 40 bytes

            tree.add_object(Object::new(obj_type, filepath, sha1_pointer));

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
