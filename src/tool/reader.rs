extern crate ply_rs;
use ply_rs::parser;

use std::fs::File;

use crate::points::{Point, Points};

use std::io::{self, BufRead, BufReader};

pub fn read(input: Option<&str>) -> Points {
    let stdin = io::stdin();

    let mut buf_read: Box<dyn BufRead> = match input {
        Some(path) => Box::new(BufReader::new(File::open(path).expect("Not a file"))),
        None => Box::new(stdin.lock()),
    };

    let point_parser = parser::Parser::<Point>::new();

    let header = point_parser.read_header(&mut buf_read).unwrap();

    let mut points_list = Vec::new();
    for (_ignore_key, element) in &header.elements {
        points_list = point_parser
            .read_payload_for_element(&mut buf_read, &element, &header)
            .unwrap();
    }

    for idx in 0..points_list.len() {
        points_list[idx].set_index(idx);
    }

    Points::of(points_list)
}
