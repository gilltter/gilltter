use std::{
    fs::File,
    io::{Read, Write},
};

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, blob::Blob},
};

mod base;
mod index;
mod objects;
mod utils;

fn gilltter_init() {
    if let Err(why) = base::make_sure_gilltter_dir_exists() {
        eprintln!(
            "Could not verify, that gilltter directory and its files exist: {}",
            why
        );
    }

    let file_info = utils::get_file_info(".gilltter");
    if (file_info.st_mode & libc::S_IFMT) == libc::S_IFDIR {
        println!("{} is a directory, all good 👍", base::GILLTTER_PATH);
    }
}

fn gilltter_add(filepath: &str) -> String {
    let mut file = File::open(filepath).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();

    let mut blob = Blob::new();
    blob.set_data(&contents);

    let filename = blob.dump_to_file().unwrap();
    filename
    // let blob_content = blob.convert_to_bytes();
    // let filedata = utils::compress(&blob_content).unwrap();
    // let filename = utils::generate_filename(&blob_content);

    // let path = String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR + "/" + filename.as_str();
    // let mut file = File::create(path).unwrap();
    // file.write_all(&filedata).unwrap();
    // file.flush().unwrap();
    // filename
}

fn gilltter_pick_blob(filepath: &str) -> Blob {
    Blob::from_file(filepath).unwrap()
}

fn main() {
    gilltter_init();

    // Imagine git add
    {
        let filename = gilltter_add("src/utils.rs");

        let blob = gilltter_pick_blob(
            &(String::from(GILLTTER_PATH) + "/" + GILLTER_OBJECTS_DIR + "/" + filename.as_str()),
        );
        println!("Content: {}", String::from_utf8_lossy(&blob.get_data()))
    }

    // Pick file
    {
        // let blob = gilltter_pick_blob("0e7a803d7ae672f1cc5124009616b8c2d25815b7");
        // println!("Content: {}", String::from_utf8_lossy(&blob.get_data()))
    }
}
