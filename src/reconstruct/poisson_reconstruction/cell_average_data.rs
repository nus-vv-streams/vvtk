use super::Real;

use nalgebra::Point3;

#[derive(PartialEq, Debug, Clone)]
pub struct CellAverageData {
    size: usize,
    average_point: Point3<Real>,
}

impl CellAverageData {
    /// Creates a new CellAverageData
    pub fn new() -> Self {
        Self {
            size: 0,
            average_point: Point3::new(0.0, 0.0, 0.0),
        }
    }

    /// Updates the average point of a cell when a new point is added
    pub fn add_point(&mut self, point: Point3<Real>) {
        // Update the average_point by multiplying the current average_point by size
        self.average_point.x *= self.size as f64;
        self.average_point.y *= self.size as f64;
        self.average_point.z *= self.size as f64;

        // Add the new point to the sum
        self.average_point.x += point.x;
        self.average_point.y += point.y;
        self.average_point.z += point.z;

        // Increment the size
        self.size += 1;

        // Calculate the new average_point by dividing the sum by the new size
        self.average_point.x /= self.size as f64;
        self.average_point.y /= self.size as f64;
        self.average_point.z /= self.size as f64;
    }

    /// Get the average point from this CellAverageData
    pub fn get_cell_average_point(&self) -> Point3<Real> {
        self.average_point.clone()
    }
}
