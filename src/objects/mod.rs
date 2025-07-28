pub mod blob;
pub mod commit;
pub mod tree;

pub trait ToFile {
    fn convert_to_bytes(&self) -> Vec<u8>;
}
