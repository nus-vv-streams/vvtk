use anyhow::Result;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub mod decoder;

pub trait Decoder {
    fn decode(&self, filename: &OsStr) -> Vec<PathBuf>;
    fn decode_folder(&self, foldername: &Path) -> Result<()>;
}
