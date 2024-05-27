use crate::formats::pointxyzrgba::PointXyzRgba;
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::prelude::*;

pub struct Acd;

impl Acd {
    pub fn calculate_metric(
        p1: &Vec<PointXyzRgba>,
        _p1_tree: &KdTree<f32, usize, 3>,
        _p2: &[PointXyzRgba],
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let acd_sum: f32 = p1
            .par_iter()
            .map(|pt| {
                let nearest_points = p2_tree
                    .nearest(&[pt.x, pt.y, pt.z], 2, &squared_euclidean)
                    .unwrap();
                let (dist, _) = nearest_points[0];
                dist
            })
            .sum();

        acd_sum as f64 / p1.len() as f64
    }

    pub fn calculate_if_none(
        acd: Option<f64>,
        p1: &Vec<PointXyzRgba>,
        p1_tree: &KdTree<f32, usize, 3>,
        p2: &Vec<PointXyzRgba>,
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> Option<f64> {
        match acd {
            Some(acd) => Some(acd),
            None => Some(Acd::calculate_metric(p1, p1_tree, p2, p2_tree)),
        }
    }
}
