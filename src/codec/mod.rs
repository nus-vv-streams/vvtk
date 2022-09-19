use anyhow::Result;
use std::path::Path;

pub mod noop;

pub trait Decoder {
    fn new() -> Self;
    fn decode(&self, filename: &str) -> String;
    fn decode_folder(&self, foldername: &Path) -> Result<()>;
}
