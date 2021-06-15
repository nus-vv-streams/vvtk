extern crate nalgebra as na;
use na::Point3;
extern crate ply_rs;
use ply_rs::ply;

pub struct Coordinate {
    data: Vec<PointCoordinate>
}

impl Coordinate {
    pub fn new(data: Vec<PointCoordinate>) -> Self {
        Coordinate {
            data: data
        }
    }

    pub fn get(&self) -> Vec<Point3<f32>> {
        let mut vec = Vec::new(); 

        for point_color in &self.data {
            vec.push(point_color.get())
        }

        vec
    }
}

#[derive(Clone)]
pub struct PointCoordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl PointCoordinate {
    pub fn new_default() -> Self {
        PointCoordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    fn new(x: f32, y: f32, z: f32) -> Self {
        PointCoordinate {
            x: x,
            y: y,
            z: z,
        }
    }

    pub fn get(&self) -> Point3<f32> {
       Point3::new(self.x, self.y, self.z)
    }
}