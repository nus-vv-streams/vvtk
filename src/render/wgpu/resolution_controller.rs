use crate::formats::pointxyzrgba::PointXyzRgba;

use super::camera::CameraState;

use cgmath::Angle;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::vec::Vec;

pub struct ResolutionController {}

impl ResolutionController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_desired_num_points(
        &mut self,
        camera_state: &CameraState,
        points: &Vec<PointXyzRgba>,
        num_of_points: usize,
    ) -> u64 {
        let fovy = camera_state.get_fovy();
        let window_size = camera_state.get_window_size();
        let aspect = window_size.width as f32 / window_size.height as f32;
        let z = camera_state.distance(self.centroid(&points));

        let height = 2.0 * z * (fovy / 2.0).tan();
        let width = height * aspect;

        let x_spacing = width / window_size.width as f32;
        let y_spacing = height / window_size.height as f32;

        let desired_spacing = (x_spacing.powi(2) + y_spacing.powi(2)).sqrt();
        let current_spacing = self.calculate_spacing(&points);

        let scaling_factor = (desired_spacing / current_spacing).powi(2);
        return (num_of_points as f32 * scaling_factor as f32) as u64;
    }

    fn centroid(&mut self, points: &Vec<PointXyzRgba>) -> [f32; 3] {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;
        let count = points.len() as f32;

        for p in points.iter() {
            sum_x += p.x;
            sum_y += p.y;
            sum_z += p.z;
        }

        [sum_x / count, sum_y / count, sum_z / count]
    }

    fn calculate_spacing(&mut self, points: &Vec<PointXyzRgba>) -> f32 {
        let mut tree = KdTree::new(3);
        for (i, p) in points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i).unwrap();
        }

        let mut sum = 0.0;
        let mut count = 0;

        for p in points.iter() {
            let result = tree
                .nearest(&[p.x, p.y, p.z], 2, &squared_euclidean)
                .unwrap();
            let (_, index) = result[1];
            let other_point = &points[*index];

            sum += squared_euclidean(
                &[p.x, p.y, p.z],
                &[other_point.x, other_point.y, other_point.z],
            )
            .sqrt();
            count += 1;
        }

        (sum / count as f32).sqrt()
    }
}
