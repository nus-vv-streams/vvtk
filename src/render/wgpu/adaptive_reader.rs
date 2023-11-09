use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use core::panic;
use std::path::Path;

use super::camera::CameraState;
use super::reader::{PointCloudFileReader, RenderReader};
use super::resolution_controller::ResolutionController;

pub struct AdaptiveReader {
    readers: Vec<PointCloudFileReader>,
    camera_state: Option<CameraState>,
    resolution_controller: ResolutionController,
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
            resolution_controller: ResolutionController::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.readers[0].len()
    }

    fn select_reader(&mut self, index: usize) -> &mut PointCloudFileReader {
        // if option is none, then we are in the first frame
        if self.camera_state.is_none() || self.readers.len() == 1 {
            return &mut self.readers[0];
        }

        let point_cloud = self.readers[index].get_at(index).unwrap();

        let desired_num_points = self.resolution_controller.get_desired_num_points(
            self.camera_state.as_ref().unwrap(),
            &point_cloud.points,
            point_cloud.number_of_points,
        );

        let num_points_by_reader = [388368, 509977, 834315];
        let resolution = self.find_resolution(&num_points_by_reader, desired_num_points);

        if resolution.is_none() {
            return &mut self.readers[0];
        }

        &mut self.readers[resolution.unwrap()]
    }

    fn find_resolution(&self, num_points: &[u64], desired_num_points: u64) -> Option<usize> {
        let size = num_points.len();
        if size == 0 {
            return None;
        }

        let mut left = 0;
        let mut right = size - 1;

        while left < right {
            let mid = left + (right - left) / 2;
            if num_points[mid] <= desired_num_points {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        Some(left.min(size - 1))
    }
}

impl RenderReader<PointCloud<PointXyzRgba>> for AdaptiveReader {
    fn start(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.select_reader(0).start()
    }

    fn get_at(&mut self, index: usize) -> Option<PointCloud<PointXyzRgba>> {
        self.select_reader(index).get_at(index)
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
