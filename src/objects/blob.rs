use std::{
    fs::{self, File},
    io::{Read, Write},
};

use anyhow::anyhow;

const BLOB_TYPE_STRING: &'static [u8] = b"blob";

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump, SPACE_STR},
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

    pub fn append_data(&mut self, data: &[u8]) {
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
        let content = &file_contents[null_pos + 1..];

        let file_type = &header[0..4];
        if file_type != BLOB_TYPE_STRING {
            return Err(anyhow!("File is not of type blob"));
        }
        let blob_size: usize = String::from_utf8_lossy(&header[5..null_pos])
            .parse()
            .unwrap();
        if blob_size != content.len() {
            return Err(anyhow!(
                "Blob size doesn't match actual content size: {} vs {}",
                blob_size,
                content.len()
            ));
        }

        return Ok(Blob {
            content: content[1..].to_owned(),
        });
    }
}

impl ObjectDump for Blob {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(BLOB_TYPE_STRING);
        bytes.extend_from_slice(SPACE_STR);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_blob_and_load_it() {
        let mut contents = Vec::<u8>::new();
        contents.extend_from_slice("heil gilltter".as_bytes());

        let mut blob = Blob::new();
        blob.set_data(&contents);

        let filename = blob.dump_to_file().unwrap();
        println!("f: {}", filename);
        let blob = Blob::from_file(&format!(".gilltter/objects/{}", filename)).unwrap();
    }
}
