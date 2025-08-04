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

fn gilltter_pick_blob(filepath: &str) -> Blob {
    Blob::from_file(filepath).unwrap()
}

fn mock_tree() {
    let mut index_mock: HashMap<String, Object> = HashMap::new();
    let utils_filepath = String::from("src/utils.rs");
    let utils_sha1 = gilltter_add(&utils_filepath);
    index_mock.insert(
        utils_sha1.clone(),
        Object::new(
            objects::tree::ObjectType::Blob,
            utils_filepath.to_string(),
            utils_sha1.clone(),
        ),
    );

    let base_filepath = String::from("src/base.rs");
    let base_sha1 = gilltter_add(&base_filepath);
    index_mock.insert(
        base_sha1.clone(),
        Object::new(
            objects::tree::ObjectType::Blob,
            base_filepath.to_string(),
            base_sha1.clone(),
        ),
    );

    // Build a tree
    let mut tree = Tree::new();

    for (_, value) in index_mock.into_iter() {
        tree.add_object(Object::new(
            ObjectType::Blob,
            value.filepath,
            value.sha1_pointer,
        ));
    }

    let name = tree.dump_to_file().unwrap();

    let tree = Tree::from_file(&format!(".gilltter/objects/{}", name)).unwrap();
    let tree_objects = tree.get_objects();
    for (_, value) in tree_objects.into_iter() {
        println!(
            "{} {} {}",
            value.obj_type as u32, value.filepath, value.sha1_pointer
        );
    }
}

fn config_mock() {
    let cfg_file = ".gilltter/config";
    let mut file = File::open(cfg_file).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    println!("{}", data);
    let config = Config::parse(data);

    let username = config.get("General", "username").unwrap();
    let age = config.get_int("General", "age").unwrap();
    println!("'{}' '{}'", username, age);
}

fn main() {
    gilltter_init();

    let mut index_mock: HashMap<String, Object> = HashMap::new();

    // Imagine tree constructing when commiting
    {
        let utils_filepath = String::from("src/utils.rs");
        let utils_sha1 = gilltter_add(&utils_filepath);
        index_mock.insert(
            utils_sha1.clone(),
            Object::new(
                objects::tree::ObjectType::Blob,
                utils_filepath.to_string(),
                utils_sha1.clone(),
            ),
        );

        let base_filepath = String::from("src/base.rs");
        let base_sha1 = gilltter_add(&base_filepath);
        index_mock.insert(
            base_sha1.clone(),
            Object::new(
                objects::tree::ObjectType::Blob,
                base_filepath.to_string(),
                base_sha1.clone(),
            ),
        );

        // Build a tree
        let mut tree = Tree::new();

        for (_, value) in index_mock.into_iter() {
            tree.add_object(Object::new(
                ObjectType::Blob,
                value.filepath,
                value.sha1_pointer,
            ));
        }

        let name = tree.dump_to_file().unwrap();

        let tree = Tree::from_file(&format!(".gilltter/objects/{}", name)).unwrap();

        // now build a commit
        // we need tree, parent, user, message in commit
    }
}
