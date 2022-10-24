use crate::pcd::{PCDField, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion, PointCloudData};
use anyhow::{bail, Result};
use ply_rs::{ply, ply::Property};
use std::path::Path;

pub fn ply_to_pcd(file_path: &Path) -> Result<Option<PointCloudData>> {
    let vertex_parser = ply_rs::parser::Parser::<Vertex>::new();
    let f = std::fs::File::open(file_path).unwrap();
    let mut f = std::io::BufReader::new(f);

    let header = {
        match vertex_parser.read_header(&mut f) {
            Ok(h) => h,
            Err(e) => {
                bail!("Failed to convert {:?}\n{e}", file_path.as_os_str());
            }
        }
    };

    let mut vertex_list = Vec::new();
    for (_, element) in &header.elements {
        // we could also just parse them in sequence, but the file format might change
        if element.name.as_str() == "vertex" {
            vertex_list = match vertex_parser.read_payload_for_element(&mut f, element, &header) {
                Ok(v) => v,
                Err(e) => {
                    bail!("Failed to convert {:?}\n{e}", file_path.as_os_str());
                }
            }
        }
    }
    if vertex_list.is_empty() {
        println!(
            "{:?} does not contain any vertices..skipping this file",
            file_path.as_os_str()
        );
        return Ok(None);
    }

    let pcd = create_pcd(vertex_list);

    Ok(Some(pcd))
}

fn create_pcd(vertices: Vec<Vertex>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgb".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                1,
            )
            .unwrap(),
        ],
        vertices.len() as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        vertices.len() as u64,
    )
    .unwrap();

    let bytes: &[u8] = bytemuck::cast_slice(&vertices[..]);
    PointCloudData::new(header, bytes.to_vec()).unwrap()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        }
    }

    fn set_property(&mut self, key: &String, property: Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.x = v,
            ("y", ply::Property::Float(v)) => self.y = v,
            ("z", ply::Property::Float(v)) => self.z = v,
            ("red", ply::Property::UChar(v)) => self.red = v,
            ("green", ply::Property::UChar(v)) => self.green = v,
            ("blue", ply::Property::UChar(v)) => self.blue = v,
            ("alpha", ply::Property::UChar(v)) => self.alpha = v,
            _ => {}
        }
    }
}
