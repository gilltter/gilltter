use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    config::Config,
    objects::{
        ObjectDump, ObjectPump,
        blob::Blob,
        commit::Commit,
        tree::{Object, ObjectType, Tree},
    },
};

mod base;
mod config;
mod index;
mod objects;
mod utils;

fn gilltter_init() -> anyhow::Result<()> {
    base::make_sure_gilltter_dir_exists()
}

fn gilltter_add(filepath: &str) -> anyhow::Result<String> {
    let mut file = File::open(filepath)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let mut blob = Blob::new();
    blob.set_data(&contents);

    let filename = blob.dump_to_file()?;
    Ok(filename)
}

fn gilltter_pick_blob(filepath: &str) -> Blob {
    Blob::from_file(filepath).unwrap()
}

fn main() {
    base::create_gilltter_project().unwrap();
    if let Err(_) = base::does_gilltter_proj_exist() {
        panic!("Gilltter Project is not initialized");
    }
    let args: Vec<String> = std::env::args().map(|arg| arg.to_string()).collect();
    if &args[1] == "add" {
        gilltter_add(&args[2]).unwrap();
    }
}
