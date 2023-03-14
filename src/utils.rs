use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    pcd::read_pcd_file,
    ply::read_ply,
    render::wgpu::camera::CameraPosition,
};

use cgmath::{InnerSpace, Point3, Vector3};

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
    // camera-to-surface vector
    let x = (v_0 - pos).normalize();
    x.dot(norm.normalize())
}

/// Get the point of intersection on the plane from a point with vector and distance from line_pt to plane along line_vec
///
/// # Arguments
///
/// - `plane_pt`: The point on the plane
/// - `plane_norm`: The normal of the plane
/// - `line_pt`: The point on the line
/// - `line_vec`: The vector of the line
fn get_point_of_intersection_with_dist(
    plane_pt: Point3<f32>,
    plane_norm: Vector3<f32>,
    line_pt: Point3<f32>,
    line_vec: Vector3<f32>,
) -> Option<(Point3<f32>, f32)> {
    let dotprod = plane_norm.dot(line_vec);
    if dotprod == 0.0 {
        return None;
    }
    let d = (plane_pt - line_pt).dot(plane_norm) / dotprod;
    dbg!(line_pt + line_vec * d, d);
    Some((line_pt + line_vec * d, d))
}

#[rustfmt::skip]
/// Get the cosines from the camera to each of the six faces of a cube. Faces that are met first (from the perspective of pos) will have negative cosine value.
/// 
/// Assumption(14Mar23): the object has a cube-shaped bounding box, centered at the origin with side length 1.
pub fn get_cosines(pos: CameraPosition) -> Vec<f32> {
    let look_vector = Vector3 {
        x: pos.yaw.0.cos(),
        y: pos.pitch.0.sin(),
        z: pos.yaw.0.sin() + pos.pitch.0.cos(),
    }
    .normalize();

    let get_cosine_pair = |(v_0, norm_0): (Point3<f32>, Vector3<f32>),
                           (v_1, norm_1): (Point3<f32>, Vector3<f32>)|
     -> (f32, f32) {
        assert!(norm_0 + norm_1 == Vector3::new(0.0, 0.0, 0.0));
        let camera_pos = pos.position;
        let res_0 = get_point_of_intersection_with_dist(v_0, norm_0, camera_pos, look_vector);
        let res_1 = get_point_of_intersection_with_dist(v_1, norm_1, camera_pos, look_vector);
        if res_0.is_none() {
            // planes are parallel to look_vector
            (1.0, 1.0)
        } else {
            // Angles returned by `back_face_culling` abs() value is similar. 
            // The negative sign is assigned to the face that is in front of the other.
            // Why do we need to do this? Because if the point intersection is behind the camera, both faces will have the same cosine value.
            let (p_0, d_0) = res_0.unwrap();
            let (_, d_1) = res_1.unwrap();
            let c_0 = back_face_culling(camera_pos, p_0, norm_0);
            if c_0 < 0.0 && d_0 < d_1 || c_0 > 0.0 && d_0 > d_1 {
                (c_0, -c_0)
            } else {
                (-c_0, c_0)
            }
        }
    };

    let (left, right) = get_cosine_pair(
        (Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: -1.0, y: 0.0, z: 0.0 }), 
        (Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 1.0, y: 0.0, z: 0.0 }));
    let (bottom, top) = get_cosine_pair(
        (Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: 0.0, y: -1.0, z: 0.0 }), 
        (Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 0.0, y: 1.0, z: 0.0 }));
    let (back, front) = get_cosine_pair(
        (Point3 { x: -0.5, y: -0.5, z: -0.5 }, Vector3 { x: 0.0, y: 0.0, z: -1.0 }), 
        (Point3 { x: 0.5, y: 0.5, z: 0.5 }, Vector3 { x: 0.0, y: 0.0, z: 1.0 }));

    vec![left, bottom, back, right, top, front]
}

/// Predict the quality of the point cloud based on the geometry and attribute quality
pub fn predict_quality(geo_qp: f32, attr_qp: f32) -> f32 {
    2.292971443660981 - 0.0020313 * geo_qp + 0.20795236 * attr_qp - 0.00464757 * geo_qp * geo_qp
        + 0.00631909 * geo_qp * attr_qp
        - 0.00678052 * attr_qp * attr_qp
}
