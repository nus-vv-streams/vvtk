use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;

use crate::tool::{reader, renderer};
use std::path::PathBuf;

/// Structure representing a directory containing ply files
pub struct PlyDir {
    title: PathBuf,
    paths: Vec<Box<Path>>,
}

impl PlyDir {
    /// Creating a new `PlyDir`
    pub fn new(path: &str) -> Self {
        let mut entries = std::fs::read_dir(path)
            .unwrap()
            .map(|res| res.map(|e| e.path().into_boxed_path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        entries.sort();

        PlyDir {
            title: PathBuf::from(path),
            paths: entries,
        }
    }

    /// Return number of ply files
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    pub fn get_title(&self) -> Option<&str> {
        self.title.file_name().unwrap().to_str()
    }

    pub fn get_paths(self) -> Vec<Box<Path>> {
        self.paths
    }

    // pub fn write_to_ble(self, new_dir: &str) {
    //     std::fs::create_dir(new_dir).unwrap();

    //     for entry in self.paths {
    //         PlyFile::write_to_ble(entry, new_dir).unwrap();
    //     }
    // }
}
