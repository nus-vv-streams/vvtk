use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    pcd::read_pcd_file,
    ply::read_ply,
};

pub fn read_file_to_point_cloud(file: &PathBuf) -> Option<PointCloud<PointXyzRgba>> {
    if let Some(ext) = file.extension().and_then(|ext| ext.to_str()) {
        let point_cloud = match ext {
            "ply" => read_ply(file),
            "pcd" => read_pcd_file(file).map(PointCloud::from).ok(),
            _ => None,
        };
        return point_cloud;
    }
    None
}

pub fn find_all_files(os_strings: &Vec<OsString>) -> Vec<PathBuf> {
    let mut files_to_convert = vec![];
    for file_str in os_strings {
        let path = Path::new(&file_str);
        if path.is_dir() {
            files_to_convert.extend(expand_directory(path));
        } else {
            files_to_convert.push(path.to_path_buf());
        }
    }
    files_to_convert
}

pub fn expand_directory(p: &Path) -> Vec<PathBuf> {
    let mut ply_files = vec![];
    let dir_entry = p.read_dir().unwrap();
    for entry in dir_entry {
        let entry = entry.unwrap().path();
        if !entry.is_file() {
            // We do not recursively search
            continue;
        }
        ply_files.push(entry);
    }

    ply_files
}

#[derive(Clone, Debug)]
pub struct SimpleRunningAverage<const N: usize> {
    values: [usize; N],
    /// pointer to the next value to be overwritten
    next: usize,
    avg: usize,
    divide_by: usize,
}

impl<const N: usize> SimpleRunningAverage<N> {
    pub fn new() -> Self {
        SimpleRunningAverage {
            values: [0; N],
            next: 0,
            avg: 0,
            divide_by: 1,
        }
    }

    /// Adds a new datapoint to the running average, removing the oldest
    pub fn add(&mut self, value: usize) {
        self.avg = self.avg + (value - self.values[self.next as usize]) / self.divide_by;
        self.values[self.next as usize] = value;
        self.next = (self.next + 1) % N;
        self.divide_by = std::cmp::min(self.divide_by + 1, N);
    }

    /// Gets the running average
    pub fn get(&self) -> usize {
        self.avg
    }
}
