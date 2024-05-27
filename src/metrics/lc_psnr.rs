use crate::formats::pointxyzrgba::PointXyzRgba;
use color_space::{FromRgb, Lab, Rgb};
use kiddo::{distance::squared_euclidean, KdTree};
use rayon::prelude::*;
// use image::{Rgb, RgbImage, ColorType};

pub struct LcPsnr;

// fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {

// }

impl LcPsnr {
    pub fn calculate_metric(
        orginal: &Vec<PointXyzRgba>,
        _original_tree: &KdTree<f32, usize, 3>,
        reconstructed: &[PointXyzRgba],
        reconstructed_tree: &KdTree<f32, usize, 3>,
    ) -> f64 {
        let error: f64 = orginal
            .par_iter()
            .map(|pt| {
                let nearest_points = reconstructed_tree
                    .nearest(&[pt.x, pt.y, pt.z], 2, &squared_euclidean)
                    .unwrap();
                let (_, idx) = nearest_points[0];
                let rgb_p2 = Rgb::new(
                    reconstructed[*idx].r as f64,
                    reconstructed[*idx].g as f64,
                    reconstructed[*idx].b as f64,
                );
                let lab_p2 = Lab::from_rgb(&rgb_p2);

                let rgb_p1 = Rgb::new(pt.r as f64, pt.g as f64, pt.b as f64);
                let lab_p1 = Lab::from_rgb(&rgb_p1);

                (lab_p1.l / 255.0 - lab_p2.l / 255.0).powi(2)
            })
            .sum();

        let l_mse = error / orginal.len() as f64;
        10f64 * (1f64 / l_mse).log(10f64)
    }
}

// #[cfg(test)]
// mod test {
//     use color_space::{Lab, FromRgb, Rgb};

//     #[test]
//     fn test_rgb_to_lab(){
//         let (r,g,b) = (255, 0, 0);
//         let rgb = Rgb::new(r as f64, g as f64, b as f64);
//         let lab = Lab::from_rgb(&rgb);
//         println!("{:?}", lab);
//     }
// }
