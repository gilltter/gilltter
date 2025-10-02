use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;

use crate::{
    base::{GILLTER_OBJECTS_DIR, GILLTTER_PATH},
    objects::{
        ObjectDump, ObjectPump,
        tree::{self, TREE_TYPE_STRING},
    },
    utils,
};

const COMMIT_TYPE_STRING: &'static str = "commit";

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

    #[allow(dead_code)]
    pub fn set_tree_sha(&mut self, sha: String) -> &mut Self {
        self.tree_sha = Some(sha.into());
        self
    }

    #[allow(dead_code)]
    pub fn get_tree_sha(&self) -> Option<String> {
        self.tree_sha.clone()
    }

    pub fn set_parent_commit_sha(&mut self, sha: Option<String>) -> &mut Self {
        self.parent_commit_sha = sha;
        self
    }

    #[allow(dead_code)]
    pub fn get_parent_commit_sha(&self) -> Option<String> {
        self.parent_commit_sha.clone()
    }

    pub fn set_username(&mut self, username: impl Into<String>) -> &mut Self {
        self.username = Some(username.into());
        self
    }

    #[allow(dead_code)]
    pub fn get_username(&self) -> Option<String> {
        self.username.clone()
    }

    pub fn set_email(&mut self, email: impl Into<String>) -> &mut Self {
        self.email = Some(email.into());
        self
    }

    #[allow(dead_code)]
    pub fn get_email(&self) -> Option<String> {
        self.email.clone()
    }

    pub fn set_message(&mut self, message: impl Into<String>) -> &mut Self {
        self.message = Some(message.into());
        self
    }

    #[allow(dead_code)]
    pub fn get_message(&self) -> Option<String> {
        self.message.clone()
    }
}

impl ObjectDump for Commit {
    fn convert_to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let tree_sha = self
            .tree_sha
            .as_ref()
            .ok_or(anyhow!("tree_sha is not set"))?;
        let username = self
            .username
            .as_ref()
            .ok_or(anyhow!("username is not set"))?;
        let email = self.email.as_ref().ok_or(anyhow!("email is not set"))?;
        let message = self.message.as_ref().ok_or(anyhow!("message is not set"))?;

        // let mut bytes = Vec::new();
        let mut bytes = Vec::new();
        // Tree setup
        write!(&mut bytes, "{} {}", tree::TREE_TYPE_STRING, tree_sha)?;

        // Parent commit setup (if there is one)
        if let Some(parent_sha) = self.parent_commit_sha.as_ref() {
            write!(&mut bytes, "parent {}", parent_sha)?;
        }

        // User info setup + current date
        let seconds_since_epoch = self.secs_since_epoch.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        // timestamp is utc
        write!(
            &mut bytes,
            "author {} {} {} ",
            username, email, seconds_since_epoch
        )?;
        write!(&mut bytes, "msg {}", message)?;

        let bytes_cnt = bytes.len();
        let mut v = Vec::new();
        write!(&mut v, "{} {}\0", COMMIT_TYPE_STRING, bytes_cnt)?;
        v.extend(bytes.iter());

        Ok(v)
    }
    fn dump_to_file(&self) -> anyhow::Result<String> {
        let commit_content = self.convert_to_bytes()?;
        let filename = utils::generate_hash(&commit_content);
        // let filedata = utils::compress(&commit_content)?;
        let filedata = commit_content; // TODO: Remove after testing

        let path = Path::new(GILLTTER_PATH)
            .join(GILLTER_OBJECTS_DIR)
            .join(filename.as_str());
        // println!("Comit path: {:#?} {}", path, filedata.len());
        let mut file = File::create(path)?;
        file.write_all(&filedata)?;
        file.flush()?;

        Ok(filename)
    }
}

impl ObjectPump for Commit {
    fn from_raw_data(data: &[u8]) -> anyhow::Result<Self> {
        let mut commit = Commit::new();

        let null_pos = data
            .iter()
            .position(|elem| *elem == "\0".as_bytes()[0])
            .ok_or(anyhow!("No null terminator"))?;
        let header = &data[0..null_pos];
        let content = &data[null_pos + 1..];

        if &header[0..COMMIT_TYPE_STRING.len()] != COMMIT_TYPE_STRING.as_bytes() {
            return Err(anyhow!("Object type is incorrect"));
        }
        let size_commit_bytes = &header[COMMIT_TYPE_STRING.len() + 1..null_pos];
        let commit_size = String::from_utf8_lossy(size_commit_bytes)
            .trim()
            .parse::<u32>()?;

        let mut data = content;

        if commit_size as usize != data.len() {
            return Err(anyhow!(
                "Commti size does not match {} != {}",
                commit_size,
                data.len()
            ));
        }

        // Get tree
        let tree_type_str = &data[0..TREE_TYPE_STRING.len()];

        if tree_type_str != TREE_TYPE_STRING.as_bytes() {
            return Err(anyhow!("Want a tree here"));
        }

        data = &data[TREE_TYPE_STRING.len() + 1..]; // start at tree [p]dsadsasa7727 < here

        let tree_sha = String::from_utf8_lossy(&data[0..40]);
        commit.set_tree_sha(tree_sha.to_string());

        data = &data[40..];

        // get parent
        // let debug_str = String::from_utf8_lossy(data);
        let parent_type_str = &data[0.."parent".len()];
        if parent_type_str == b"parent" {
            // its ok, no parent
            data = &data["parent".len() + 1..];
            let parent_sha = String::from_utf8_lossy(&data[0..40]);
            commit.set_parent_commit_sha(Some(parent_sha.to_string()));

            // Get author
            data = &data[40..];
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
            return Err(anyhow!("Expected msg"));
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

                // let data = utils::decompress(&file_contents)?;
                let data = file_contents;
                return Commit::from_raw_data(&data);
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
    use super::*;

    #[test]
    #[should_panic]
    fn panic_on_unset_fields() {
        let commit = Commit::new();
        commit.convert_to_bytes().unwrap();
    }

    #[test]
    fn no_panic_if_fields_set() {
        let mut commit = Commit::new();
        commit.set_tree_sha(String::from_utf8_lossy(&[87u8; 40]).to_string());
        commit.set_username("Pencil".to_string());
        commit.set_email("pedosia@gmail.com".to_string());
        commit.set_message("Wotofak bitch ya molodoi legenda".to_string());
        commit.convert_to_bytes().unwrap();
    }

    #[test]
    fn commit_to_file() {
        let mut commit = Commit::new();
        commit.set_tree_sha(String::from_utf8_lossy(&[87u8; 40]).to_string());
        commit.set_username("Pencil".to_string());
        commit.set_email("pedosia@gmail.com".to_string());
        commit.set_message("Wotofak bitch ya molodoi legenda".to_string());
        commit.convert_to_bytes().unwrap();
    }

    #[test]
    fn commit_from_file() {
        let mut commit = Commit::new();
        commit.set_tree_sha(String::from_utf8_lossy(&[87u8; 40]).to_string());
        commit.set_username("Pencil".to_string());
        commit.set_email("pedosia@gmail.com".to_string());
        commit.set_message("Wotofak bitch ya molodoi legenda".to_string());

        let commit_bytes = commit.convert_to_bytes().unwrap();
        println!("Dumped: '{}'", std::str::from_utf8(&commit_bytes).unwrap());
        let hash = utils::generate_hash(&commit_bytes);

        let commit = Commit::from_raw_data(&commit_bytes).unwrap();
        let commit_bytes = commit.convert_to_bytes().unwrap();
        println!("Pumped: '{}'", std::str::from_utf8(&commit_bytes).unwrap());
        let hash2 = utils::generate_hash(&commit_bytes);
        assert_eq!(hash, hash2);
    }
}
