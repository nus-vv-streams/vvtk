use rand::seq::SliceRandom;
use rand::thread_rng;

use kiddo::KdTree;
// use std::iter::Iterator;
// use std::sync::*;

use crate::point::Point;
// use nalgebra::Point3;

// use std::cmp::Ordering;

//use crate::color::{Color, PointColor};
//use crate::coordinate::Coordinate;
// use crate::interpolate_controller::kdtree_dim;

// use std::f32::consts::PI;

// use crate::interpolate::inf_norm;

#[derive(Clone)]
/// Class of Points containing all necessary metadata
pub struct PointCloud {
    /// Data is a vector of type Point, storing all coordinate and colour data
    pub data: Vec<Point>,
}

impl Default for PointCloud {
    fn default() -> Self {
        PointCloud::new()
    }
}

impl PointCloud {
    /// Creates new instance of PointCloud
    pub fn new() -> Self {
        PointCloud {
            data: Vec::new(),
        }
    }

    /// Appends new Point to stored data
    pub fn add(&mut self, elem: Point) {
        self.data.push(elem);
    }

    /// Creates new instance of PointCloud given a vector of Point
    pub fn of(data: Vec<Point>) -> Self {
        PointCloud {
            data,
        }
    }

    /// Returns length of stored data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if stored data vector is empty
    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    /// Returns stored data as a vector of Point
    pub fn get_data(self) -> Vec<Point> {
        self.data
    }

    /// Returns clone of stored data
    pub fn get_clone_data(&self) -> Vec<Point> {
        self.data.clone()
    }

    /*
    /// Returns new instance of Colour portion of stored data
    fn get_colors(self) -> color::Color {
        Color::new(self.data.into_iter().map(|point| point.color).collect())
    }

    /// Returns new instance of Coordinate portion of stored data
    fn get_coords(self) -> coordinate::Coordinate {
        Coordinate::new(self.data.into_iter().map(|point| point.coord).collect())
    }

    /// Returns new instances of Coordinate and Colour portions of stored data as a tuple
    fn get_coords_cols(self) -> (Coordinate, Color) {
        let mut coords = Vec::new();
        let mut colors = Vec::new();
        for point in self.data {
            coords.push(point.coord);
            colors.push(point.color);
        }

        (Coordinate::new(coords), Color::new(colors))
    }
    */

    /// Constructs and returns a 3D kdtree from a class of PointCloud
    pub fn to_kdtree(self) -> KdTree<f32, usize, 3> {
        let mut kdtree: KdTree<f32, usize, 3> = KdTree::with_per_node_capacity(64).unwrap();
        let mut shuffled_points = self.data;
        shuffled_points.shuffle(&mut thread_rng());
        for point in &shuffled_points {
            kdtree
                .add(&point.coord(),  point.index)
                .unwrap();
        }
        kdtree
    }

    /// Constructs and returns a 6D kdtree from a class of PointCloud
    pub fn to_6dtree(self) -> KdTree<f32, usize, 6> {
        let mut kdtree: KdTree<f32, usize, 6> = KdTree::with_per_node_capacity(64).unwrap();
        let mut shuffled_points = self.data;
        shuffled_points.shuffle(&mut thread_rng());
        for point in &shuffled_points {
            kdtree
                .add(&point.coord_and_colors(), point.index)
                .unwrap();
        }
        kdtree
    }

    /*
    /// Highlights unmapped points as Green in the reference frame
    pub fn mark_unmapped_points(
        &mut self,
        kd_tree: Arc<kiddo::KdTree<f32, usize, 3>>,
        exists_output_dir: bool,
        dist_func: for<'r, 's> fn(&'r [f32; 3], &'s [f32; 3]) -> f32,
        ) {
        let mut mapped_points = 0;
        let mut all_unmapped: bool = true;

        for point in self.reference_frame.clone().iter_mut() {
            if point.mapping == 0 {
                let k_nearest_indices = point.get_nearest_neighbours(&kd_tree, 3, dist_func);
                for idx in &k_nearest_indices {
                    if self.reference_frame[*idx].mapping != 0 {
                        all_unmapped = false;
                    }
                }

                if all_unmapped {
                    for idx in k_nearest_indices {
                        // self.reference_frame[idx].color = PointColor::new(0, 255, 0);
                    }
                }

                all_unmapped = true;
            } else {
                mapped_points += 1;
            }
        }

        if exists_output_dir {
            println!(
                "mapped points: {}; total points: {}",
                mapped_points,
                self.reference_frame.len()
                );
        }
    }
    */

    /*
    /// Highlights points in close range to cracks as Red in the interpolated frame
    pub fn mark_points_near_cracks(
        &mut self,
        point_data: &PointCloud,
        exists_output_dir: bool,
        ) -> PointCloud {
        let mut marked_interpolated_frame = point_data.clone();

        let mut points_near_cracks = 0;

        for idx in 0..point_data.data.len() {
            marked_interpolated_frame.data[idx].point_size = 1.0;
            if point_data.data[idx].near_crack {
                // marked_interpolated_frame.data[idx].color = PointColor::new(255, 0, 0);
                points_near_cracks += 1;
            }
        }

        if exists_output_dir {
            println!("number of points near cracks: {}", points_near_cracks);
        }

        marked_interpolated_frame
    }
    */

    /*
    /// Changes point size based on surrounding point density
    pub fn adjust_point_sizes(&mut self, radius: f32) {
        let interpolated_kd_tree = self.clone().to_kdtree();

        for idx in 0..self.data.len() {
            let density = interpolated_kd_tree
                .within_unsorted(
                    &[
                    self.data[idx].coord.x,
                    self.data[idx].coord.y,
                    self.data[idx].coord.z,
                    ],
                    radius,
                    &inf_norm,
                    )
                .unwrap()
                .len() as f32
                / (radius.powi(2) * PI);

            if density <= self.data[idx].point_density {
                self.data[idx].near_crack = true;
                self.data[idx].point_size = 2.0;
            }
        }
    }
    */
}

impl IntoIterator for PointCloud {
    type Item = Point;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
