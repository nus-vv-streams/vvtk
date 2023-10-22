use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use core::panic;
use std::path::Path;

use super::camera::CameraState;
use super::reader::{PointCloudFileReader, RenderReader};

pub struct AdaptiveReader {
    readers: Vec<PointCloudFileReader>,
    camera_state: Option<CameraState>,
}

fn infer_format(src: &String) -> String {
    let choices = ["pcd", "ply", "bin", "http"];
    const PCD: usize = 0;
    const PLY: usize = 1;
    const BIN: usize = 2;

    if choices.contains(&src.as_str()) {
        return src.clone();
    }

    let path = Path::new(src);
    // infer by counting extension numbers (pcd ply and bin)

    let mut choice_count = [0, 0, 0];
    for file_entry in path.read_dir().unwrap() {
        match file_entry {
            Ok(entry) => {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq("pcd") {
                        choice_count[PCD] += 1;
                    } else if ext.eq("ply") {
                        choice_count[PLY] += 1;
                    } else if ext.eq("bin") {
                        choice_count[BIN] += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}")
            }
        }
    }

    let max_index = choice_count
        .iter()
        .enumerate()
        .max_by_key(|(_, &item)| item)
        .map(|(index, _)| index);
    choices[max_index.unwrap()].to_string()
}

impl AdaptiveReader {
    pub fn new(src: &Vec<String>) -> Self {
        // TODO: remove the hard limit on the src len
        if src.len() != 1 && src.len() != 3 {
            panic!("src can only be of size 1 or 3")
        }

        let play_format = infer_format(&src[0]);
        let paths = src.iter().map(|s| Path::new(s)).collect::<Vec<_>>();

        // println!("Playing files in {:?} with format {}", path, play_format);
        let mut readers = vec![];

        for path in paths.iter() {
            readers.push(PointCloudFileReader::from_directory(path, &play_format));
        }

        if readers.is_empty() || readers[0].is_empty() {
            panic!("Must provide at least one file!");
        }

        let len = readers[0].len();
        for reader in readers.iter() {
            if reader.len() != len {
                panic!("All readers must have the same length");
            }
        }

        Self {
            readers,
            camera_state: None,
        }
    }

    pub fn len(&self) -> usize {
        self.readers[0].len()
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for AdaptiveReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.readers[0].start()
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.readers[0].get_at(index)
    }

    fn len(&self) -> usize {
        self.readers[0].len()
    }

    fn is_empty(&self) -> bool {
        self.readers[0].is_empty()
    }

    fn set_len(&mut self, len: usize) {
        for reader in self.readers.iter_mut() {
            reader.set_len(len);
        }
    }

    fn set_camera_state(&mut self, camera_state: Option<CameraState>) {
        self.camera_state = camera_state;
    }
}
