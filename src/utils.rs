use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    pcd::{read_pcd_file, PCDDataType, create_pcd, write_pcd_file},
    ply::read_ply,
};
use ply_rs::{ply, parser, writer, ply::{Encoding, Payload}, ply::DefaultElement};
use std::fs::File;



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



pub fn ply_to_ply(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
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

pub fn pcd_to_pcd(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pcd = read_pcd_file(file_path.clone()).unwrap();
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

pub fn ply_to_pcd(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pointxyzrgba = read_ply(file_path.clone()).unwrap();
    let pcd = create_pcd(&pointxyzrgba);

    let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
    let output_file = output_path.join(filename.clone());
    if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }
}


pub fn pcd_to_ply(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pcd = read_pcd_file(&file_path).unwrap();

    let x_prop_def = ply_rs::ply::PropertyDef::new("x".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let y_prop_def = ply_rs::ply::PropertyDef::new("y".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let z_prop_def = ply_rs::ply::PropertyDef::new("z".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let red_prop_def = ply_rs::ply::PropertyDef::new("red".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    let green_prop_def = ply_rs::ply::PropertyDef::new("green".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    let blue_prop_def = ply_rs::ply::PropertyDef::new("blue".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    
    let mut element = ply_rs::ply::ElementDef::new("vertex".to_string());
    element.properties.insert("x".to_string(), x_prop_def);
    element.properties.insert("y".to_string(), y_prop_def);
    element.properties.insert("z".to_string(), z_prop_def);
    element.properties.insert("red".to_string(), red_prop_def);
    element.properties.insert("green".to_string(), green_prop_def);
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



#[cfg(target_endian = "little")]
fn set_encoding() -> Encoding {
    Encoding::BinaryLittleEndian
}

#[cfg(target_endian = "big")]
fn set_encoding() -> Encoding {
    Encoding::BinaryBigEndian
}