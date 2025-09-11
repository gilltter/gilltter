use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use anyhow::anyhow;

const BLOB_TYPE_STRING: &'static [u8] = b"blob";

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump},
    utils,
};

#[derive(Clone)]
pub struct Blob {
    content: Vec<u8>,
}

impl Blob {
    pub fn new() -> Self {
        Self { content: vec![] }
    }

    #[allow(dead_code)]
    pub fn get_data(&self) -> Vec<u8> {
        self.content.clone()
    }

    #[allow(dead_code)]
    pub fn append_data(&mut self, data: &[u8]) {
        self.content.extend_from_slice(data);
    }
    pub fn set_data(&mut self, data: &[u8]) {
        self.content = data.to_owned();
    }
}

impl ObjectPump for Blob {
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match fs::File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                // let data = utils::decompress(&file_contents)?;
                let data = file_contents;
                return Blob::from_raw_data(&data);
            }
            Err(why) => {
                return Err(anyhow!("Could not open the file {}: {}", filepath.to_string_lossy(), why));
            }
        }
    }

    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let file_contents = data.to_owned(); // TODO: Remove after testing

        let null_pos = file_contents
            .iter()
            .position(|element| *element == *"\0".as_bytes().first().unwrap())
            .ok_or(anyhow!("No null terminator in file"))?;

        let header = &file_contents[..null_pos];
        let blob_size: usize = String::from_utf8_lossy(&header[5..null_pos])
            .parse().map_err(|err| anyhow!("Blob size is not usize: {}", err))?;
        let file_type = &header[0..4];
        if file_type != BLOB_TYPE_STRING {
            return Err(anyhow!("File is not of type blob"));
        }

        let content = &file_contents[null_pos + 1..];

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
        bytes.extend_from_slice(
            format!(
                "{} {}\0",
                String::from_utf8_lossy(BLOB_TYPE_STRING),
                self.content.len()
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&self.content);
        bytes
    }

    fn dump_to_file(&self) -> anyhow::Result<String> {
        let blob_content = self.convert_to_bytes();
        // let filedata = utils::compress(&blob_content)?;
        let filedata = blob_content.clone(); // TODO: Remove after testing
        let filename = utils::generate_hash(&blob_content);

        let path = Path::new(GILLTTER_PATH).join(GILLTER_OBJECTS_DIR).join(filename.as_str());
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

        #[allow(unused)]
        let blob = Blob::from_file(Path::new(&format!(".gilltter/objects/{}", filename))).unwrap();
    }
}
