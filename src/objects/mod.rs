pub mod blob;
pub mod commit;
pub mod tree;

pub const SPACE_STR: &'static [u8] = b" ";

pub trait ObjectDump {
    fn convert_to_bytes(&self) -> Vec<u8>;
    fn dump_to_file(&self) -> anyhow::Result<String>;
}

pub trait ObjectPump: Sized {
    fn from_file(filepath: &str) -> anyhow::Result<Self>;
    fn from_data(data: &[u8]) -> anyhow::Result<Self>;
}
