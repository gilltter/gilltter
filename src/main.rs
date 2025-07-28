use std::{
    fs::File,
    io::{Read, Write},
};

use crate::objects::{ToFile, blob::Blob};

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

fn main() {
    gilltter_init();

    // Imagine git add
    {
        let mut file = File::open("src/main.rs").unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let mut blob = Blob::new();
        blob.set_data(&contents);

        let blob_content = blob.convert_to_bytes();
        let filedata = utils::compress(&blob_content).unwrap();
        let filename = utils::generate_filename(&blob_content);

        let mut file = File::create(filename).unwrap();
        file.write_all(&filedata).unwrap();
        file.flush().unwrap();
    }

    // Pick file
    {
        let blob = Blob::from_file("cc2ecb0c8a282e9d322a66f470cd681c20082f33").unwrap();
        println!("Content: {}", String::from_utf8_lossy(&blob.get_data()))
    }
}
