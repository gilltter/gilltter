use std::{fs, path};

const GILLTTER_PATH: &str = ".gilltter";

fn main() {
    if !path::Path::new(GILLTTER_PATH).exists() {
        if let Err(why) = fs::create_dir(GILLTTER_PATH) {
            eprintln!("Could not create Gilltter project directory: {}", why);
        }
    }

    println!("Hello, Dear gitler users!");
}
