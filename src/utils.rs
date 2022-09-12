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
