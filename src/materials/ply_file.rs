extern crate ply_rs;
use ply_rs::parser;
// use ply_rs::ply::{ Ply, DefaultElement, Encoding, ElementDef, PropertyDef, PropertyType, ScalarType, Property, Addable };
use ply_rs::ply::{ Ply, DefaultElement, Encoding, ElementDef, PropertyDef, PropertyType, ScalarType, Property, Addable };
use ply_rs::writer::{ Writer };

use std::fs::File;
use std::io::prelude::*;

use crate::points::{ Points, Point };
use std::path::{ Path };
// use crate::color::{ Color, PointColor };
// use crate::color_rgb::{ ColorRGB, PointColorRGB };
// use crate::coordinate::{ Coordinate, PointCoordinate };


#[derive(Debug)]
pub struct PlyFile {
    file: File
}

    
impl PlyFile {
    pub fn new(path: &str) -> Result<Self, &str> {
        let new_path = Path::new(path);
        if new_path.is_file() {
            Ok(PlyFile {
                file: File::open(new_path).unwrap()
            })
        } else {
            Err("It is not a file")
        }
    }

    pub fn create(path: &str) -> Result<Self, &str> {
        let new_path = Path::new(path);

        Ok(PlyFile {
            file: File::create(new_path).unwrap()
        })
        
    }

    
    pub fn read(&self) -> Points {
        let mut f = std::io::BufReader::new(&self.file);
    
        let point_parser = parser::Parser::<Point>::new();
    
        let header = point_parser.read_header(&mut f).unwrap();
    
        let mut points_list = Vec::new();
        for (_ignore_key, element) in &header.elements {
            points_list = point_parser.read_payload_for_element(&mut f, &element, &header).unwrap();
        }

        for idx in 0..points_list.len()
        {
            points_list[idx].set_index(idx);
        }

        Points::of(points_list)
    }

    pub fn writen_as_binary(self, data: Points) -> std::io::Result<()> {
        let mut buf = Vec::<u8>::new();

        let mut ply = {
            let mut ply = Ply::<DefaultElement>::new();
            // ply.header.encoding = Encoding::Ascii;
            ply.header.encoding = Encoding::BinaryLittleEndian;
            ply.header.comments.push("A beautiful comment!".to_string());

            let mut point_element = ElementDef::new("vertex".to_string());
            let p = PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("z".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("red".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("green".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("blue".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            ply.header.elements.add(point_element);

            let mut points = Vec::new();

            for entry in data.get_data() {
                let coord = entry.get_coord();
                let color = entry.get_color();

                let mut point = DefaultElement::new();
                point.insert("x".to_string(), Property::Float(coord.x));
                point.insert("y".to_string(), Property::Float(coord.y));
                point.insert("z".to_string(), Property::Float(coord.z));
                point.insert("red".to_string(), Property::UChar(color.red));
                point.insert("green".to_string(), Property::UChar(color.green));
                point.insert("blue".to_string(), Property::UChar(color.blue));
                points.push(point);
            }

            ply.payload.insert("vertex".to_string(), points);
            ply.make_consistent().unwrap();
            ply
        };

        let w = Writer::new();
        w.write_ply(&mut buf, &mut ply).unwrap();

        let mut file = self.file;
        file.write_all(&buf)?;
        
        Ok(())
    }

    pub fn writen_as_ascii(self, data: Points) -> std::io::Result<()> {
        let mut buf = Vec::<u8>::new();

        let mut ply = {
            let mut ply = Ply::<DefaultElement>::new();
            // ply.header.encoding = Encoding::Ascii;
            ply.header.encoding = Encoding::Ascii;
            ply.header.comments.push("A beautiful comment!".to_string());

            let mut point_element = ElementDef::new("vertex".to_string());
            let p = PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("z".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("red".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("green".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("blue".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            ply.header.elements.add(point_element);

            let mut points = Vec::new();

            for entry in data.get_data() {
                let coord = entry.get_coord();
                let color = entry.get_color();

                let mut point = DefaultElement::new();
                point.insert("x".to_string(), Property::Float(coord.x));
                point.insert("y".to_string(), Property::Float(coord.y));
                point.insert("z".to_string(), Property::Float(coord.z));
                point.insert("red".to_string(), Property::UChar(color.red));
                point.insert("green".to_string(), Property::UChar(color.green));
                point.insert("blue".to_string(), Property::UChar(color.blue));
                points.push(point);
            }

            ply.payload.insert("vertex".to_string(), points);
            ply.make_consistent().unwrap();
            ply
        };

        let w = Writer::new();
        w.write_ply(&mut buf, &mut ply).unwrap();

        let mut file = self.file;
        file.write_all(&buf)?;
        
        Ok(())
    }

    pub fn render(&self) {
        self.read().render()
    }

    pub fn take_sreenshoot(&self) {
        self.read().take_sreenshoot_to_path("plySource/out/png/screenshoot.png")
    }

    pub fn take_sreenshoot_to_path(&self, path: &str) {
        self.read().take_sreenshoot_to_path(path)
    }
}