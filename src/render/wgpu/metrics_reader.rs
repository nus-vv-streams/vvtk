use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::{metrics::Metrics, utils::expand_directory};

pub struct MetricsReader {
    files: Vec<PathBuf>,
}

impl MetricsReader {
    pub fn from_directory(directory: &Path) -> Self {
        let files = expand_directory(directory);
        Self { files }
    }

    fn file_at(&self, index: usize) -> Option<&PathBuf> {
        self.files.get(index)
    }

    pub fn get_at(&self, index: usize) -> Option<Metrics> {
        self.file_at(index)
            .map(|f| {
                File::open(f).unwrap_or_else(|_| panic!("Failed to open file {:?}", f.as_os_str()))
            })
            .map(|mut f| Metrics::from_reader(&mut f))
    }
}
