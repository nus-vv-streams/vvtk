use crate::{
    formats::{pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal, PointCloud},
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

pub fn pcd_to_ply_from_data_normal(
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
    let nx_prop_def = ply_rs::ply::PropertyDef::new(
        "nx".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
    );
    let ny_prop_def = ply_rs::ply::PropertyDef::new(
        "ny".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
    );
    let nz_prop_def = ply_rs::ply::PropertyDef::new(
        "nz".to_string(),
        ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float),
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
    element.properties.insert("nx".to_string(), nx_prop_def);
    element.properties.insert("ny".to_string(), ny_prop_def);
    element.properties.insert("nz".to_string(), nz_prop_def);
    element.count = pcd.header().width() as usize;

    let mut ply_header = ply_rs::ply::Header::new();
    ply_header.encoding = match storage_type {
        PCDDataType::Ascii => ply_rs::ply::Encoding::Ascii,
        PCDDataType::Binary => set_encoding(),
        _ => unreachable!(),
    };
    ply_header.elements.insert("vertex".to_string(), element);

    let pcd_pointxyzrgbanormal: PointCloud<PointXyzRgbaNormal> = pcd.into();
    let mut pay_load_vec = Vec::<DefaultElement>::new();
    pcd_pointxyzrgbanormal.points.into_iter().for_each(|point| {
        let mut ply_point = DefaultElement::new();
        ply_point.insert("x".to_string(), ply_rs::ply::Property::Float(point.x));
        ply_point.insert("y".to_string(), ply_rs::ply::Property::Float(point.y));
        ply_point.insert("z".to_string(), ply_rs::ply::Property::Float(point.z));
        ply_point.insert("red".to_string(), ply_rs::ply::Property::UChar(point.r));
        ply_point.insert("green".to_string(), ply_rs::ply::Property::UChar(point.g));
        ply_point.insert("blue".to_string(), ply_rs::ply::Property::UChar(point.b));
        ply_point.insert("nx".to_string(), ply_rs::ply::Property::Float(point.nx));
        ply_point.insert("ny".to_string(), ply_rs::ply::Property::Float(point.ny));
        ply_point.insert("nz".to_string(), ply_rs::ply::Property::Float(point.nz));
        pay_load_vec.push(ply_point);
    });
    let mut pay_load = Payload::<DefaultElement>::new();
    pay_load.insert("vertex".to_string(), pay_load_vec);

    let mut ply = ply_rs::ply::Ply::<DefaultElement>::new();
    ply.header = ply_header;
    ply.payload = pay_load;

    let dir = output_path.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir_all(dir).unwrap();
    }

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
mod test {

    use super::*;

    #[test]
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

    #[test]
    fn test_padding() {
        let x = 101;
        let y = 4;
        let mut files = vec![];
        for i in 0..=x {
            let filename = format!("{:0width$}.pcd", i, width = y);
            // println!("{}", filename);
            files.push(filename);
        }
        files.sort();
        println!("{:?}", files);
    }
}
