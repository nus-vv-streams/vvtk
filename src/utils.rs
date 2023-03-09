use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    pcd::read_pcd_file,
    ply::read_ply,
};

use cgmath::{Point3, Vector3};

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
    values: [i64; N],
    /// pointer to the next value to be overwritten
    next: usize,
    avg: i64,
    divide_by: i64,
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
    /// Ignores if datapoint is 0.
    pub fn add(&mut self, value: i64) {
        if value == 0 {
            return;
        }
        self.avg += (value - self.values[self.next]) / self.divide_by;
        self.values[self.next] = value;
        self.next = (self.next + 1) % N;
        self.divide_by = std::cmp::min(self.divide_by + 1, N as i64);
    }

    /// Gets the running average
    pub fn get(&self) -> i64 {
        self.avg
    }
}

// https://en.wikipedia.org/wiki/Back-face_culling
///
/// Returns the cosine of the angle between the vector from the camera to the point and the normal of the triangle.
///
/// # Arguments
///
/// - `pos`: The position of the camera
/// - `v_0`: The position of a vertex of the surface
/// - `norm`: The surface normal
fn back_face_culling(pos: Point3<f32>, v_0: Point3<f32>, norm: Vector3<f32>) -> f32 {
    use cgmath::InnerSpace;
    // camera-to-surface vector
    let x = (v_0 - pos).normalize();
    x.dot(norm.normalize())
}

/// Get the cosines from the camera to each of the six faces of a cube
/// assuming the cube is centered at the origin with side length 1.
#[rustfmt::skip]
pub fn get_cosines(pos: Point3<f32>) -> Vec<f32> {
    vec![
        // Left,
        back_face_culling(pos, Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: -1.0, y: 0.0, z: 0.0 }),
        // Bottom,
        back_face_culling(pos, Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: 0.0, y: -1.0, z: 0.0 }),
        // Back,
        back_face_culling(pos, Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: 0.0, y: 0.0, z: -1.0 }),
        // Right,
        back_face_culling(pos, Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 1.0, y: 0.0, z: 0.0 }),
        // Top,
        back_face_culling(pos, Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 0.0, y: 1.0, z: 0.0 }),
        // Front,
        back_face_culling(pos, Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 0.0, y: 0.0, z: 1.0 }),
    ]
}
