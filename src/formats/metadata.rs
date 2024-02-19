use serde::{Deserialize, Serialize};

use super::bounds::Bounds;

#[derive(Serialize, Deserialize)]
pub struct MetaData {
    pub bounds: Vec<Bounds>,
    pub point_nums: Vec<Vec<usize>>,
    pub num_of_additional_file: usize,
    pub partitions: (usize, usize, usize),
}

impl MetaData {
    pub fn new(
        bounds: Vec<Bounds>,
        point_nums: Vec<Vec<usize>>,
        num_of_additional_file: usize,
        partitions: (usize, usize, usize),
    ) -> Self {
        Self {
            bounds,
            point_nums,
            num_of_additional_file,
            partitions,
        }
    }

    pub fn default() -> Self {
        Self {
            bounds: vec![],
            point_nums: vec![],
            num_of_additional_file: 0,
            partitions: (0, 0, 0),
        }
    }

    pub fn next(&mut self, bound: Bounds, point_num: Vec<usize>) {
        self.bounds.push(bound);
        self.point_nums.push(point_num);
    }
}
