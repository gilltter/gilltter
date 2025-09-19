use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, arg, command};

use crate::{
    base::{GILLTTER_INDEX_FILE, GILLTTER_PATH},
    index::index::Index,
    objects::ObjectPump,
};

mod base;
mod commands;
mod config;
mod index;
mod objects;
mod utils;

#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "gilltter")]
#[command(about = "Simple version control system in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {},

    #[command(arg_required_else_help = true)]
    Add {
        #[command(subcommand)]
        command: AddCommands,
    },

    #[command(arg_required_else_help = true)]
    Commit {
        message: Option<String>,
    },

    Status,
}

#[derive(Subcommand, Debug, Clone)]
enum AddCommands {
    All,
    Filename {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
}

fn main() {
    // base::gilltter_status().unwrap();

    // return;
    // println!("Welcome to {}", "Penis".red().bold());

    let args = Cli::parse();
    match args.command {
        Commands::Init {} => base::gilltter_init().unwrap(),
        Commands::Add { command } => match command {
            AddCommands::Filename { file } => {
                // println!("Adding");
                if let Err(why) = commands::add::add(&file) {
                    eprintln!(
                        "Could not add a file '{}', because: {}",
                        file.to_string_lossy(),
                        why
                    );
                }
            }
            AddCommands::All => {
                todo!("Add all");
            }
        },
        Commands::Commit { message } => {
            match Index::from_file(&Path::new(GILLTTER_PATH).join(GILLTTER_INDEX_FILE)) {
                Ok(index) => {
                    if let Err(why) =
                        commands::commit::commit(&index, message.expect("Type a message"))
                    {
                        eprintln!("Could not commit: {}", why);
                    }
                }
                Err(why) => {
                    eprintln!("Could not parse index file: {}", why);
                }
            }
        }
        Commands::Status => {
            if let Err(why) = commands::status::gilltter_status() {
                eprintln!("Status failed: {}", why);
            }
        }
    }
}
