use crate::color::PointColor;
use crate::points::{Point, Points};
use crate::traits::ColorRecovery;
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

// impl ColorRecovery for Coordinate {
//     fn nearest_point_recovery(self, points: Points) -> Points {
//         let kd_tree = points.to_kdtree();

//         Points::of(
//             self.data
//                 .into_iter()
//                 .map(|coord| coord.set_color(coord.get_nearest(&kd_tree).get_color(), 0)) //SET TO 0 SINCE FUNCTION SEEMS UNUSED
//                 .collect(),
//         )
//     }
// }

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

    // pub(crate) fn get_nearest(&self, kd_tree: &KdTree<f32, usize, [f32; 3]>) -> Point {
    //     kd_tree.nearest(&[self.x, self.y, self.z], 1, &squared_euclidean).unwrap()[0].1.clone()
    // }

    // pub(crate) fn get_nearests(&self, kd_tree: &KdTree<f32, Point, [f32; 3]>, quantity: usize) -> Points {
    //     Points::of(
    //         kd_tree
    //          .nearest(&[self.x, self.y, self.z], quantity, &squared_euclidean).unwrap()
    //             .into_iter()
    //             .map(|found| found.1.clone())
    //             .collect(),
    //     )
    // }
}

// impl KdPoint for PointCoordinate {
//     type Scalar = f32;
//     type Dim = typenum::U3; // 3 dimensional tree.
//     fn at(&self, k: usize) -> f32 {
//         match k {
//             0 => self.x,
//             1 => self.y,
//             2 => self.z,
//             _ => panic!("Oh no, don't have {}", k),
//         }
//     }
// }
