extern crate ply_rs;
use crate::errors::*;
use crate::ply::Ply;
use crate::point::Point;
use crate::pointcloud::PointCloud;
use ply_rs::parser;
use ply_rs::ply;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

impl ply::PropertyAccess for Point {
    fn new() -> Self {
        Point::new_default()
    }

    fn set_property(&mut self, key: &String, property: ply::Property) {
        match (key.as_str(), property) {
            ("x", ply::Property::Float(v)) => self.coord.x = v,
            ("y", ply::Property::Float(v)) => self.coord.y = v,
            ("z", ply::Property::Float(v)) => self.coord.z = v,
            ("red", ply::Property::UChar(v)) => self.color.r = v,
            ("green", ply::Property::UChar(v)) => self.color.g = v,
            ("blue", ply::Property::UChar(v)) => self.color.b = v,
            (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
        }
    }
}

/// Read any form of ply file and return the collections of points.
///
/// # Arguments
/// * `input` - the path to the file that needed to be read
///
/// # Examples
/// ```
/// use iswr::reader;
///
/// reader::read(Some("path/to/your/ply/file")).unwrap().reader();
/// ```
pub fn read(input: Option<&str>) -> Result<Ply> {
    let stdin = io::stdin();
    let mut file_name: Option<PathBuf> = None;

    let result_buf_read: Result<Box<dyn BufRead>> = match input {
        Some(path) => {
            let path_buf = PathBuf::from(path);

            if !is_ply_file(&path_buf) {
                bail!(format!(
                    "{}{}{}",
                    "Extension of file: ",
                    input.unwrap(),
                    " expected to be .ply"
                ))
            }

            file_name = Some(path_buf);
            Ok(Box::new(BufReader::new(File::open(path)?)))
        }
        None => Ok(Box::new(stdin.lock())),
    };

    let mut buf_read = result_buf_read?;

    let point_parser = parser::Parser::<Point>::new();

    let header = point_parser.read_header(&mut buf_read).chain_err(|| {
        format!(
            "{}{}",
            "Unable to read the header of the input: ",
            input.unwrap()
        )
    })?;

    let mut points_list = Vec::new();
    for (key, element) in &header.elements {
        if key.eq("vertex") {
            points_list = point_parser.read_payload_for_element(&mut buf_read, element, &header)?;
        }
    }

    for (idx, item) in points_list.iter_mut().enumerate() {
        item.set_index(idx);
    }

    Ok(Ply::of(file_name, PointCloud::of(points_list)))
}

/// Check if PathBuf's extension exist and equal "ply"
fn is_ply_file(path_buf: &Path) -> bool {
    if path_buf
        .extension()
        .filter(|e| e.to_str() == Some("ply"))
        .is_none()
    {
        return false;
    }

    true
}
