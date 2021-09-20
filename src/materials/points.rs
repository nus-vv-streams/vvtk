use rand::seq::SliceRandom;
use rand::thread_rng;

use kiddo::KdTree;
use std::iter::Iterator;
use std::sync::*;

use crate::point::Point;
use nalgebra::Point3;
// use std::cmp::Ordering;

use crate::color::{Color, PointColor};
use crate::coordinate::Coordinate;
use crate::interpolate_controller::kdtree_dim;

use std::f32::consts::PI;

use crate::interpolate::inf_norm;

#[derive(Clone)]
/// Class of Points containing all necessary metadata
pub struct Points {
    /// Data is a vector of type Point, storing all coordinate and colour data
    pub data: Vec<Point>,
    /// Stores the coordinate delta between the next and prev frames
    pub delta_pos_vector: Vec<Point3<f32>>,
    /// Stores the colour delta between the next and prev frames
    pub delta_colours: Vec<Point3<f32>>,
    /// Stores the next frame as a reference for mapping count and unmapped points
    pub reference_frame: Vec<Point>,
}

impl Default for Points {
    fn default() -> Self {
        Points::new()
    }
}

impl Points {
    /// Creates new instance of Points
    pub fn new() -> Self {
        Points {
            data: Vec::new(),
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

    /// Appends new Point to stored data
    pub fn add(&mut self, elem: Point) {
        self.data.push(elem);
    }

    /// Creates new instance of Points given a vector of Point
    pub fn of(data: Vec<Point>) -> Self {
        Points {
            data,
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
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

    /// Returns new instance of Colour portion of stored data
    pub fn get_colors(self) -> Color {
        Color::new(
            self.data
                .into_iter()
                .map(|point| point.point_color)
                .collect(),
        )
    }

    /// Returns new instance of Coordinate portion of stored data
    pub fn get_coords(self) -> Coordinate {
        Coordinate::new(
            self.data
                .into_iter()
                .map(|point| point.point_coord)
                .collect(),
        )
    }

    /// Returns new instances of Coordinate and Colour portions of stored data as a tuple
    pub fn get_coords_cols(self) -> (Coordinate, Color) {
        let mut coords = Vec::new();
        let mut colors = Vec::new();
        for point in self.data {
            coords.push(point.point_coord);
            colors.push(point.point_color);
        }

        (Coordinate::new(coords), Color::new(colors))
    }

    #[cfg(feature = "dim_3")]
    /// Constructs and returns a 3D kdtree from a class of Points
    pub fn to_kdtree(self) -> KdTree<f32, usize, 3> {
        let mut kdtree: KdTree<f32, usize, 3> = KdTree::with_capacity(64).unwrap();
        let mut shuffled_points = self.data;
        shuffled_points.shuffle(&mut thread_rng());
        for point in &shuffled_points {
            kdtree
                .add(
                    &[
                        point.point_coord.x,
                        point.point_coord.y,
                        point.point_coord.z,
                    ],
                    point.index,
                )
                .unwrap();
        }
        kdtree
    }

    #[cfg(feature = "dim_6")]
    /// Constructs and returns a 6D kdtree from a class of Points
    pub fn to_kdtree(self) -> KdTree<f32, usize, 6> {
        let mut kdtree: KdTree<f32, usize, 6> = KdTree::with_capacity(64).unwrap();
        let mut shuffled_points = self.data;
        shuffled_points.shuffle(&mut thread_rng());
        for point in &shuffled_points {
            kdtree
                .add(
                    &[
                        point.point_coord.x,
                        point.point_coord.y,
                        point.point_coord.z,
                        point.point_color.red as f32,
                        point.point_color.green as f32,
                        point.point_color.blue as f32,
                    ],
                    point.index,
                )
                .unwrap();
        }
        kdtree
    }

    /// Highlights unmapped points as Green in the reference frame
    pub fn mark_unmapped_points(
        &mut self,
        kd_tree: Arc<kiddo::KdTree<f32, usize, { kdtree_dim() }>>,
        exists_output_dir: bool,
        dist_func: for<'r, 's> fn(&'r [f32], &'s [f32]) -> f32,
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
                        self.reference_frame[idx].point_color = PointColor::new(0, 255, 0);
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

    /// Highlights points in close range to cracks as Red in the interpolated frame
    pub fn mark_points_near_cracks(
        &mut self,
        point_data: &Points,
        exists_output_dir: bool,
    ) -> Points {
        let mut marked_interpolated_frame = point_data.clone();

        let mut points_near_cracks = 0;

        for idx in 0..point_data.data.len() {
            marked_interpolated_frame.data[idx].point_size = 1.0;
            if point_data.data[idx].near_crack {
                marked_interpolated_frame.data[idx].point_color = PointColor::new(255, 0, 0);
                points_near_cracks += 1;
            }
        }

        if exists_output_dir {
            println!("number of points near cracks: {}", points_near_cracks);
        }

        marked_interpolated_frame
    }

    /// Changes point size based on surrounding point density
    pub fn adjust_point_sizes(&mut self, radius: f32) {
        let interpolated_kd_tree = self.clone().to_kdtree();

        for idx in 0..self.data.len() {
            let density = interpolated_kd_tree
                .within_unsorted(
                    &[
                        self.data[idx].point_coord.x,
                        self.data[idx].point_coord.y,
                        self.data[idx].point_coord.z,
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

    /// Accepts argument of points in case this function is called in main before any interpolation function is called i.e. will be used to calculate a simple delta
    /// this function is also called in each of the interpolation functions, taking in the vector of closest points i.e. fn can be used in 2 ways
    pub fn frame_delta(&mut self, prev: Points) {
        let (next_coordinates_obj, next_colours_obj) = self.clone().get_coords_cols();

        let next_coordinates = next_coordinates_obj.get_borrow_data();
        let next_colours = next_colours_obj.get_borrow_data();

        let (prev_coordinates_obj, prev_colours_obj) = prev.get_coords_cols();

        let prev_coordinates = prev_coordinates_obj.get_borrow_data();
        let prev_colours = prev_colours_obj.get_borrow_data();

        for (pos, _e) in prev_coordinates.iter().enumerate() {
            let (x, y, z) = (
                next_coordinates[pos].x - prev_coordinates[pos].x,
                next_coordinates[pos].y - prev_coordinates[pos].y,
                next_coordinates[pos].z - prev_coordinates[pos].z,
            );
            self.delta_pos_vector.push(Point3::new(x, y, z));
        }

        for (pos, _e) in prev_colours.iter().enumerate() {
            let (x, y, z) = (
                next_colours[pos].red as f32 - prev_colours[pos].red as f32,
                next_colours[pos].green as f32 - prev_colours[pos].green as f32,
                next_colours[pos].blue as f32 - prev_colours[pos].blue as f32,
            );
            self.delta_colours.push(Point3::new(x, y, z));
        }
    }

    /// Returns clone of vector containing delta of coordinates between next and prev frames
    pub fn get_delta_pos_vector(&self) -> Vec<Point3<f32>> {
        self.delta_pos_vector.clone()
    }

    /// Returns clone of vector containing delta of colours between next and prev frames
    pub fn get_delta_colours(&self) -> Vec<Point3<f32>> {
        self.delta_colours.clone()
    }
}

impl IntoIterator for Points {
    type Item = Point;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
