use serde::{Deserialize, Serialize};

use super::bounds::Bounds;

#[derive(Serialize, Deserialize)]
pub struct MetaData {
    pub bounds: Vec<Bounds>,
    pub base_point_num: Vec<Vec<usize>>,
    pub additional_point_num: Vec<Vec<usize>>,
    pub partitions: (usize, usize, usize),
}

impl MetaData {
    pub fn new(
        bounds: Vec<Bounds>,
        base_point_num: Vec<Vec<usize>>,
        additional_point_num: Vec<Vec<usize>>,
        partitions: (usize, usize, usize),
    ) -> Self {
        Self {
            bounds,
            base_point_num,
            additional_point_num,
            partitions,
        }
    }

    pub fn default() -> Self {
        Self {
            bounds: vec![],
            base_point_num: vec![],
            additional_point_num: vec![],
            partitions: (0, 0, 0),
        }
    }

    pub fn next(
        &mut self,
        bound: Bounds,
        base_point_num: Vec<usize>,
        additional_point_num: Vec<usize>,
    ) {
        self.bounds.push(bound);
        self.base_point_num.push(base_point_num);
        self.additional_point_num.push(additional_point_num);
    }
}
