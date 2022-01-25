use crate::points::PointCloud;
use ply_rs::ply;
use std::sync::Arc;
// use std::thread;
use crate::color::PointColor;
use crate::coordinate::PointCoordinate;
// use crate::interpolate::inf_norm;
use crate::params::Params;
// use kiddo::distance::squared_euclidean;
// use crate::interpolate_controller::kdtree_dim;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
/// Structure presenting a point
pub struct Point {
    pub(crate) coord: PointCoordinate,
    pub(crate) color: PointColor,
    pub(crate) mapping: u16,
    pub(crate) index: usize,
    pub(crate) point_density: f32,
    pub(crate) point_size: f32,
    pub(crate) near_crack: bool,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord && self.color == other.color
    }
}

impl Point {
    pub(crate) fn new(
        coord: PointCoordinate,
        color: PointColor,
        mapping: u16,
        index: usize,
        point_density: f32,
        point_size: f32,
        near_crack: bool,
    ) -> Self {
        Point {
            coord,
            color,
            mapping,
            index,
            point_density,
            point_size,
            near_crack,
        }
    }

    pub(crate) fn new_default() -> Self {
        Point {
            coord: PointCoordinate::new_default(),
            color: PointColor::new_default(),
            mapping: 0,
            index: 0,
            point_density: 0.0,
            point_size: 1.0,
            near_crack: false,
        }
    }

    pub(crate) fn get_coord(&self) -> &PointCoordinate {
        &self.coord
    }

    pub(crate) fn get_color(&self) -> &PointColor {
        &self.color
    }

    pub(crate) fn set_index(&mut self, idx: usize) {
        self.index = idx;
    }

    pub fn get_point(&self) -> [f32; 3] {
        [self.coord.x, self.coord.y, self.coord.z]
    }

    pub fn get_6dpoint(&self) -> [f32; 6] {
        [
            self.coord.x,
            self.coord.y,
            self.coord.z,
            self.color.red as f32,
            self.color.green as f32,
            self.color.blue as f32,
        ]
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

    /// Returns a Point whose coordinates and colours are the average of 2 given points
    pub fn get_average(&self, another_point: &Point, prev_weight: f32, next_weight: f32) -> Point {
        Point::new(
            self.coord
                .get_average(&another_point.coord, prev_weight, next_weight),
            self.color
                .get_average(&another_point.color, prev_weight, next_weight),
            0,
            another_point.index,
            another_point.point_density,
            (self.point_size + another_point.point_size) / 2.0,
            false,
        )
    }

    fn get_coord_delta(&self, another_point: &Point) -> f32 {
        self.coord.get_coord_delta(&another_point.coord)
    }

    fn get_color_delta(&self, another_point: &Point) -> f32 {
        self.color.get_color_delta(&another_point.color)
    }

    ///penalization
    fn get_difference(
        &self,
        another_point: &Point,
        another_point_mapping: u16,
        params: &Arc<Params>,
    ) -> f32 {
        // let max_coor: f32 = 3.0 * params.scale_coor_delta.powi(2);
        // let scale_coor = max_coor.sqrt();
        let scale_coor: f32 = 3.0_f32.sqrt() * params.scale_coor_delta;

        // let max_col: f32 = 3.0 * params.scale_col_delta.powi(2);
        // let scale_col = max_col.sqrt();
        let scale_col: f32 = 3.0_f32.sqrt() * params.scale_col_delta;

        self.get_coord_delta(another_point) * params.penalize_coor / scale_coor
            + self.get_color_delta(another_point) * params.penalize_col / scale_col
            + another_point_mapping as f32 * params.penalize_mapped
    }

    fn get_closest(
        &self,
        next_points: &Arc<PointCloud>,
        k_nearest_indices: &[usize],
        reference_frame: &mut Vec<Point>,
        params: &Arc<Params>,
    ) -> Point {
        let mut min: f32 = f32::MAX;
        let mut result: Point;

        // let mut result_idx = 0;
        let mut result_idx = k_nearest_indices[k_nearest_indices.len() - 1];
        for idx in k_nearest_indices {
            let cur = self.get_difference(
                &next_points.data[*idx],
                reference_frame[*idx].mapping,
                params,
            );

            if cur < min || ((cur - min).abs() < f32::MIN_POSITIVE && *idx < result_idx) {
                min = cur;
                result_idx = *idx;
            }
        }

        result = next_points.data[result_idx].clone();

        //This is point density in t0
        result.point_density =
            k_nearest_indices.len() as f32 / (params.density_radius.powi(2) * PI);
        reference_frame[result_idx].mapping += 1;
        result
    }

    pub fn get_average_closest(
        &self,
        next_points: &Arc<PointCloud>,
        k_nearest_indices: &[usize],
        reference_frame: &mut Vec<Point>,
        params: &Arc<Params>,
    ) -> Point {
        if k_nearest_indices.is_empty() {
            return self.clone();
        }

        let p = &self.get_closest(next_points, k_nearest_indices, reference_frame, params);
        self.get_average(p, params.prev_weight, params.next_weight)
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
}

impl ply::PropertyAccess for Point {
    fn new() -> Self {
        Point::new_default()
    }

    fn set_property(&mut self, key: &String, property: ply::Property) {
        match (key.as_str(), property) {
            ("x", ply::Property::Float(v)) => self.coord.x = v,
            ("y", ply::Property::Float(v)) => self.coord.y = v,
            ("z", ply::Property::Float(v)) => self.coord.z = v,
            ("red", ply::Property::UChar(v)) => self.color.red = v,
            ("green", ply::Property::UChar(v)) => self.color.green = v,
            ("blue", ply::Property::UChar(v)) => self.color.blue = v,
            (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
        }
    }
}
