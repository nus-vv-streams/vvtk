use crate::formats::pointxyzrgba::PointXyzRgba;
use float_ord::FloatOrd;
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::prelude::*;
use std::cmp;

pub struct Hd;

impl Hd {
    pub fn calculate_metric(
        p1: &Vec<PointXyzRgba>,
        p1_tree: &KdTree<f32, usize, 3>,
        p2: &Vec<PointXyzRgba>,
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let p1_to_p2 = Hd::get_hd(p1, p1_tree, p2, p2_tree);
        let p2_to_p1 = Hd::get_hd(p2, p2_tree, p1, p1_tree);

        f64::max(p1_to_p2, p2_to_p1)
    }

    fn get_hd(
        p1: &Vec<PointXyzRgba>,
        p1_tree: &KdTree<f32, usize, 3>,
        p2: &Vec<PointXyzRgba>,
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let hd_max = p1
            .par_iter()
            .map(|pt| {
                let nearest_points = p2_tree
                    .nearest(&[pt.x, pt.y, pt.z], 2, &squared_euclidean)
                    .unwrap();
                let (dist, _) = nearest_points[0];
                FloatOrd(dist)
            })
            .max()
            .unwrap();

        hd_max.0.into()
    }
}
