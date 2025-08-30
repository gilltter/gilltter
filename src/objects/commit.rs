use std::{
    fs::File,
    io::{Read, Write},
    path::{self, Path},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{ObjectDump, ObjectPump, tree::TREE_TYPE_STRING},
    utils,
};

const COMMIT_TYPE_STRING: &'static [u8] = b"commit";

pub struct Commit {
    tree_sha: Option<String>,
    parent_commit_sha: Option<String>,
    username: Option<String>,
    email: Option<String>,
    message: Option<String>,
    secs_since_epoch: Option<u64>,
}

impl Commit {
    pub fn new() -> Self {
        Self {
            tree_sha: None,
            parent_commit_sha: None,
            username: None,
            email: None,
            message: None,
            secs_since_epoch: None,
        }
    }

    pub fn set_tree_sha(&mut self, sha: impl Into<String>) -> &mut Self {
        self.tree_sha = Some(sha.into());
        self
    }

    pub fn get_tree_sha(&self) -> Option<String> {
        self.tree_sha.clone()
    }

    pub fn set_parent_commit_sha(&mut self, sha: impl Into<String>) -> &mut Self {
        self.parent_commit_sha = Some(sha.into());
        self
    }

    pub fn get_parent_commit_sha(&self) -> Option<String> {
        self.parent_commit_sha.clone()
    }

    pub fn set_username(&mut self, username: impl Into<String>) -> &mut Self {
        self.username = Some(username.into());
        self
    }
    pub fn get_username(&self) -> Option<String> {
        self.username.clone()
    }

    pub fn set_email(&mut self, email: impl Into<String>) -> &mut Self {
        self.email = Some(email.into());
        self
    }
    pub fn get_email(&self) -> Option<String> {
        self.email.clone()
    }

    pub fn set_message(&mut self, message: impl Into<String>) -> &mut Self {
        self.message = Some(message.into());
        self
    }
    pub fn get_message(&self) -> Option<String> {
        self.message.clone()
    }
}

impl ObjectDump for Commit {
    fn convert_to_bytes(&self) -> Vec<u8> {
        if self.tree_sha.is_none() || self.username.is_none() || self.email.is_none() {
            panic!("Ты долбоеб?")
        }
        let mut bytes = Vec::new();

        bytes.extend_from_slice(COMMIT_TYPE_STRING);
        // bytes.extend_from_slice(format!(" {}\0", bytes_cnt).as_bytes());
        // Tree setup
        bytes.extend_from_slice(format!("tree {}", self.tree_sha.as_ref().unwrap()).as_bytes());

        // Parent commit setup (if there is one)
        if let Some(parent_sha) = self.parent_commit_sha.as_ref() {
            bytes.extend_from_slice(format!("parent {}", parent_sha).as_bytes());
        }

        // User info setup + current date
        let seconds_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap(); // it shouldnt fail right
        // timestamp is utc
        bytes.extend_from_slice(
            format!(
                "author {} {} {} ",
                self.username.as_ref().unwrap(),
                self.email.as_ref().unwrap(),
                seconds_since_epoch.as_secs()
            )
            .as_bytes(),
        );

        // Message
        bytes.extend_from_slice(format!("msg {}", self.message.as_ref().unwrap()).as_bytes());

        let bytes_cnt = bytes.len();
        let v = bytes.split_off(COMMIT_TYPE_STRING.len());
        bytes.extend_from_slice(&format!(" {}\0", bytes_cnt).as_bytes());
        bytes.extend(v.iter());

        bytes
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let commit_content = self.convert_to_bytes();
        let filename = utils::generate_filename(&commit_content);
        let filedata = utils::compress(&commit_content)?;

        let path = path::PathBuf::from(
            String::from(GILLTTER_PATH)
                + utils::get_separator()
                + GILLTER_OBJECTS_DIR
                + utils::get_separator()
                + filename.as_str(),
        );
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;

        Ok(filename)
    }
}

impl ObjectPump for Commit {
    // TODO: Range checking
    fn from_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut commit = Commit::new();
        let data = utils::decompress(data)?;

        let null_pos = data
            .iter()
            .position(|elem| *elem == "\0".as_bytes()[0])
            .ok_or(anyhow!("No null terminator"))?;
        let header = &data[0..null_pos];
        let content = &data[null_pos + 1..];

        if &header[0..COMMIT_TYPE_STRING.len()] != COMMIT_TYPE_STRING {
            return Err(anyhow!("Object type is incorrect"));
        }
        let size_commit_bytes = &header[COMMIT_TYPE_STRING.len() + 1..null_pos];
        let _commit_size = String::from_utf8_lossy(size_commit_bytes)
            .trim()
            .parse::<u32>()?;

        let mut data = content;

        // Get treee
        let tree_type_str = &data[0..TREE_TYPE_STRING.len()];
        if tree_type_str != TREE_TYPE_STRING {
            return Err(anyhow!("Want a tree here fuck you"));
        }

        data = &data[TREE_TYPE_STRING.len() + 1..]; // start at tree [p]dsadsasa7727 < here
        let tree_sha = String::from_utf8_lossy(&data[0..40]);
        commit.set_tree_sha(tree_sha);

        data = &data[40..];

        // get parent
        let debug_str = String::from_utf8_lossy(data);
        println!("Parent debug: {}", debug_str);
        let parent_type_str = &data[0.."parent".len()];
        if parent_type_str == b"parent" {
            // its ok, no parent
            data = &data["parent".len() + 1..];
            let parent_sha = String::from_utf8_lossy(&data[0..40]);
            commit.set_parent_commit_sha(parent_sha);

            // Get author
            data = &data[40..];
            // return Err(anyhow!("Want a parent here fuck u"));
        }

        let author_str = &data[0.."author".len()];
        if author_str != b"author" {
            return Err(anyhow!("Want ann author here fuck u"));
        }

        data = &data["author".len() + 1..]; // now we at username
        let space_pos = data
            .iter()
            .position(|elem| *elem == " ".as_bytes()[0])
            .ok_or(anyhow!("No space after author"))?;

        // Get username
        let username_str = &data[0..space_pos];
        let username = String::from_utf8_lossy(username_str);
        commit.set_username(username);

        // Get email
        data = &data[space_pos + 1..]; // we at email
        let space_pos = data
            .iter()
            .position(|elem| *elem == " ".as_bytes()[0])
            .ok_or(anyhow!("No space after username"))?;
        let email_str = &data[0..space_pos];
        let email = String::from_utf8_lossy(email_str);
        commit.set_email(email);

        // Get time since epoch
        data = &data[space_pos + 1..];
        let space_pos = data
            .iter()
            .position(|elem| *elem == " ".as_bytes()[0])
            .ok_or(anyhow!("No space after username"))?;
        let time_str = &data[0..space_pos];
        let time = String::from_utf8_lossy(time_str);
        let secs_since_epoch = time.parse::<u64>()?;
        commit.secs_since_epoch = Some(secs_since_epoch);

        // Get message
        data = &data[space_pos + 1..];
        let space_pos = data
            .iter()
            .position(|elem| *elem == " ".as_bytes()[0])
            .ok_or(anyhow!("no messag"))?;
        let message_str = &data[0..space_pos];
        if message_str != b"msg" {
            return Err(anyhow!("Its not a msg"));
        }

        let actual_messsage = String::from_utf8_lossy(&data[space_pos + 1..]);
        commit.set_message(actual_messsage);
        // done

        Ok(commit)
    }
    fn from_file(filepath: &Path) -> anyhow::Result<Self> {
        match File::open(filepath) {
            Ok(mut file) => {
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents)?;

                return Commit::from_data(&file_contents);
            }
            Err(why) => {
                eprintln!("Could not open the file: {}", why);
                return Err(anyhow!("Could not open file: {}", why));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        config::Config,
        objects::{
            self,
            blob::Blob,
            tree::{FileType, Object, Tree},
        },
    };

    use super::*;

    fn gilltter_add(filepath: &str) -> String {
        // let mut file = File::open(filepath).unwrap();
        // let mut contents = Vec::new();
        // file.read_to_end(&mut contents).unwrap();

        let mut blob = Blob::new();
        blob.set_data(filepath.as_bytes());

        let filename = blob.dump_to_file().unwrap();
        filename
    }

    #[test]
    fn create_commit_and_load_then() {
        let mut index_mock: HashMap<String, Object> = HashMap::new();
        let utils_filepath = String::from("src/utils.rs");
        let utils_sha1 = gilltter_add(&utils_filepath);
        index_mock.insert(
            utils_sha1.clone(),
            Object::new(
                objects::tree::FileType::RegularFile,
                utils_filepath.to_string(),
                utils_sha1.clone(),
            ),
        );

        let base_filepath = String::from("src/base.rs");
        let base_sha1 = gilltter_add(&base_filepath);
        index_mock.insert(
            base_sha1.clone(),
            Object::new(
                objects::tree::FileType::RegularFile,
                base_filepath.to_string(),
                base_sha1.clone(),
            ),
        );

        // Build a tree
        let mut tree = Tree::new();

        for (_, value) in index_mock.into_iter() {
            tree.add_object(Object::new(
                FileType::RegularFile,
                value.filepath,
                value.sha1_pointer,
            ));
        }

        let tree_name = tree.dump_to_file().unwrap();

        let tree = Tree::from_file(Path::new(&format!(".gilltter/objects/{}", tree_name))).unwrap();

        // now build a commit
        // we need tree, parent, user, message in commit
        let config = Config::from_file(Path::new(".gilltter/config")).unwrap();
        let username = String::from("bitch");
        let email = String::from("idiot@mgial.com");

        let mut commit = Commit::new();
        commit.set_tree_sha(tree_name);
        commit.set_username(username);
        commit.set_email(email);
        commit.set_message("fuck niggas");

        commit.convert_to_bytes();
        let commit_name = commit.dump_to_file().unwrap();

        let commit = Commit::from_file(Path::new(&format!(".gilltter/objects/{}", commit_name))).unwrap();
        println!(
            "'{}' '{}' '{}'",
            commit.get_email().unwrap(),
            commit.get_username().unwrap(),
            commit.get_message().unwrap()
        );
    }
}
