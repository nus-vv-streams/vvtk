use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    pcd::{create_pcd, read_pcd_file, write_pcd_file, PCDDataType, PointCloudData},
    ply::read_ply,
    velodyne::read_velodyn_bin_file,
};
use ply_rs::{
    parser, ply,
    ply::DefaultElement,
    ply::{Encoding, Payload},
    writer,
};
use std::fs::File;
use std::str::FromStr;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    dash::{ThroughputPrediction, ViewportPrediction},
};

#[cfg(feature = "render")]
use crate::render::wgpu::camera::CameraPosition;

use cgmath::{InnerSpace, Point3, Vector3};

pub fn read_file_to_point_cloud(file: &PathBuf) -> Option<PointCloud<PointXyzRgba>> {
    if let Some(ext) = file.extension().and_then(|ext| ext.to_str()) {
        let point_cloud = match ext {
            "ply" => read_ply(file),
            "pcd" => read_pcd_file(file).map(PointCloud::from).ok(),
            "bin" => read_velodyn_bin_file(file).map(PointCloud::from).ok(),
            _ => None,
        };
        return point_cloud;
    }
    None
}

fn check_files_existence(files: &Vec<OsString>) -> bool {
    let mut flag = true;
    for file_str in files {
        let path = Path::new(&file_str);
        if !path.exists() {
            println!("File {:?} does not exist", path);
            flag = false;
        }
    }
    flag
}

pub fn find_all_files(os_strings: &Vec<OsString>) -> Vec<PathBuf> {
    if !check_files_existence(os_strings) {
        panic!("Some files do not exist")
    }
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
        // ignore file start with .
        if entry
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with('.')
        {
            continue;
        }
        ply_files.push(entry);
    }

    ply_files
}

/// return last value recorded
pub struct LastValue<T> {
    last_value: Option<T>,
}

impl<T> LastValue<T> {
    pub fn new() -> Self {
        LastValue { last_value: None }
    }
}

impl ThroughputPrediction for LastValue<f64> {
    fn add(&mut self, value: f64) {
        self.last_value = Some(value);
    }

    fn predict(&self) -> Option<f64> {
        self.last_value
    }
}

impl ViewportPrediction for LastValue<CameraPosition> {
    fn add(&mut self, value: CameraPosition) {
        self.last_value = Some(value);
    }

    fn predict(&self) -> Option<CameraPosition> {
        self.last_value
    }
}

#[derive(Clone, Debug)]
/// returns the average of the last N values
pub struct SimpleRunningAverage<T, const N: usize> {
    values: [T; N],
    /// pointer to the next value to be overwritten
    next: usize,
    avg: Option<T>,
    divide_by: usize,
}

impl<T, const N: usize> SimpleRunningAverage<T, N>
where
    T: Default + Copy,
{
    pub fn new() -> Self {
        SimpleRunningAverage {
            values: [T::default(); N],
            next: 0,
            avg: None,
            divide_by: 0,
        }
    }
}

impl<const N: usize> ThroughputPrediction for SimpleRunningAverage<f64, N> {
    /// Adds a new datapoint to the running average, removing the oldest
    /// Ignores if datapoint is 0.
    fn add(&mut self, value: f64) {
        if value == 0.0 {
            return;
        }
        self.avg = Some(
            (self.avg.unwrap_or_default() * self.divide_by as f64
                + (value - self.values[self.next]))
                / std::cmp::min(N, self.divide_by + 1) as f64,
        );
        self.values[self.next] = value;
        self.next = (self.next + 1) % N;
        self.divide_by = std::cmp::min(self.divide_by + 1, N);
    }

    fn predict(&self) -> Option<f64> {
        self.avg
    }
}

pub struct ExponentialMovingAverage<T> {
    last_value: Option<T>,
    alpha: T,
    last_prediction: Option<T>,
}

impl<T> ExponentialMovingAverage<T>
where
    T: Copy + Default,
{
    pub fn new(alpha: T) -> Self {
        ExponentialMovingAverage {
            last_value: None,
            alpha,
            last_prediction: None,
        }
    }
}

impl ThroughputPrediction for ExponentialMovingAverage<f64> {
    /// Adds a new datapoint to the running average, removing the oldest
    /// Ignores if datapoint is 0.
    fn add(&mut self, value: f64) {
        self.last_value = Some(value);
        let pred = self
            .last_prediction
            .map(|last_pred| last_pred * (1.0 - self.alpha) + self.alpha * value)
            .unwrap_or(value);
        self.last_prediction = Some(pred);
    }

    /// Predicts the running average
    fn predict(&self) -> Option<f64> {
        self.last_prediction
    }
}

/// Gradient Adaptive Exponential Moving Average
pub struct GAEMA<T> {
    last_value: Option<T>,
    last_last_value: Option<T>,
    /// pointer to the next value to be overwritten
    count: usize,
    last_alpha: T,
    last_prediction: Option<T>,
    alltime_average: T,
}

impl<T> GAEMA<T>
where
    T: Copy + Default,
{
    pub fn new(alpha: T) -> Self {
        GAEMA {
            last_value: None,
            last_last_value: None,
            count: 0,
            last_alpha: alpha,
            last_prediction: None,
            alltime_average: T::default(),
        }
    }
}

impl ThroughputPrediction for GAEMA<f64> {
    fn add(&mut self, value: f64) {
        self.last_last_value = self.last_value;
        self.last_value = Some(value);
        self.count += 1;
        self.alltime_average =
            (self.alltime_average * (self.count - 1) as f64 + value) / self.count as f64;

        let m_inst_i =
            (self.last_value.unwrap_or_default() - self.last_last_value.unwrap_or_default()).abs();
        let m_norm_i = self.alltime_average / (self.count as f64 + 1.0e-10);
        let alpha = self.last_alpha.powf(m_norm_i / m_inst_i);
        self.last_alpha = alpha;
        let pred = self
            .last_prediction
            .map(|last_pred| last_pred * (1.0 - alpha) + alpha * value)
            .unwrap_or(value);
        self.last_prediction = Some(pred);
    }

    fn predict(&self) -> Option<f64> {
        self.last_prediction
    }
}

/// Low Pass Exponential Moving Average
pub struct LPEMA<T> {
    last_value: Option<T>,
    last_last_value: Option<T>,
    /// pointer to the next value to be overwritten
    count: usize,
    last_alpha: T,
    last_prediction: Option<T>,
    alltime_average: T,
}

impl<T> LPEMA<T>
where
    T: Copy + Default,
{
    pub fn new(alpha: T) -> Self {
        LPEMA {
            last_value: None,
            last_last_value: None,
            count: 0,
            last_alpha: alpha,
            last_prediction: None,
            alltime_average: T::default(),
        }
    }
}

impl ThroughputPrediction for LPEMA<f64> {
    fn add(&mut self, value: f64) {
        self.last_last_value = self.last_value;
        self.last_value = Some(value);
        self.count += 1;
        self.alltime_average =
            (self.alltime_average * (self.count - 1) as f64 + value) / self.count as f64;

        let m_inst_i =
            (self.last_value.unwrap_or_default() - self.last_last_value.unwrap_or_default()).abs();
        let m_norm_i = self.alltime_average / (self.count as f64 + 1.0e-10);
        let alpha = 1.0 / (1.0 + m_inst_i / m_norm_i);
        self.last_alpha = alpha;
        let pred = self
            .last_prediction
            .map(|last_pred| last_pred * (1.0 - alpha) + alpha * value)
            .unwrap_or(value);
        self.last_prediction = Some(pred);
    }

    fn predict(&self) -> Option<f64> {
        self.last_prediction
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
    Some((line_pt + line_vec * d, d))
}

#[rustfmt::skip]
#[cfg(feature = "render")]
/// Get the cosines from the camera to each of the six faces of a cube. Faces that are met first (from the perspective of pos) will have negative cosine value.
/// 
/// Assumption(14Mar23): the object has a cube-shaped bounding box, centered at the origin with side length 1.
pub fn get_cosines(pos: CameraPosition) -> Vec<f32> {
    use log::debug;

    let look_vector = Vector3 {
        x: pos.yaw.0.cos(),
        y: pos.pitch.0.sin(),
        z: pos.yaw.0.sin() + pos.yaw.0.sin().signum() * pos.pitch.0.cos(),
    }
    .normalize();
    debug!("look_vector: {:?}, camera_pos: {:?}", Vector3 {
        x: pos.yaw.0.cos(),
        y: pos.pitch.0.sin(),
        z: pos.yaw.0.sin() + pos.yaw.0.sin().signum() * pos.pitch.0.cos(),
    }, pos);

    let get_cosine_pair = |(v_0, norm_0): (Point3<f32>, Vector3<f32>),
                           (v_1, norm_1): (Point3<f32>, Vector3<f32>)|
     -> (f32, f32) {
        assert!(norm_0 + norm_1 == Vector3::new(0.0, 0.0, 0.0));
        let camera_pos = pos.position;
        let res_0 = get_point_of_intersection_with_dist(v_0, norm_0, camera_pos, look_vector);
        let res_1 = get_point_of_intersection_with_dist(v_1, norm_1, camera_pos, look_vector);
        if let Some((p_0, d_0)) = res_0 {
            // Angles returned by `back_face_culling` abs() value is similar. 
            // The negative sign is assigned to the face that is in front of the other.
            // Why do we need to do this? Because if the point intersection is behind the camera, both faces will have the same cosine value.
            let (_, d_1) = res_1.unwrap();
            let c_0 = back_face_culling(camera_pos, p_0, norm_0);
            if c_0 < 0.0 && d_0 < d_1 || c_0 > 0.0 && d_0 > d_1 {
                (c_0, -c_0)
            } else {
                (-c_0, c_0)
            }
        } else {            // planes are parallel to look_vector
            (1.0, 1.0)
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
    2.292_971_4 - 0.0020313 * geo_qp + 0.20795236 * attr_qp - 0.00464757 * geo_qp * geo_qp
        + 0.00631909 * geo_qp * attr_qp
        - 0.00678052 * attr_qp * attr_qp
}

pub fn ply_to_ply(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let ply_parser = parser::Parser::<ply::DefaultElement>::new();
    let mut f = std::fs::File::open(&file_path).unwrap();
    let mut ply = ply_parser.read_ply(&mut f).unwrap();

    ply.header.encoding = match storage_type {
        PCDDataType::Ascii => ply_rs::ply::Encoding::Ascii,
        PCDDataType::Binary => set_encoding(),
        _ => unreachable!(),
    };

    let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
    let output_file = output_path.join(filename);
    let mut file = File::create(&output_file).unwrap();

    let ply_writer = writer::Writer::<ply::DefaultElement>::new();
    if let Err(e) = ply_writer.write_ply(&mut file, &mut ply) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }
}

pub fn pcd_to_pcd(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let pcd = read_pcd_file(file_path.clone()).unwrap();
    create_file_write_pcd_helper(&pcd, output_path, storage_type, file_path);
}

pub fn create_file_write_pcd_helper(
    pcd: &PointCloudData,
    output_path: &Path,
    storage_type: PCDDataType,
    file_path: PathBuf,
) {
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
    let output_file = output_path.join(filename);
    if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }
}

pub fn ply_to_pcd(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let pointxyzrgba = read_ply(file_path.clone()).unwrap();
    let pcd = create_pcd(&pointxyzrgba);
    create_file_write_pcd_helper(&pcd, output_path, storage_type, file_path);
}

pub fn pcd_to_ply_from_data(
    output_path: &Path,
    storage_type: PCDDataType,
    pcd: PointCloudData,
) -> Result<(), Box<dyn std::error::Error>> {
    let x_prop_def = ply_rs::ply::PropertyDef::new(
        "x".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
    );
    let y_prop_def = ply_rs::ply::PropertyDef::new(
        "y".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
    );
    let z_prop_def = ply_rs::ply::PropertyDef::new(
        "z".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
    );
    let red_prop_def = ply_rs::ply::PropertyDef::new(
        "red".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar),
    );
    let green_prop_def = ply_rs::ply::PropertyDef::new(
        "green".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar),
    );
    let blue_prop_def = ply_rs::ply::PropertyDef::new(
        "blue".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar),
    );

    let mut element = ply_rs::ply::ElementDef::new("vertex".to_string());
    element.properties.insert("x".to_string(), x_prop_def);
    element.properties.insert("y".to_string(), y_prop_def);
    element.properties.insert("z".to_string(), z_prop_def);
    element.properties.insert("red".to_string(), red_prop_def);
    element
        .properties
        .insert("green".to_string(), green_prop_def);
    element.properties.insert("blue".to_string(), blue_prop_def);
    element.count = pcd.header().width() as usize;

    let mut ply_header = ply_rs::ply::Header::new();
    ply_header.encoding = match storage_type {
        PCDDataType::Ascii => ply_rs::ply::Encoding::Ascii,
        PCDDataType::Binary => set_encoding(),
        _ => unreachable!(),
    };
    ply_header.elements.insert("vertex".to_string(), element);

    let pcd_pointxyzrgba: PointCloud<PointXyzRgba> = pcd.into();
    let mut pay_load_vec = Vec::<DefaultElement>::new();
    pcd_pointxyzrgba.points.into_iter().for_each(|point| {
        let mut ply_point = DefaultElement::new();
        ply_point.insert("x".to_string(), ply_rs::ply::Property::Float(point.x));
        ply_point.insert("y".to_string(), ply_rs::ply::Property::Float(point.y));
        ply_point.insert("z".to_string(), ply_rs::ply::Property::Float(point.z));
        ply_point.insert("red".to_string(), ply_rs::ply::Property::UChar(point.r));
        ply_point.insert("green".to_string(), ply_rs::ply::Property::UChar(point.g));
        ply_point.insert("blue".to_string(), ply_rs::ply::Property::UChar(point.b));
        pay_load_vec.push(ply_point);
    });
    let mut pay_load = Payload::<DefaultElement>::new();
    pay_load.insert("vertex".to_string(), pay_load_vec);

    let mut ply = ply_rs::ply::Ply::<DefaultElement>::new();
    ply.header = ply_header;
    ply.payload = pay_load;

    // println!("Writing to {:?}", output_path);
    // get dir part and check existence, create if not exist
    let dir = output_path.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir_all(dir).unwrap();
    }

    println!("Writing to {:?}", output_path);
    let mut file = File::create(output_path).unwrap();

    let ply_writer = writer::Writer::<ply::DefaultElement>::new();
    if let Err(e) = ply_writer.write_ply(&mut file, &mut ply) {
        Result::Err(Box::new(e))
    } else {
        Result::Ok(())
    }
}

pub fn pcd_to_ply(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let pcd = read_pcd_file(&file_path).unwrap();
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
    let output_file = output_path.join(filename);
    if let Err(e) = pcd_to_ply_from_data(&output_file, storage_type, pcd) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.to_str(),
        );
    }
}

pub fn velodyne_bin_to_ply(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let vbd = read_velodyn_bin_file(&file_path).unwrap();
    let pc: PointCloud<PointXyzRgba> = vbd.into();
    let pcd: PointCloudData = create_pcd(&pc);
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
    let output_file = output_path.join(filename);
    if let Err(e) = pcd_to_ply_from_data(&output_file, storage_type, pcd) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.to_str(),
        );
    }
}

pub fn velodyne_bin_to_pcd(output_path: &Path, storage_type: PCDDataType, file_path: PathBuf) {
    let vbd = read_velodyn_bin_file(&file_path).unwrap();
    let pointxyzrgba: PointCloud<PointXyzRgba> = vbd.into();
    let pcd: PointCloudData = create_pcd(&pointxyzrgba);
    create_file_write_pcd_helper(&pcd, output_path, storage_type, file_path);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConvertOutputFormat {
    PLY,
    PCD,
    PNG,
    MP4,
}

impl ToString for ConvertOutputFormat {
    fn to_string(&self) -> String {
        match self {
            ConvertOutputFormat::PLY => "ply",
            ConvertOutputFormat::PCD => "pcd",
            ConvertOutputFormat::PNG => "png",
            ConvertOutputFormat::MP4 => "mp4",
        }
        .to_string()
    }
}

impl FromStr for ConvertOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ply" => Ok(ConvertOutputFormat::PLY),
            "pcd" => Ok(ConvertOutputFormat::PCD),
            "png" => Ok(ConvertOutputFormat::PNG),
            "mp4" => Ok(ConvertOutputFormat::MP4),
            _ => Err(format!("{} is not a valid output format", s)),
        }
    }
}

#[cfg(target_endian = "little")]
fn set_encoding() -> Encoding {
    Encoding::BinaryLittleEndian
}

#[cfg(target_endian = "big")]
fn set_encoding() -> Encoding {
    Encoding::BinaryBigEndian
}


#[cfg(test)]
mod tests {
    use super::*;
    const EPSILON: f64 = 0.0001;

    #[test]
    fn test_simple_running_avg() {
        let mut avg = SimpleRunningAverage::<f64, 3>::new();
        assert_eq!(avg.predict(), None);
        avg.add(1.0);
        assert!((avg.predict().unwrap() - 1.0).abs() < EPSILON);
        avg.add(2.0);
        assert!((avg.predict().unwrap() - 1.5).abs() < EPSILON);
        avg.add(2.0);
        assert!((avg.predict().unwrap() - 1.66666667).abs() < EPSILON);
        avg.add(3.0);
        assert!((avg.predict().unwrap() - 2.33333333).abs() < EPSILON);
        avg.add(5.0);
        avg.add(10.0);
        assert!((avg.predict().unwrap() - 6.0).abs() < EPSILON);
        avg.add(7.0);
        assert!((avg.predict().unwrap() - 7.33333333).abs() < EPSILON);
    }

    #[test]
    fn test_ema() {
        let mut ema = ExponentialMovingAverage::new(0.1);
        assert_eq!(ema.predict(), None);
        ema.add(1.0);
        assert!((ema.predict().unwrap() - 1.0).abs() < EPSILON);
        ema.add(2.0);
        assert!((ema.predict().unwrap() - 1.1).abs() < EPSILON);
        ema.add(2.0);
        assert!((ema.predict().unwrap() - 1.19).abs() < EPSILON);
        ema.add(3.0);
        assert!((ema.predict().unwrap() - 1.371).abs() < EPSILON);
        ema.add(5.0);
        ema.add(10.0);
        assert!((ema.predict().unwrap() - 2.56051).abs() < EPSILON);
        ema.add(7.0);
        assert!((ema.predict().unwrap() - 3.004459).abs() < EPSILON);
        assert!((ema.predict().unwrap() - 3.004459).abs() < EPSILON);
    }

    // #[test]
    // fn test_gaema() {
    //     let mut gaema = GAEMA::new(0.1);
    //     assert_eq!(gaema.predict(), None);
    //     gaema.add(1.0);
    //     assert!((gaema.predict().unwrap() - 1.0).abs() < EPSILON);
    //     gaema.add(2.0);
    //     assert!((gaema.predict().unwrap() - 1.177828).abs() < EPSILON);
    //     gaema.add(2.0);
    //     assert!((gaema.predict().unwrap() - 1.177880).abs() < EPSILON);
    //     gaema.add(3.0);
    //     assert!((gaema.predict().unwrap() - 2.087109).abs() < EPSILON);
    //     gaema.add(5.0);
    //     gaema.add(10.0);
    //     assert!((gaema.predict().unwrap() - 8.662998).abs() < EPSILON);
    //     gaema.add(7.0);
    //     assert!((gaema.predict().unwrap() - 7.061845).abs() < EPSILON);
    // }

    #[test]
    fn test_gaema() {
        let mut lpema = LPEMA::new(0.1);
        assert_eq!(lpema.predict(), None);
        lpema.add(1.0);
        assert!((lpema.predict().unwrap() - 1.0).abs() < EPSILON);
        lpema.add(2.0);
        assert!((lpema.predict().unwrap() - 1.428571).abs() < EPSILON);
        lpema.add(2.0);
        assert!((lpema.predict().unwrap() - 2.0).abs() < EPSILON);
        lpema.add(3.0);
        assert!((lpema.predict().unwrap() - 2.333333).abs() < EPSILON);
        lpema.add(5.0);
        lpema.add(10.0);
        assert!((lpema.predict().unwrap() - 3.689890).abs() < EPSILON);
        lpema.add(7.0);
        assert!((lpema.predict().unwrap() - 4.250925).abs() < EPSILON);
    }

    fn test_read_ply() {
        let ply_ascii_path = PathBuf::from("./test_files/ply_ascii/longdress_vox10_1213_short.ply");
        let pc = read_ply(&ply_ascii_path).unwrap();
        assert_eq!(pc.number_of_points, 20);
        assert_eq!(
            pc.points[0],
            PointXyzRgba {
                x: 171.0,
                y: 63.0,
                z: 255.0,
                r: 183,
                g: 165,
                b: 155,
                a: 255
            }
        );
        assert_eq!(
            pc.points[19],
            PointXyzRgba {
                x: 175.0,
                y: 60.0,
                z: 253.0,
                r: 161,
                g: 145,
                b: 133,
                a: 255
            }
        );
    }

    #[test]
    fn test_ply_to_ply() {
        let ply_ascii_path = PathBuf::from("./test_files/ply_ascii/longdress_vox10_1213_short.ply");
        let output_path = PathBuf::from("./test_files/ply_binary");
        ply_to_ply(&output_path, PCDDataType::Binary, ply_ascii_path);
        let output_path = output_path.join("longdress_vox10_1213_short.ply");
        let pc = read_file_to_point_cloud(&output_path).unwrap();
        assert_eq!(pc.number_of_points, 20);
        assert_eq!(
            pc.points[0],
            PointXyzRgba {
                x: 171.0,
                y: 63.0,
                z: 255.0,
                r: 183,
                g: 165,
                b: 155,
                a: 255
            }
        );
        assert_eq!(
            pc.points[9],
            PointXyzRgba {
                x: 172.0,
                y: 61.0,
                z: 255.0,
                r: 161,
                g: 145,
                b: 134,
                a: 255
            }
        );
        assert_eq!(
            pc.points[19],
            PointXyzRgba {
                x: 175.0,
                y: 60.0,
                z: 253.0,
                r: 161,
                g: 145,
                b: 133,
                a: 255
            }
        );
    }

    #[test]
    fn test_ply_to_pcd() {
        let ply_ascii_path = PathBuf::from("./test_files/ply_ascii/longdress_vox10_1213_short.ply");
        let output_path = PathBuf::from("./test_files/pcd_binary");
        ply_to_pcd(&output_path, PCDDataType::Binary, ply_ascii_path.clone());
        let output_path = output_path.join("longdress_vox10_1213_short.pcd");
        let pc = read_file_to_point_cloud(&output_path).unwrap();
        assert_eq!(pc.number_of_points, 20);
        assert_eq!(
            pc.points[0],
            PointXyzRgba {
                x: 171.0,
                y: 63.0,
                z: 255.0,
                r: 183,
                g: 165,
                b: 155,
                a: 255
            }
        );
        assert_eq!(
            pc.points[9],
            PointXyzRgba {
                x: 172.0,
                y: 61.0,
                z: 255.0,
                r: 161,
                g: 145,
                b: 134,
                a: 255
            }
        );
        assert_eq!(
            pc.points[19],
            PointXyzRgba {
                x: 175.0,
                y: 60.0,
                z: 253.0,
                r: 161,
                g: 145,
                b: 133,
                a: 255
            }
        );

        let output_path = PathBuf::from("./test_files/pcd_ascii");
        ply_to_pcd(&output_path, PCDDataType::Ascii, ply_ascii_path);
        let output_path = output_path.join("longdress_vox10_1213_short.pcd");
        let pc = read_file_to_point_cloud(&output_path).unwrap();
        assert_eq!(pc.number_of_points, 20);
        assert_eq!(
            pc.points[0],
            PointXyzRgba {
                x: 171.0,
                y: 63.0,
                z: 255.0,
                r: 183,
                g: 165,
                b: 155,
                a: 255
            }
        );
        assert_eq!(
            pc.points[9],
            PointXyzRgba {
                x: 172.0,
                y: 61.0,
                z: 255.0,
                r: 161,
                g: 145,
                b: 134,
                a: 255
            }
        );
        assert_eq!(
            pc.points[19],
            PointXyzRgba {
                x: 175.0,
                y: 60.0,
                z: 253.0,
                r: 161,
                g: 145,
                b: 133,
                a: 255
            }
        );
    }

    #[test]
    fn test_pcd_to_ply() {
        let pcd_ascii_path = PathBuf::from("./test_files/pcd_ascii/longdress_vox10_1213_short.pcd");
        let output_path = PathBuf::from("./test_files/ply_ascii/from_pcd");
        pcd_to_ply(&output_path, PCDDataType::Ascii, pcd_ascii_path);
        let output_path = output_path.join("longdress_vox10_1213_short.ply");
        let pc = read_file_to_point_cloud(&output_path).unwrap();
        assert_eq!(pc.number_of_points, 20);
        assert_eq!(
            pc.points[0],
            PointXyzRgba {
                x: 171.0,
                y: 63.0,
                z: 255.0,
                r: 183,
                g: 165,
                b: 155,
                a: 255
            }
        );
        assert_eq!(
            pc.points[9],
            PointXyzRgba {
                x: 172.0,
                y: 61.0,
                z: 255.0,
                r: 161,
                g: 145,
                b: 134,
                a: 255
            }
        );
        assert_eq!(
            pc.points[19],
            PointXyzRgba {
                x: 175.0,
                y: 60.0,
                z: 253.0,
                r: 161,
                g: 145,
                b: 133,
                a: 255
            }
        );
    }
}
