use crate::{
    base::{self, GILLTER_CONFIG_FILE, GILLTER_HEAD_FILE, GILLTTER_INDEX_FILE, GILLTTER_PATH},
    config::{self, Config},
    objects::{
        ObjectDump, ObjectPump,
        commit::Commit,
        tree::{self, Tree, TreeObject},
    },
    utils,
};
use anyhow::anyhow;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Clone)]
pub enum IndexType {
    RegularFile,
    SymbolicLink,
}

impl IndexType {
    pub fn to_bytes(&self) -> Vec<u8> {
        // TODO: Make it take a byte, not 6
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

#[derive(Debug, Clone)]
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

    pub fn add(&mut self, entry: IndexEntry) {
        self.indices.push(entry);
    }

    pub fn remove_all(&mut self) {
        self.indices.clear();
    }

    pub fn remove(&mut self, filepath: &Path) -> bool {
        let pos = self
            .indices
            .iter()
            .position(|val| val.filename.as_path() == Path::new(filepath));
        if let Some(pos) = pos {
            self.indices.remove(pos);
            return true;
        }
        false
    }
}

impl ObjectPump for Index {
    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut index = Index::new();

        let reader = BufReader::new(Cursor::new(data));
        for line in reader.lines() {
            let line = line?;
            let elements_vec: Vec<&str> = line.split(' ').collect();

            // Extract fields
            let idx_type = IndexType::from_bytes(elements_vec[0].as_bytes())
                .ok_or(anyhow!("There is no index type bytes"))?;
            let ctime = elements_vec[1].parse::<i64>()?;
            let mtime = elements_vec[2].parse::<i64>()?;
            let file_size = elements_vec[3].parse::<u64>()?;
            let filename = PathBuf::from_str(elements_vec[4])?;
            let sha1_hash = elements_vec[5].to_owned();

            let entry = IndexEntry::new(ctime, mtime, file_size, idx_type, filename, sha1_hash);
            index.add(entry);
        }

        Ok(index)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents).unwrap();

                // let data = utils::decompress(&file_contents)?;
                let data = file_contents;
                return Self::from_raw_data(&data);
            }
            Err(why) => {
                return Err(anyhow!(
                    "Could not open index file: '{}', because {}",
                    filepath.to_string_lossy(),
                    why
                ));
            }
        }
    }
}

impl ObjectDump for Index {
    fn convert_to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        if self.indices.is_empty() {
            return Err(anyhow!("Can't convert empty Index to bytes"));
            // panic!("Can't convert empty Index to bytes");
        }
        let mut bytes = Vec::new();
        for index in &self.indices {
            bytes.extend_from_slice(&index.convert_to_bytes());
        }
        Ok(bytes)
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let index_content = self.convert_to_bytes()?;
        // let compressed_content = utils::compress(&index_content)?;
        let compressed_content = index_content; // TODO: Remove after testing

        let path = Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE);
        let mut index_file = OpenOptions::new().write(true).truncate(true).open(&path)?; // No point in using 'create(true)', since files are there at this point

        index_file.write_all(&compressed_content)?;
        index_file.flush()?;
        Ok(path.to_string_lossy().to_string())
    }
}
