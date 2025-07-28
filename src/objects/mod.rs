pub mod blob;
pub mod commit;
pub mod tree;

pub trait ObjectDump {
    fn convert_to_bytes(&self) -> Vec<u8>;
    fn dump_to_file(&self) -> anyhow::Result<String>;
}
