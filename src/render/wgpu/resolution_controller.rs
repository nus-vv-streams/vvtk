use crate::formats::pointxyzrgba::PointXyzRgba;

use super::camera::CameraState;

use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::vec::Vec;

pub struct ResolutionController {
    anchor_spacing: f32,
    anchor_num_points: usize,
    centroid: [f32; 3],
}

impl ResolutionController {
    pub fn new(points: &Vec<PointXyzRgba>, anchor_num_points: usize) -> Self {
        let anchor_spacing = Self::calculate_spacing(&points);
        let centroid = Self::centroid(&points);

        Self {
            anchor_spacing,
            anchor_num_points,
            centroid,
        }
    }

    pub fn get_desired_num_points(
        &mut self,
        camera_state: &CameraState,
        _points: &Vec<PointXyzRgba>,
    ) -> u64 {
        let window_size = camera_state.get_window_size();
        let z = camera_state.distance(self.centroid);
        let (clip_width, clip_height) = camera_state.get_clip_plane_at_z(z);
        let (width, height) =
            camera_state.transform_clip_to_world_plane(clip_width, clip_height, self.centroid);

        println!("z: {}, width: {}, height: {}", z, width, height);

        let x_spacing = width / window_size.width as f32;
        let y_spacing = height / window_size.height as f32;

        println!("x_spacing: {}, y_spacing: {}", x_spacing, y_spacing);

        let desired_spacing = (x_spacing.powi(2) + y_spacing.powi(2)).sqrt();
        // let scaling_factor = (self.anchor_spacing / desired_spacing).powi(2);
        let scaling_factor = self.anchor_spacing / desired_spacing;

        println!(
            "desired_spacing: {}, anchor_spacing: {}, scaling_factor: {}",
            desired_spacing, self.anchor_spacing, scaling_factor
        );

        return (self.anchor_num_points as f32 * scaling_factor as f32) as u64;
    }

    fn centroid(points: &Vec<PointXyzRgba>) -> [f32; 3] {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;

        for p in points.iter() {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            min_z = min_z.min(p.z);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
            max_z = max_z.max(p.z);
        }

        [
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
            (min_z + max_z) / 2.0,
        ]
    }

    fn calculate_spacing(points: &Vec<PointXyzRgba>) -> f32 {
        let mut tree = KdTree::new(3);
        for (i, p) in points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i).unwrap();
        }

        let mut sum = 0.0;
        let mut count = 0;

        for p in points.iter() {
            let avg_spacing = tree
                .nearest(&[p.x, p.y, p.z], 4, &squared_euclidean)
                .unwrap()
                .iter()
                .skip(1) // dont count the first point
                .map(|(d, _)| d.sqrt())
                .sum::<f32>()
                / 3.0;

            sum += avg_spacing;
            count += 1;
        }

        (sum / count as f32).sqrt()
    }
}
