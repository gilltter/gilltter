use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::{base::GILLTTER_PATH, gilltter_add};

pub fn add_one_in_index(filepath: &Path) -> anyhow::Result<()> {
    let index_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(Path::new(GILLTTER_PATH).join("index"))?;

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

    let utils_sha1 = gilltter_add(filepath.to_str().unwrap()).unwrap();

    let mut writer = BufWriter::new(index_file);
    writer.seek(std::io::SeekFrom::Start(start_index as u64))?;
    writer.write_all(
        format!("{} {} {}\n", utils_sha1, "blob", filepath.to_str().unwrap()).as_bytes(),
    )?;
    Ok(())
}
