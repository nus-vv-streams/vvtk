extern crate ply_rs;
use ply_rs::ply;
extern crate nalgebra as na;

use crate::color;
use crate::coordinate;
use crate::renderer;

#[derive(Clone)]
pub struct Points {
    pub data: Vec<Point>
}

impl Points {
    pub fn new(data: Vec<Point>) -> Self {
        Points {
            data: data
        }
    }

    pub fn get_data(self) -> Vec<Point> {
        self.data
    }

    pub fn get_colors(&self) -> color::Color {
        color::Color::new(self.data.clone().into_iter().map(|point| point.point_color).collect())
    }

    pub fn get_coords(&self) -> coordinate::Coordinate {
        coordinate::Coordinate::new(self.data.clone().into_iter().map(|point| point.point_coord).collect())
    }

    pub fn render(&self) {
        renderer::Renderer::new().render_image(&self);
    }
}

#[derive(Clone)]
pub struct Point {
    pub point_coord: coordinate::PointCoordinate,
    pub point_color: color::PointColor
}

impl ply::PropertyAccess for Point {
    fn new() -> Self {
        Point {
            point_coord: coordinate::PointCoordinate::new_default(),
            point_color: color::PointColor::new_default()
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.point_coord.x = v,
            ("y", ply::Property::Float(v)) => self.point_coord.y = v,
            ("z", ply::Property::Float(v)) => self.point_coord.z = v,
            ("red", ply::Property::UChar(v)) => self.point_color.red = v,
            ("green", ply::Property::UChar(v)) => self.point_color.green = v,
            ("blue", ply::Property::UChar(v)) => self.point_color.blue = v,
            (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
        }
    }
}