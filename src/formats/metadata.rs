use serde::{Deserialize, Serialize};

use super::bounds::Bounds;

#[derive(Serialize, Deserialize)]
pub struct MetaData {
    pub bounds: Vec<Vec<Bounds>>,
    pub centroids: Vec<Vec<[f32; 3]>>,
    pub num_of_additional_file: usize,
    pub partitions: (usize, usize, usize),
}

impl MetaData {
    pub fn new(
        bounds: Vec<Vec<Bounds>>,
        centroids: Vec<Vec<[f32; 3]>>,
        num_of_additional_file: usize,
        partitions: (usize, usize, usize),
    ) -> Self {
        Self {
            bounds,
            centroids,
            num_of_additional_file,
            partitions,
        }
    }

    pub fn default() -> Self {
        Self {
            bounds: vec![],
            centroids: vec![],
            num_of_additional_file: 0,
            partitions: (0, 0, 0),
        }
    }

    pub fn next(&mut self, bounds: Vec<Bounds>, centroid: Vec<[f32; 3]>) {
        self.bounds.push(bounds);
        self.centroids.push(centroid);
    }
}
