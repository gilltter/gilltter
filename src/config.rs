// we need to parse config and make functions to set these settings
// config format will be like:
/*
 * [<Category-name>]
 * <key> = <value> (no space)
 */

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Cursor, Read},
    path,
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_CONFIG_FILE, GILLTTER_PATH},
    objects::ObjectPump,
};

pub struct Config {
    variables: HashMap<String, HashMap<String, String>>, // Category -> [<var-name> <var-value, ...]
}

impl Config {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn parse(data: String) -> Config {
        let mut config = Config::new();
        let mut current_category = String::new();
        let reader = BufReader::new(Cursor::new(data));

        for line in reader.lines() {
            let line = line.unwrap();
            let line = line.trim();

            if line.starts_with('[') && line.ends_with(']') {
                let category_name = &line[1..line.len() - 1];
                current_category = category_name.to_owned();
            } else {
                if let Some(equal_pos) = line.chars().position(|ch| ch == '=') {
                    let name = &line[0..equal_pos];
                    let value = &line[equal_pos + 1..];
                    config.add(&current_category, name, value);
                }
            }
        }
        config
    }
    pub fn add(&mut self, category_name: &str, name: &str, value: &str) {
        self.variables
            .entry(category_name.to_owned())
            .or_insert(HashMap::new())
            .insert(name.to_owned(), value.to_owned());
    }
    pub fn get(&self, category_name: &str, name: &str) -> Option<String> {
        if let Some(vars) = self.variables.get(category_name) {
            return vars.get(name).map(|s| s.to_string());
        }
        None
    }
    pub fn get_int(&self, category_name: &str, name: &str) -> Option<i32> {
        self.variables
            .get(category_name)
            .and_then(|vars| vars.get(name).and_then(|s| s.parse::<i32>().ok()))
    }
}

impl ObjectPump for Config {
    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let data = String::from_utf8_lossy(data).to_string();
        Ok(Config::parse(data))
    }
    fn from_file(filepath: &str) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;

                Config::from_data(&contents)
            }
            Err(why) => Err(anyhow!("Could not open file: {}", why)),
        }
    }
}
