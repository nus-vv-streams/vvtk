use crate::formats::pointxyzrgba::PointXyzRgba;

use super::antialias::AntiAlias;
use super::camera::CameraState;

use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::vec::Vec;

pub struct ResolutionController {
    anchor_spacing: f32,
    anchor_num_points: usize,
    // centroid: [f32; 3],
    points: Vec<PointXyzRgba>,
}

impl ResolutionController {
    pub fn new(
        points: &Vec<PointXyzRgba>,
        anchor_num_points: usize,
        anti_alias: AntiAlias,
    ) -> Self {
        let points = anti_alias.apply(points);
        let anchor_spacing = Self::calculate_spacing(&points);
        // let centroid = Self::centroid(&points);

        Self {
            anchor_spacing,
            anchor_num_points,
            // centroid,
            points,
        }
    }

    pub fn get_desired_num_points(&mut self, camera_state: &CameraState) -> usize {
        let window_size = camera_state.get_window_size();

        // get nearest distance by comparing each point to the camera
        let mut z = f32::MAX;

        for p in self.points.iter() {
            let d = camera_state.distance([p.x, p.y, p.z]);
            if d < z {
                z = d;
            }
        }

        // let z = camera_state.distance(self.centroid);
        let (width, height) = camera_state.get_plane_at_z(z);

        println!("z: {}, width: {}, height: {}", z, width, height);
        println!("window_size: {:?}", window_size);

        let x_spacing = width / window_size.width as f32;
        let y_spacing = height / window_size.height as f32;

        println!("x_spacing: {}, y_spacing: {}", x_spacing, y_spacing);

        let desired_spacing = x_spacing.min(y_spacing);
        let scaling_factor = (self.anchor_spacing / desired_spacing).powi(3);

        println!(
            "desired_spacing: {}, anchor_spacing: {}, scaling_factor: {}",
            desired_spacing, self.anchor_spacing, scaling_factor
        );

        return (self.anchor_num_points as f32 * scaling_factor as f32) as usize;
    }

    fn calculate_spacing(points: &Vec<PointXyzRgba>) -> f32 {
        let mut tree = KdTree::new(3);
        for (i, p) in points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i).unwrap();
        }

        let mut sum = 0.0;
        let k_nearest = 4;

        for p in points.iter() {
            let avg_spacing = tree
                .nearest(&[p.x, p.y, p.z], k_nearest, &squared_euclidean)
                .unwrap()
                .iter()
                .skip(1) // ignore the first point (same point)
                .map(|(d, _)| d.sqrt())
                .sum::<f32>()
                / (k_nearest - 1) as f32;

            sum += avg_spacing;
        }

        sum / points.len() as f32
    }
}
