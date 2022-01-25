use crate::errors::*;

use crate::points::PointCloud;

use ply_rs::ply::{
    Addable, DefaultElement, ElementDef, Encoding, Ply, Property, PropertyDef, PropertyType,
    ScalarType,
};

use ply_rs::writer::Writer;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Write a ply file to hard drive
pub fn write(written_points: PointCloud, form: Option<&str>, output: Option<&str>) -> Result<()> {
    let encoding = match form {
        Some("ascii") => Some(Encoding::Ascii),
        Some("binary") => Some(Encoding::BinaryLittleEndian),
        Some(&_) => None,
        None => Some(Encoding::Ascii),
    };

    let mut buf = Vec::<u8>::new();

    let mut ply = {
        let mut ply = Ply::<DefaultElement>::new();
        ply.header.encoding = encoding.chain_err(|| "Invalid ply encoding form")?;
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

        for entry in written_points.get_data() {
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

    match output {
        Some(path) => {
            File::create(Path::new(path))
                .chain_err(|| "Cannot create path")?
                .write_all(&buf)?;
        }
        None => {
            io::stdout().write_all(&buf)?;
        }
    };

    Ok(())
}
