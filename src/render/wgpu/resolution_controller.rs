use crate::formats::metadata::MetaData;
use crate::formats::pointxyzrgba::PointXyzRgba;

use super::antialias::AntiAlias;
use super::camera::CameraState;

use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::iter::zip;
use std::vec::Vec;

pub struct ResolutionController {
    anchor_spacing: f32,
    anti_alias: AntiAlias,
    metadata: Option<MetaData>,
}

impl ResolutionController {
    pub fn new(
        points: &Vec<PointXyzRgba>,
        metadata: Option<MetaData>,
        anti_alias: AntiAlias,
    ) -> Self {
        let points = anti_alias.apply(points);
        let anchor_spacing = Self::calculate_spacing(&points);

        Self {
            anchor_spacing,
            anti_alias,
            metadata,
        }
    }

    pub fn get_desired_num_points(
        &self,
        index: usize,
        camera_state: &CameraState,
        exclude_base_points: bool,
    ) -> Vec<usize> {
        let metadata = self.metadata.as_ref().unwrap();

        let centroids = metadata.centroids.get(index).unwrap();
        let base_point_num = metadata.point_nums.get(index).unwrap();

        zip(centroids.iter(), base_point_num.iter())
            .map(|(centroid, point_num)| {
                if let Some(centroid) = centroid {
                    let desired_num =
                        self.get_desired_num_points_at(camera_state, centroid, *point_num);
                    if exclude_base_points {
                        desired_num - (*point_num).min(desired_num)
                    } else {
                        desired_num
                    }
                } else {
                    0
                }
            })
            .collect()
    }

    fn get_desired_num_points_at(
        &self,
        camera_state: &CameraState,
        poi: &[f32; 3],
        current_point_num: usize,
    ) -> usize {
        let window_size = camera_state.get_window_size();
        let z = camera_state.distance(self.anti_alias.apply_single(poi));

        let (width, height) = camera_state.get_plane_at_z(z);
        // println!("z: {}, width: {}, height: {}", z, width, height);
        // println!("window_size: {:?}", window_size);

        let x_spacing = width / window_size.width as f32;
        let y_spacing = height / window_size.height as f32;
        // println!("x_spacing: {}, y_spacing: {}", x_spacing, y_spacing);

        let desired_spacing = x_spacing.min(y_spacing);
        let scaling_factor = (self.anchor_spacing / desired_spacing).powi(3);
        // println!(
        //     "desired_spacing: {}, anchor_spacing: {}, scaling_factor: {}",
        //     desired_spacing, self.anchor_spacing, scaling_factor
        // );

        return (current_point_num as f32 * scaling_factor as f32) as usize;
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
