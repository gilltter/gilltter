use crate::base::GILLTTER_IGNORE_FILE;
use anyhow::anyhow;
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader},
};

pub(crate) fn gilltter_get_ignorefile() -> anyhow::Result<Vec<String>> {
    let mut result = Vec::new();
    let file = File::open(GILLTTER_IGNORE_FILE)?;

    let reader = BufReader::new(file);
    for line in reader.lines().into_iter() {
        let line = line?;
        if line.starts_with('#') {
            continue;
        }

        result.push(line);
    }
    Ok(result)
}

pub(crate) fn should_ignore(entry: &OsStr, ignore_files: &[String]) -> anyhow::Result<bool> {
    let name = entry
        .to_str()
        .ok_or_else(|| anyhow!("Could not convert OsStr to &str"))?;
    for el in ignore_files {
        let pat = glob::Pattern::new(el)?; // может вернуть ошибку
        if pat.matches(name) {
            return Ok(true);
        }
    }

    Ok(false)
}
