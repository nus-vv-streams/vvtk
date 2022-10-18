use crate::codec::Decoder;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Error, Result};

pub struct NoopDecoder {}

impl NoopDecoder {
    pub fn new() -> Self {
        return NoopDecoder {};
    }
}

impl Decoder for NoopDecoder {
    fn decode(&self, filename: &OsStr) -> Vec<PathBuf> {
        vec![PathBuf::from(filename)]
    }

    fn decode_folder(&self, directory: &Path) -> Result<()> {
        let path = Path::new(directory);
        for file_entry in path.read_dir().unwrap() {
            match file_entry {
                Ok(_entry) => {}
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(())
    }
}

pub struct DracoDecoder {
    path: PathBuf,
}

impl DracoDecoder {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        DracoDecoder { path: path.into() }
    }
}

impl Decoder for DracoDecoder {
    // filename must be full path
    fn decode(&self, filename: &OsStr) -> Vec<PathBuf> {
        let mut output_filename = PathBuf::from(filename);
        output_filename.set_extension("ply");
        let status = Command::new(&self.path)
            .arg("-i")
            .arg(filename)
            .arg("-o")
            .arg(output_filename.to_str().unwrap())
            .status();
        if status.is_ok() {
            vec![output_filename]
        } else {
            vec![]
        }
    }

    fn decode_folder(&self, directory: &Path) -> Result<()> {
        let path = Path::new(directory);
        for file_entry in path.read_dir().unwrap() {
            match file_entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        self.decode(path.into_os_string().as_os_str());
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(())
    }
}
