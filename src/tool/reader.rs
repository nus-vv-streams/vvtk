extern crate ply_rs;

use crate::errors::*;
use crate::points::{Point, Points};
use ply_rs::parser;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

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
pub fn read(input: Option<&str>) -> Result<Points> {
    let stdin = io::stdin();
    let mut file_name: Option<String> = None;

    let result_buf_read: Result<Box<dyn BufRead>> = match input {
        Some(path) => {
            let path = Path::new(path);
            let is_ply_file = path.extension().filter(|e| e.to_str() == Some("ply"));

            file_name = path
                .file_name()
                .map(|name| name.to_owned().into_string().unwrap());

            match is_ply_file {
                Some(_) => Ok(Box::new(BufReader::new(File::open(path)?))),
                None => bail!(format!(
                    "{}{}{}",
                    "Extension of file: ",
                    input.unwrap(),
                    " expected to be .ply"
                )),
            }
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

    Ok(Points::of(file_name, points_list))
}
