use std::path::Path;

use ply_rs::ply::Property;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

pub fn read_ply<P: AsRef<Path>>(path_buf: P) -> Option<PointCloud<PointXyzRgba>> {
    let vertex_parser = ply_rs::parser::Parser::<PointXyzRgba>::new();
    let f = std::fs::File::open(path_buf.as_ref())
        .unwrap_or_else(|_| panic!("Unable to open file {:?}", path_buf.as_ref()));
    let mut f = std::io::BufReader::new(f);

    let header = vertex_parser.read_header(&mut f).unwrap_or_else(|_| panic!("Failed to read header for ply file {:?}",
        path_buf.as_ref()));

    let mut vertex_list = Vec::new();
    for (_, element) in &header.elements {
        if element.name.as_str() == "vertex" {
            vertex_list = match vertex_parser.read_payload_for_element(&mut f, element, &header) {
                Ok(v) => v,
                Err(e) => {
                    println!("Failed to convert {:?}\n{e}", path_buf.as_ref());
                    return None;
                }
            }
        }
    }

    Some(PointCloud {
        number_of_points: vertex_list.len(),
        points: vertex_list,
    })
}

impl ply_rs::ply::PropertyAccess for PointXyzRgba {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn set_property(&mut self, key: &String, property: Property) {
        match (key.as_ref(), property) {
            ("x", Property::Double(v)) => self.x = v as f32,
            ("y", Property::Double(v)) => self.y = v as f32,
            ("z", Property::Double(v)) => self.z = v as f32,
            ("x", Property::UInt(v)) => self.x = v as f32,
            ("y", Property::UInt(v)) => self.y = v as f32,
            ("z", Property::UInt(v)) => self.z = v as f32,
            ("x", Property::Float(v)) => self.x = v,
            ("y", Property::Float(v)) => self.y = v,
            ("z", Property::Float(v)) => self.z = v,
            ("red", Property::UChar(v)) => self.r = v,
            ("green", Property::UChar(v)) => self.g = v,
            ("blue", Property::UChar(v)) => self.b = v,
            ("alpha", Property::UChar(v)) => self.a = v,
            _ => {}
        }
    }
}
