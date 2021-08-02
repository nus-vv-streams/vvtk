use crate::color::PointColor;
use crate::points::Point;

// use kd_tree::{KdPoint, KdTree};
// use kdtree::KdTree;
// use kdtree::ErrorKind;
// use kdtree::distance::squared_euclidean;

use kiddo::distance::squared_euclidean;
use kiddo::ErrorKind;
use kiddo::KdTree;

use nalgebra::Point3;

pub struct Coordinate {
    data: Vec<PointCoordinate>,
}

impl Coordinate {
    pub fn new(data: Vec<PointCoordinate>) -> Self {
        Coordinate { data }
    }

    pub fn get_point_coor_vec(&self) -> &Vec<PointCoordinate> {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct PointCoordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl PartialEq for PointCoordinate {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl PointCoordinate {
    pub fn new_default() -> Self {
        PointCoordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        PointCoordinate { x, y, z }
    }

    pub fn get_point3(&self) -> Point3<f32> {
        Point3::new(self.x, self.y, self.z)
    }

    pub fn set_color(&self, point_color: &PointColor, index: usize) -> Point {
        Point::new(self.clone(), point_color.clone(), 0, index, 0.0, 1.0, false)
    }

    pub fn get_average(&self, another_point: &PointCoordinate) -> PointCoordinate {
        PointCoordinate::new(
            (self.x + another_point.x) / 2.0,
            (self.y + another_point.y) / 2.0,
            (self.z + another_point.z) / 2.0,
        )
    }

    pub fn get_coord_delta(&self, another_point: &PointCoordinate) -> f32 {
        (self.x - another_point.x)
            .hypot(self.y - another_point.y)
            .hypot(self.z - another_point.z)
    }
}
