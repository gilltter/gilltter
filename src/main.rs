use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use clap::{Arg, ArgAction, Command};

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
}

fn main() {
    let mut app = Command::new("gilltter")
        .version("0.1")
        .about("Simple version control system on Rust")
        .subcommand(Command::new("init").about("Initialize gilltter repo"))
        .subcommand(
            Command::new("add").about("Adding file to index").args([
                Arg::new("all")
                    .short('a')
                    .long("all")
                    .action(ArgAction::SetTrue),
                Arg::new("filename").index(1),
            ]),
        );

    let help = app.render_help();
    let args = app.get_matches();
    let command = args.subcommand().unwrap();

    match command.0 {
        "init" => gilltter_init(),
        "add" => {
            if *command.1.get_one::<bool>("all").unwrap() {
                println!("add all");
            } else if let Some(filename) = command.1.get_one::<String>("filename") {
                println!("add {filename}");
                index::index::add_one_in_index(Path::new(filename)).unwrap();
            } else {
                print!("{help}");
            }
        }
        _ => (), // unreachable
    }
}
