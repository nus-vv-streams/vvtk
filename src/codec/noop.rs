use crate::codec::Decoder;
use std::path::Path;

use anyhow::{Error, Result};

pub struct NoopDecoder {}

impl Decoder for NoopDecoder {
    fn new() -> Self {
        NoopDecoder {}
    }

    fn decode(&self, filename: &str) -> String {
        filename.to_owned()
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
