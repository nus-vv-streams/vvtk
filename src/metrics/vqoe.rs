use crate::formats::pointxyzrgba::PointXyzRgba;
use color_space::{FromRgb, Lab, Rgb};
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::prelude::*;
use super::cd::Cd;

pub struct VQoE;

impl VQoE {
    pub fn calculate_metric(
        acd_rt: Option<f64>,
        acd_tr: Option<f64>,
        cd: Option<f64>,
        original: &Vec<PointXyzRgba>,
        original_tree: &KdTree<f32, usize, 3>,
        reconstructed: &Vec<PointXyzRgba>,
        reconstructed_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let cd = match (cd, acd_rt, acd_tr) {
            (Some(cd), _, _) => Some(cd),
            (_, Some(acd_rt), Some(acd_tr)) => Some((acd_rt + acd_tr) / 2.0),
            _ => Cd::calculate_metric(original, original_tree, reconstructed, reconstructed_tree)
                .into(),
        };

        let alpha = 0.6597;
        let distance = VQoE::calculate_l2_distance(
            original,
            original_tree,
            reconstructed,
            reconstructed_tree,
        );
        alpha * cd.unwrap() + (1.0 - alpha) * distance
    } 


    pub fn calculate_l2_distance(
        original: &Vec<PointXyzRgba>,
        original_tree: &KdTree<f32, usize, 3>,
        reconstructed: &Vec<PointXyzRgba>,
        reconstructed_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let luminance_histogram_original = VQoE::get_histogram_distribution(original);
        let luminance_histogram_reconstructed = VQoE::get_histogram_distribution(reconstructed);

        let histogram_l2_distance = luminance_histogram_original
            .iter()
            .zip(luminance_histogram_reconstructed.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>();
        histogram_l2_distance
    }

    fn get_histogram_distribution(points: &Vec<PointXyzRgba>) -> Vec<f64> {
        let mut histogram = vec![0u32; 256];
        for pt in points {
            let rgb = Rgb::new(pt.r as f64, pt.g as f64, pt.b as f64);
            let lab = Lab::from_rgb(&rgb);
            let luminance = lab.l as usize;
            // check luminance is between 0~255
            assert!(luminance <= 255);
            histogram[luminance] += 1;
        }
        let total_points = points.len() as f64;
        histogram.iter().map(|x| *x as f64 / total_points).collect()
    }
}
