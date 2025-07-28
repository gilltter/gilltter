use std::{fs, io::Read};

use anyhow::anyhow;

use crate::{objects::ToFile, utils};

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
    pub fn from_file(filepath: &str) -> anyhow::Result<Self> {
        match fs::File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;
                let file_contents = utils::decompress(&file_contents)?;

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
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open the file"));
            }
        }
    }
}

impl ToFile for Blob {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.reserve(self.content.len() + 8);
        bytes.extend_from_slice("blob ".as_bytes());
        bytes.extend_from_slice(&self.content.len().to_string().as_bytes());
        bytes.extend_from_slice("\0".as_bytes());
        bytes.extend_from_slice(&self.content);
        bytes
    }
}
