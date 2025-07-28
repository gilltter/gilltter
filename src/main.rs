use std::{fs::File, io::Write};

use crate::objects::{ToFile, blob::Blob};

mod base;
mod index;
mod objects;
mod utils;

fn main() {
    // if let Err(why) = base::make_sure_gilltter_dir_exists() {
    //     eprintln!(
    //         "Could not verify, that gilltter directory and its files exist: {}",
    //         why
    //     );
    // }

    // let file_info = utils::get_file_info(".gilltter");
    // if (file_info.st_mode & libc::S_IFMT) == libc::S_IFDIR {
    //     println!("{} is a directory, all good 👍", base::GILLTTER_PATH);
    // }

    let mut blob = Blob::new();
    blob.append_data("niggas faggots".as_bytes());
    // write to file
    // then read from file
    let mut file = File::create("fuck.me").unwrap();
    file.write(&blob.convert_to_bytes()).unwrap();
    file.flush().unwrap();
    drop(file);

    println!("Hello, Dear gilltter users!");
}
