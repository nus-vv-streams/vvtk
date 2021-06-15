extern crate nalgebra as na;
use na::Point3;
extern crate ply_rs;
use ply_rs::ply;

pub struct Color {
    data: Vec<PointColor>
}

impl Color {
    pub fn new(data: Vec<PointColor>) -> Self {
        Color {
            data: data
        }
    }

    pub fn get(self) -> Vec<Point3<f32>> {
        self.data.into_iter().map(|point_color| point_color.get()).collect()
    }
}

#[derive(Clone)]
pub struct PointColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl PointColor {
    pub fn new_default() -> Self {
        PointColor {
            red: 0,
            green: 0,
            blue: 0
        }
    }

    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        PointColor {
            red: red,
            green: green,
            blue: blue,
        }
    }

    pub fn get(&self) -> Point3<f32> {
       Point3::new(self.red as f32 /256.0, self.green as f32 /256.0, self.blue as f32 /256.0)
    }
}