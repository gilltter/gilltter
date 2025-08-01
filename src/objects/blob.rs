use std::{
    fs::{self, File},
    io::{Read, Write},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump},
    utils,
};

pub struct Blob {
    content: Vec<u8>,
}

impl Blob {
    pub fn new() -> Self {
        Self { content: vec![] }
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.content.clone()
    }

    pub fn _append_data(&mut self, data: &[u8]) {
        self.content.extend_from_slice(data);
    }
    pub fn set_data(&mut self, data: &[u8]) {
        self.content = data.to_owned();
    }
}

impl ObjectPump for Blob {
    fn from_file(filepath: &str) -> anyhow::Result<Self> {
        match fs::File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                return Blob::from_data(&file_contents);
            }
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open the file"));
            }
        }
    }

    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let file_contents = utils::decompress(&data)?;

        let null_pos = file_contents
            .iter()
            .position(|element| *element == *"\0".as_bytes().first().unwrap())
            .ok_or(anyhow!("No null terminator in file"))?;

        let header = &file_contents[..null_pos];
        let content = &file_contents[null_pos..];

        let file_type = &header[0..4];
        if file_type != "blob".as_bytes() {
            return Err(anyhow!("File is not of type blob"));
        }

        return Ok(Blob {
            content: content[1..].to_owned(),
        });
    }
}

impl ObjectDump for Blob {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.reserve(self.content.len() + 8);
        bytes.extend_from_slice("blob ".as_bytes());
        bytes.extend_from_slice(&self.content.len().to_string().as_bytes());
        bytes.extend_from_slice("\0".as_bytes());
        bytes.extend_from_slice(&self.content);
        bytes
    }

    fn dump_to_file(&self) -> anyhow::Result<String> {
        let blob_content = self.convert_to_bytes();
        let filedata = utils::compress(&blob_content)?;
        let filename = utils::generate_filename(&blob_content);

        let path =
            String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR + "/" + filename.as_str();
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;
        Ok(filename)
    }
}
