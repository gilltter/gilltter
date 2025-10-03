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

                let data = utils::decompress(&file_contents)?;
                // let data = file_contents;
                return Blob::from_raw_data(&data);
            }
            Err(why) => {
                return Err(anyhow!(
                    "Could not open the file {}: {}",
                    filepath.to_string_lossy(),
                    why
                ));
            }
        }
    }

    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let space_pos = data
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| anyhow!("Invalid blob header: missing space"))?;
        let null_pos = data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow!("Invalid blob header: missing null terminator"))?;

        if space_pos >= null_pos {
            return Err(anyhow!("Invalid blob header: space comes after null"));
        }

        let obj_type = std::str::from_utf8(&data[..space_pos])
            .map_err(|e| anyhow!("Invalid UTF-8 in object type: {}", e))?;
        if obj_type != "blob" {
            return Err(anyhow!(
                "Object type is wrong, expected: blob, got: {}",
                obj_type
            ));
        }

        // длина — байты между пробелом и нулём (ascii digits)
        let len_bytes = &data[space_pos + 1..null_pos];
        let len_str = std::str::from_utf8(len_bytes)
            .map_err(|e| anyhow!("Invalid UTF-8 in length field: {}", e))?;
        let obj_len = len_str
            .parse::<usize>()
            .map_err(|e| anyhow!("Invalid length value '{}': {}", len_str, e))?;

        let content_start = null_pos + 1;
        if data.len() < content_start {
            return Err(anyhow!("Data too short: no content after header"));
        }

        if data.len() != content_start + obj_len {
            return Err(anyhow!(
                "Blob size does not match: header says {}, actual {}",
                obj_len,
                data.len() - content_start
            ));
        }

        let content = data[content_start..content_start + obj_len].to_vec();

        Ok(Blob { content })
    }
}

impl ObjectDump for Blob {
    // Blob cant fail here
    fn convert_to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        write!(
            &mut bytes,
            "{} {}\0",
            String::from_utf8_lossy(BLOB_TYPE_STRING),
            self.content.len()
        )?;
        bytes.extend_from_slice(&self.content);
        Ok(bytes)
    }

    fn dump_to_file(&self) -> anyhow::Result<String> {
        let blob_content = self.convert_to_bytes()?;
        let filedata = utils::compress(&blob_content)?;
        // let filedata = blob_content.clone(); // TODO: Remove after testing
        let filename = utils::generate_hash(&blob_content);

        let path = Path::new(GILLTTER_PATH)
            .join(GILLTER_OBJECTS_DIR)
            .join(filename.as_str());
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;
        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn save_blob() {
    //     let mut contents = Vec::<u8>::new();
    //     contents.extend_from_slice("hi gilltter".as_bytes());

    //     let mut blob = Blob::new();
    //     blob.set_data(&contents);

    //     let blob_bytes = blob.convert_to_bytes().unwrap();
    //     assert!(!blob_bytes.is_empty());
    // }

    fn get_blob() -> Blob {
        let mut contents = Vec::<u8>::new();
        contents.extend_from_slice("hi gilltter".as_bytes());

        let mut blob = Blob::new();
        blob.set_data(&contents);

        let blob_bytes = blob.convert_to_bytes();
        blob
    }

    #[test]
    fn blob_from_compressed_bytes() {
        let blob = get_blob();
        let blob_bytes = blob.convert_to_bytes().unwrap();
        let compressed_bytes = utils::compress(&blob_bytes).unwrap();

        let decompressed_bytes = utils::decompress(&compressed_bytes).unwrap();
        let blob = Blob::from_raw_data(&decompressed_bytes).unwrap();
        let data = blob.get_data();
        assert_eq!(String::from_utf8_lossy(&data), "hi gilltter");
    }
}
