extern crate ply_rs;
use ply_rs::parser;

use std::fs::File;

use crate::errors::*;

use crate::points::{Point, Points};

use std::io::{self, BufRead, BufReader};

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
    for (_ignore_key, element) in &header.elements {
        points_list = point_parser.read_payload_for_element(&mut buf_read, &element, &header)?;
    }

    for idx in 0..points_list.len() {
        points_list[idx].set_index(idx);
    }

    Ok(Points::of(points_list))
}
