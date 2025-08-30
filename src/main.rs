use std::{fs::File, io::Read, path::Path};

use clap::{Arg, ArgAction, Command};

use crate::objects::{ObjectDump, blob::Blob};

mod base;
mod config;
mod index;
mod objects;
mod utils;

fn gilltter_init() -> anyhow::Result<()> {
    base::create_gilltter_project()
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

fn main() {
    gilltter_init().unwrap();
    let mut commands = Command::new("gilltter")
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

    let help = commands.render_help();
    let args = commands.get_matches();
    let command = args.subcommand().unwrap();

    match command.0 {
        "init" => gilltter_init().unwrap(),
        "add" => {
            if *command.1.get_one::<bool>("all").unwrap() {
                println!("add all");
            } else if let Some(filename) = command.1.get_one::<String>("filename") {
                println!("add {filename}");
                if let Err(why) = index::index::add_one_in_index(Path::new(filename)) {
                    eprintln!("Could not add a file '{}', because: {}", filename, why);
                }
            } else {
                print!("{help}");
            }
        }
        _ => (), // unreachable
    }
}
