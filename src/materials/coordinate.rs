use crate::color::PointColor;
use crate::points::Point;

use nalgebra::Point3;

/// Structure representing a collection of coordinates (in RGB) in the collection the points.
pub struct Coordinate {
    data: Vec<PointCoordinate>,
}

impl Coordinate {
    /// Creating a new collection of coordinates with specific data
    pub fn new(data: Vec<PointCoordinate>) -> Self {
        Coordinate { data }
    }

    /// Get a data under the borrow type
    pub fn get_borrow_data(&self) -> &Vec<PointCoordinate> {
        &self.data
    }
}

/// Structure representing the 3D-coordinate of one point.
#[derive(Debug, Clone)]
pub struct PointCoordinate {
    /// x-coordinate
    pub x: f32,
    /// y-coordinate
    pub y: f32,
    /// z-coordinate
    pub z: f32,
}

impl PartialEq for PointCoordinate {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl PointCoordinate {
    /// Return the original
    pub fn new_default() -> Self {
        PointCoordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Return a `PointCoordinate` with specific coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        PointCoordinate { x, y, z }
    }

    /// Return the `Point3` type of the `PointCoordinate` for rendering
    pub fn get_point3(&self) -> Point3<f32> {
        Point3::new(self.x, self.y, self.z)
    }

    /// Add `PointColor` and `index` to create a `Point`
    pub fn set_color(&self, point_color: &PointColor, index: usize) -> Point {
        Point::new(self.clone(), point_color.clone(), 0, index, 0.0, 1.0, false)
    }

    /// Return a midpoint of two `PointCoordinate`s
    pub fn get_average(&self, another_point: &PointCoordinate, prev_weight: f32, next_weight: f32) -> PointCoordinate {
        // PointCoordinate::new(
        //     (self.x + another_point.x) / 2.0,
        //     (self.y + another_point.y) / 2.0,
        //     (self.z + another_point.z) / 2.0,
        // )

         PointCoordinate::new(
            (self.x * prev_weight) + (another_point.x * next_weight),
            (self.y * prev_weight) + (another_point.y * next_weight),
            (self.z * prev_weight) + (another_point.z * next_weight),
        )
    }

    /// Return the distance between two `PointCoordinate`s
    pub fn get_coord_delta(&self, another_point: &PointCoordinate) -> f32 {
        (self.x - another_point.x)
            .hypot(self.y - another_point.y)
            .hypot(self.z - another_point.z)
    }
}
