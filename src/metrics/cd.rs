use crate::formats::pointxyzrgba::PointXyzRgba;
use kiddo::KdTree;

use super::acd::Acd;

pub struct Cd;

impl Cd {
    pub fn calculate_metric(
        p1: &Vec<PointXyzRgba>,
        p1_tree: &KdTree<f32, usize, 3>,
        p2: &Vec<PointXyzRgba>,
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let acd_rt = Acd::calculate_metric(p1, p1_tree, p2, p2_tree);
        let acd_tr = Acd::calculate_metric(p2, p2_tree, p1, p1_tree);

        (acd_rt + acd_tr) / 2.0
    }

    pub fn calculate_from_acd(
        acd_rt: Option<f64>,
        acd_tr: Option<f64>,
        p1: &Vec<PointXyzRgba>,
        p1_tree: &KdTree<f32, usize, 3>,
        p2: &Vec<PointXyzRgba>,
        p2_tree: &KdTree<f32, usize, 3>,
    ) -> Option<f64> {
        match (acd_rt, acd_tr) {
            (Some(acd_rt), Some(acd_tr)) => Some((acd_rt + acd_tr) / 2.0),
            _ => Some(Cd::calculate_metric(p1, p1_tree, p2, p2_tree)),
        }
    }
}
