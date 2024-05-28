use super::cd::Cd;
use crate::formats::pointxyzrgba::PointXyzRgba;
// use color_space::{FromRgb, Lab, Rgb};
use kiddo::KdTree;

pub struct VQoE;

fn rgb_to_yuv(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let (r, g, b) = rgb;
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = -0.14713 * r - 0.28886 * g - 0.436 * b + 128.0;
    let v = 0.615 * r - 0.51499 * g - 0.10001 * b + 128.0;

    // clip to 0-255
    let y = y.max(0.0).min(255.0);
    let u = u.max(0.0).min(255.0);
    let v = v.max(0.0).min(255.0);

    (y as u8, u as u8, v as u8)
}

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

        let alpha = 0.6597; // empirically determined
        let distance =
            VQoE::calculate_l2_distance(original, original_tree, reconstructed, reconstructed_tree);
        alpha * cd.unwrap() + (1.0 - alpha) * distance
    }

    pub fn calculate_l2_distance(
        original: &Vec<PointXyzRgba>,
        _original_tree: &KdTree<f32, usize, 3>,
        reconstructed: &Vec<PointXyzRgba>,
        _reconstructed_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let luminance_histogram_original = VQoE::get_histogram_distribution(original);
        let luminance_histogram_reconstructed = VQoE::get_histogram_distribution(reconstructed);

        let histogram_l2_distance: f64 = luminance_histogram_original
            .iter()
            .zip(luminance_histogram_reconstructed.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum();
        histogram_l2_distance.sqrt()
    }

    fn get_histogram_distribution(points: &Vec<PointXyzRgba>) -> Vec<f64> {
        let mut histogram = vec![0u32; 256];
        for pt in points {
            let (y, _u, _v) = rgb_to_yuv((pt.r, pt.g, pt.b));
            let luminance = y as usize;
            // check luminance is between 0~255
            assert!(luminance <= 255);
            histogram[luminance] += 1;
        }
        let total_points = points.len() as f64;
        histogram.iter().map(|x| *x as f64 / total_points).collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_yuv_to_rgb() {
        let rgb = (255, 128, 0);
        let yuv = rgb_to_yuv(rgb);
        println!("{:?}", yuv);
    }
}
