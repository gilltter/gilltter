use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf}, str::FromStr,
};
use anyhow::anyhow;
use crate::{
    base::{GILLTER_CONFIG_FILE, GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    gilltter_add,
    objects::{ObjectDump, ObjectPump},
    utils,
};

enum IndexType {
    RegularFile,
    SymbolicLink,
}

impl IndexType {
    pub fn to_bytes(&self) -> Vec<u8> {
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

struct IndexEntry {
    index_type: IndexType,
    ctime: i64,     // metadata last changed time
    mtime: i64, // file contents last changed time (used for comparing working tree with index, if differs, then file not staged), also used for comparing index file with last commit,
    file_size: u64, // in bytes
    filename: PathBuf,
    sha1_hash: String, // goes last, cuz it is fixed
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
                self.ctime, self.mtime, self.file_size, self.filename.to_str().expect("No filename"), self.sha1_hash
            )
            .as_bytes(),
        ); // TODO: Get rid of strings and make it compact and optimized (binary format)
        bytes
    }
}

struct Index {
    indices: Vec<IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Self { indices: Vec::new() }
    }
}

impl ObjectPump for Index {
    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut index = Index::new();

        let reader = BufReader::new(Cursor::new(data));
        for line in reader.lines() {
            let line = line?;
            let elements_vec: Vec<&str> = line.split(' ').collect();

            let idx_type = IndexType::from_bytes(elements_vec[0].as_bytes()).expect("Index type should be here");
            let ctime = elements_vec[1].parse::<i64>()?;
            let mtime = elements_vec[2].parse::<i64>()?;
            let file_size = elements_vec[3].parse::<u64>()?;
            let filename = PathBuf::from_str(elements_vec[4])?;
            let sha1_hash = elements_vec[5].to_owned();

            index.indices.push(IndexEntry::new(ctime, mtime, file_size, idx_type, filename, sha1_hash));
        }

        Ok(index)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                return Self::from_data(&file_contents);
            }
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open the file"));
            }
        }
    }
}

impl ObjectDump for Index {
    fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for index in &self.indices {
            bytes.extend_from_slice(&index.convert_to_bytes());
        }
        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let index_content = self.convert_to_bytes();

        let path = Path::new(GILLTTER_PATH).join(GILLTER_CONFIG_FILE);
        let mut index_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)?;

        index_file.write_all(&index_content)?;
        index_file.flush()?;
        Ok(path.to_string_lossy().to_string())
    }
}

pub fn add_one_in_index(filepath: &Path) -> anyhow::Result<()> {
    let mut index = Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTER_CONFIG_FILE))?;
    if index.indices.iter().find(|element| element.filename == filepath).is_some() {
        eprintln!("Such file already exists, fuck you!");
        return Err(anyhow!("Such file already exists"));
    }

    // println!("we here");
    let file_sha1 = gilltter_add(filepath.to_str().unwrap()).unwrap();
    let add_file_metadata = std::fs::metadata(filepath)?;
    let entry: IndexEntry = IndexEntry::new(
        add_file_metadata.ctime(),
        add_file_metadata.mtime(),
        add_file_metadata.size(),
        if add_file_metadata.is_symlink() { IndexType::SymbolicLink } else { IndexType::RegularFile },
        PathBuf::from_str(&filepath.to_string_lossy())?,
        file_sha1,
    );
    index.indices.push(entry);

    index.dump_to_file()?;
    Ok(())
}


/* old parsing */
/*
let mut reader = BufReader::new(index_file.try_clone()?);
    let mut start_index = 0;
    for i in reader.by_ref().lines() {
        if let Ok(s) = i {
            let s_ = s.split_ascii_whitespace().collect::<Vec<&str>>();
            let thing_type = s_[1];
            let name = PathBuf::from(&s_[2..].join(" "));
            if filepath.parent().unwrap() == name.parent().unwrap() {
                break;
            }
            println!("{:?} {:?}", filepath, name);
            start_index += s.len() + 1;
        }
    }
*/