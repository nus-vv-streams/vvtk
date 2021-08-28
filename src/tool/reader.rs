extern crate ply_rs;
use ply_rs::parser;

use std::fs::File;

use crate::errors::*;

use crate::points::{Point, Points};

use std::io::{self, BufRead, BufReader};

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

    let mut buf_read: Box<dyn BufRead> = match input {
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
        None => Box::new(stdin.lock()),
    };

    let point_parser = parser::Parser::<Point>::new();

    let header = point_parser
        .read_header(&mut buf_read)
        .chain_err(|| "Unable to read the header of the input")?;

    let mut points_list = Vec::new();
    for (key, element) in &header.elements {
        if key.eq("vertex") {
            points_list = point_parser.read_payload_for_element(&mut buf_read, element, &header)?;
        }
    }

    for (idx, item) in points_list.iter_mut().enumerate() {
        item.set_index(idx);
    }

    Ok(Points::of(points_list))
}
