use crate::point::Point;
use crate::pointcloud::PointCloud;
use crate::processing::conceal::interpolate_params::InterpolateParams;

// use kiddo::KdTree;
use nalgebra::Point3;
// use rand::seq::SliceRandom;
// use rand::thread_rng;
// use std::iter::Iterator;
use std::sync::*;
// use std::cmp::Ordering;

//use crate::color::{Color, PointColor};
//use crate::coordinate::Coordinate;
// use crate::interpolate_controller::kdtree_dim;


#[derive(Clone)]
pub struct ConcealedPointCloud {
    pub pc: PointCloud,
    /// Stores the coordinate delta between the next and prev frames
    pub delta_pos_vector: Vec<Point3<f32>>,
    /// Stores the colour delta between the next and prev frames
    pub delta_colours: Vec<Point3<f32>>,
    /// Stores the next frame as a reference for mapping count and unmapped points
    pub reference_frame: Vec<Point>,
}

impl Point {
    ///penalization
    fn get_difference(
        &self,
        another_point: &Point,
        params: &Arc<InterpolateParams>,
        ) -> f32 {
        // let max_coor: f32 = 3.0 * params.scale_coor_delta.powi(2);
        // let scale_coor = max_coor.sqrt();
        let scale_coor: f32 = 3.0_f32.sqrt() * params.scale_coor_delta;

        // let max_col: f32 = 3.0 * params.scale_col_delta.powi(2);
        // let scale_col = max_col.sqrt();
        let scale_col: f32 = 3.0_f32.sqrt() * params.scale_col_delta;

        self.get_coord_delta(another_point) * params.penalize_coor / scale_coor
            + self.get_color_delta(another_point) * params.penalize_col / scale_col
    }

    /// Return the index of the point from a set P that is closest 
    /// to this point.  P is given as an array of indices into PointCloud 
    /// object.
    /// 
    fn get_closest_index(&self,
        points: &Arc<PointCloud>,
        indices: &[usize],
        params: &Arc<InterpolateParams>,
        ) -> usize {
        let mut min: f32 = f32::MAX;
        let mut min_idx = indices[indices.len() - 1];

        // let mut result_idx = 0;
        for i in indices {
            let cur = self.get_difference(&points.data[*i], params);

            if cur <= min {
                min = cur;
                min_idx = *i;
            }
        }
        min_idx
    }

    pub fn interpolate_with_closest(
        &self,
        points: &Arc<PointCloud>,
        indices: &[usize],
        params: &Arc<InterpolateParams>,
        ) -> Point {

        if indices.is_empty() {
            return self.clone();
        }

        let idx = self.get_closest_index(points, indices, params);
        let p = &points.data[idx];
        self.get_weighted_average(p, params.prev_weight)
            // p.clone()
            // self.clone()
    }

    pub fn method_of_neighbour_query(
        &self,
        // kd_tree: &Arc<kiddo::KdTree<f32, usize, { kdtree_dim() }>>,
        kd_tree: &Arc<kiddo::KdTree<f32, usize, 3>>,
        options_for_nearest: usize,
        _radius: f32,
        dist_func: for<'r, 's> fn(&'r [f32; 3], &'s [f32; 3]) -> f32,
        ) -> Vec<usize> {
        self.get_nearest_neighbours(kd_tree, options_for_nearest, dist_func)
    }

#[cfg(feature = "by_radius")]
    /// queries neighbours by radius
    pub fn method_of_neighbour_query(
        &self,
        kd_tree: &Arc<kiddo::KdTree<f32, usize, { kdtree_dim() }>>,
        _options_for_nearest: usize,
        radius: f32,
        dist_func: for<'r, 's> fn(&'r [f32], &'s [f32]) -> f32,
        ) -> Vec<usize> {
        // let mut x = Vec::new(); x.push(self.index); if self.index + 1 < kd_tree.size() {x.push(self.index + 1);}
        // x

        self.get_radius_neghbours(kd_tree, radius, dist_func)
    }

    /// Returns neighbouring points within a given radius
    pub fn get_radius_neghbours(
        &self,
        kd_tree: &Arc<kiddo::KdTree<f32, usize, 3>>,
        radius: f32,
        dist_func: for<'r, 's> fn(&'r [f32; 3], &'s [f32; 3]) -> f32,
        ) -> Vec<usize> {
        kd_tree
        .within_unsorted(&self.get_point(), radius, &dist_func)
        .unwrap()
        .into_iter()
        .map(|found| *found.1)
        .collect()
        }
    
        /// Returns k neighboring points
        pub fn get_nearest_neighbours(
        &self,
        kd_tree: &Arc<kiddo::KdTree<f32, usize, 3>>,
        quantity: usize,
        dist_func: for<'r, 's> fn(&'r [f32; 3], &'s [f32; 3]) -> f32,
        ) -> Vec<usize> {
        kd_tree
        .nearest(&self.get_point(), quantity, &dist_func)
        .unwrap()
        .into_iter()
        .map(|found| *found.1)
        .collect()
        }
}

impl ConcealedPointCloud {
    /// Creates new instance of PointCloud
    pub fn new() -> Self {
        ConcealedPointCloud {
            pc: PointCloud::new(),
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

    /// Creates new instance of PointCloud
    pub fn new_from_point_cloud(pc: PointCloud) -> Self {
        ConcealedPointCloud {
            pc: pc.clone(),
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

    /// Creates new instance of PointCloud given a vector of Point
    pub fn of(data: Vec<Point>) -> Self {
        ConcealedPointCloud {
            pc: PointCloud::of(data),
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

}
