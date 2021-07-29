use kd_tree::{KdPoint, KdTree};
use ply_rs::ply;
use std::iter::Iterator;

use crate::color::{Color, PointColor};
use crate::coordinate::{Coordinate, PointCoordinate};
use crate::renderer::Renderer;

#[allow(unused_imports)]
use std::time::{Duration, Instant};

use nalgebra::Point3;
use std::any::type_name;
use std::cmp::Ordering;

use std::f32::consts::PI;

use crate::filter::FilterProducer;
use crate::transform::TransformProducer;

fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}

#[derive(Clone)]
pub struct Points {
    pub data: Vec<Point>,
    pub delta_pos_vector: Vec<Point3<f32>>,
    pub delta_colours: Vec<Point3<f32>>,
    pub reference_frame: Vec<Point>,
}

impl Default for Points {
    fn default() -> Self {
        Points::new()
    }
}

impl Points {
    pub fn new() -> Self {
        Points {
            data: Vec::new(),
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

    pub fn add(&mut self, elem: Point) {
        self.data.push(elem);
    }

    pub fn of(data: Vec<Point>) -> Self {
        Points {
            data,
            delta_pos_vector: Vec::new(),
            delta_colours: Vec::new(),
            reference_frame: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn get_data(self) -> Vec<Point> {
        self.data
    }

    pub fn get_clone_data(&self) -> Vec<Point> {
        self.data.clone()
    }

    pub fn get_colors(self) -> Color {
        Color::new(
            self.data
                .into_iter()
                .map(|point| point.point_color)
                .collect(),
        )
    }

    pub fn get_coords(self) -> Coordinate {
        Coordinate::new(
            self.data
                .into_iter()
                .map(|point| point.point_coord)
                .collect(),
        )
    }

    pub fn get_coords_cols(self) -> (Coordinate, Color) {
        let mut coords = Vec::new();
        let mut colors = Vec::new();
        for point in self.data {
            coords.push(point.point_coord);
            colors.push(point.point_color);
        }

        (Coordinate::new(coords), Color::new(colors))
    }

    pub fn render(&self) {
        self.render_with_camera(None, None)
    }

    pub fn render_with_camera(&self, eye: Option<Point3<f32>>, at: Option<Point3<f32>>) {
        let mut renderer = Renderer::new();

        renderer.config_camera(eye, at);

        while renderer.render() {
            renderer.render_frame(&self);
        }
    }

    // pub fn take_sreenshoot_to_path(&self, path: &str) {
    //     let mut renderer = Renderer::new();
    //     renderer.rendering();
    //     renderer.render_frame(&self);
    //     renderer.rendering();
    //     renderer.render_frame(&self);
    //     renderer.rendering();
    //     renderer.screenshoot_to_path(path);
    // }

    pub fn to_kdtree(self) -> KdTree<Point> {
        KdTree::build_by_ordered_float(self.data)
    }

    pub fn mark_unmapped_points(&mut self, kd_tree: KdTree<Point>) {
        let mut mapped_points = 0;
        let mut all_unmapped: bool = true;

        for point in self.reference_frame.clone().iter_mut() {
            if point.mapping == 0 {
                let k_nearest_indices = point.get_nearests(&kd_tree, 3);
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

        println!(
            "mapped points: {}; total points: {}",
            mapped_points,
            self.reference_frame.len()
        );
    }

    pub fn mark_points_near_cracks(&mut self, point_data: &Points) -> Points {
        let mut marked_interpolated_frame = point_data.clone();

        let mut points_near_cracks = 0;

        for idx in 0..point_data.data.len() {
            marked_interpolated_frame.data[idx].point_size = 1.0;
            if point_data.data[idx].near_crack {
                //self.reference_frame[point.get_index()].point_color = PointColor::new(255, 0, 0);
                marked_interpolated_frame.data[idx].point_color = PointColor::new(255, 0, 0);
                points_near_cracks += 1;
            }
        }

        println!("number of points near cracks: {}", points_near_cracks);
        marked_interpolated_frame
    }

    //changing point size based on surrounding point density
    pub fn adjust_point_sizes(&mut self, radius: f32) {
        let interpolated_kd_tree = self.clone().to_kdtree();

        for idx in 0..self.data.len() {
            let density = interpolated_kd_tree
                .within_radius(&self.data[idx], radius)
                .len() as f32
                / (radius.powf(2.0) * PI);

            if density <= self.data[idx].point_density {
                self.data[idx].near_crack = true;
                self.data[idx].point_size = 2.0;
            }
        }
    }

    //deprecated interoolation method. closest_with_ratio_average_points_recovery can achieve this by setting opitons for nearest to 1
    // pub fn average_points_recovery(&mut self, points: Points) -> (Points, Points) {
    //     self.reference_frame = points.clone().get_data();

    //     let kd_tree = points.to_kdtree();
    //     let x = self.clone();

    //     let point_data = Points::of(x.get_data().into_iter()
    //         .map(|point| point.get_average(&point.get_nearest(&kd_tree, &mut self.reference_frame)))
    //         .collect());

    //     self.frame_delta(point_data.clone());

    //     self.mark_unmapped_points(kd_tree);

    //     (point_data, Points::of(self.reference_frame.clone()))
    // }

    pub fn closest_with_ratio_average_points_recovery(
        &mut self,
        next_points: &Points,
        penalize_coor: f32,
        penalize_col: f32,
        penalize_mapped: f32,
        radius: f32,
        options_for_nearest: usize,
        show_unmapped_points: bool,
        resize_near_cracks: bool,
        mark_enlarged: bool,
        compute_frame_delta: bool,
    ) -> (Points, Points, Points) {
        //start time
        let now = Instant::now();
        // if show_unmapped_points {self.reference_frame = next_points.data.clone(); }
        self.reference_frame = next_points.data.clone();

        let kd_tree = next_points.clone().to_kdtree();
        let x = self.data.clone();

        // println!("cloning time: {}", now.elapsed().as_secs());

        let mut point_data = Points::of(
            x.into_iter()
                .map(|point| {
                    point.get_average_closest_from_kdtree(
                        &next_points,
                        &kd_tree,
                        penalize_coor,
                        penalize_col,
                        &mut self.reference_frame,
                        penalize_mapped,
                        radius,
                        options_for_nearest,
                    )
                })
                .collect(),
        );

        println!("interpolation time: {}", now.elapsed().as_secs());

        if compute_frame_delta {
            self.frame_delta(point_data.clone());
        }

        if show_unmapped_points {
            self.mark_unmapped_points(kd_tree);
        }

        /////////////
        //point_data.render(); //original interpolated frame
        /////////////

        if resize_near_cracks {
            point_data.adjust_point_sizes(radius);
        }

        let marked_interpolated_frame = Points::new();
        if resize_near_cracks && mark_enlarged {
            let _marked_interpolated_frame = self.mark_points_near_cracks(&point_data);
        }

        (
            point_data,
            Points::of(self.reference_frame.clone()),
            marked_interpolated_frame,
        )
    }

    //accepts argument of points in case this function is called in main before any interpolation function is called i.e. will be used to calculate a simple delta
    // this function is also called in each of the interpolation functions, taking in the vector of closest points i.e. fn can be used in 2 ways
    pub fn frame_delta(&mut self, prev: Points) {
        // let next_coordinates_obj = self.clone().get_coords();
        // let next_colours_obj = self.clone().get_colors();

        let (next_coordinates_obj, next_colours_obj) = self.clone().get_coords_cols();

        let next_coordinates = next_coordinates_obj.get_point_coor_vec();
        let next_colours = next_colours_obj.get_point_col_vec();

        // let prev_coordinates_obj = prev.clone().get_coords();
        // let prev_colours_obj = prev.get_colors();

        let (prev_coordinates_obj, prev_colours_obj) = prev.get_coords_cols();

        let prev_coordinates = prev_coordinates_obj.get_point_coor_vec();
        let prev_colours = prev_colours_obj.get_point_col_vec();

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

    pub fn get_delta_pos_vector(&self) -> Vec<Point3<f32>> {
        self.delta_pos_vector.clone()
    }

    pub fn get_delta_colours(&self) -> Vec<Point3<f32>> {
        self.delta_colours.clone()
    }

    pub fn fat(
        &self,
        filter_producer: &FilterProducer,
        transform_producer: &TransformProducer,
        transform_producer_remain: &TransformProducer,
    ) -> Points {
        let mut res = Points::new();
        let filter = filter_producer(self);
        let change = transform_producer(self);
        let change_remain = transform_producer_remain(self);

        for point in &self.data {
            if filter(point) {
                res.add(change(point))
            } else {
                res.add(change_remain(point))
            }
        }
        res
    }
}

impl IntoIterator for Points {
    type Item = Point;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct Point {
    pub point_coord: PointCoordinate,
    pub point_color: PointColor,
    pub mapping: u16,
    pub index: usize,
    pub point_density: f32,
    pub point_size: f32,
    pub near_crack: bool,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.point_coord == other.point_coord && self.point_color == other.point_color
    }
}

impl Point {
    pub fn new(
        point_coord: PointCoordinate,
        point_color: PointColor,
        mapping: u16,
        index: usize,
        point_density: f32,
        point_size: f32,
        near_crack: bool,
    ) -> Self {
        Point {
            point_coord,
            point_color,
            mapping,
            index,
            point_density,
            point_size,
            near_crack,
        }
    }

    pub fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        let mut first_dist_from_ori = f32::powf(self.point_coord.x, 2.0)
            + f32::powf(self.point_coord.y, 2.0)
            + f32::powf(self.point_coord.z, 2.0);
        first_dist_from_ori = first_dist_from_ori.sqrt();

        let next_dist_from_ori = f32::powf(other.point_coord.x, 2.0)
            + f32::powf(other.point_coord.y, 2.0)
            + f32::powf(other.point_coord.z, 2.0);

        if first_dist_from_ori < next_dist_from_ori {
            return Some(Ordering::Less);
        } else if first_dist_from_ori > next_dist_from_ori {
            return Some(Ordering::Greater);
        }

        Some(Ordering::Equal)
    }

    pub fn new_default() -> Self {
        Point {
            point_coord: PointCoordinate::new_default(),
            point_color: PointColor::new_default(),
            mapping: 0,
            index: 0,
            point_density: 0.0,
            point_size: 1.0,
            near_crack: false,
        }
    }

    pub fn get_coord(&self) -> &PointCoordinate {
        &self.point_coord
    }

    pub fn get_color(&self) -> &PointColor {
        &self.point_color
    }

    pub fn set_index(&mut self, idx: usize) {
        self.index = idx;
    }

    pub fn get_index(&mut self) -> usize {
        self.index
    }

    //deprecated
    // pub fn get_nearest(&self, kd_tree: &KdTree<Point>, reference_frame: &mut Vec<Point>) -> Point {
    //     let mut nearest_point = kd_tree.nearest(self).unwrap().item.clone();
    //     reference_frame[nearest_point.get_index()].mapping += 1;

    //     nearest_point
    // }

    //penalization
    //update count in kdtree point
    pub fn get_nearests(&self, kd_tree: &KdTree<Point>, quantity: usize) -> Vec<usize> {
        kd_tree
            .nearests(self, quantity)
            .into_iter()
            .map(|found| found.item.index)
            .collect()
    }

    pub fn get_average(&self, another_point: &Point) -> Point {
        Point::new(
            self.point_coord.get_average(&another_point.point_coord),
            self.point_color.get_average(&another_point.point_color),
            0,
            another_point.index,
            another_point.point_density, //(self.point_density + another_point.point_density) / 2.0)
            (self.point_size + another_point.point_size) / 2.0,
            false,
        )
    }

    fn get_coord_delta(&self, another_point: &Point) -> f32 {
        self.point_coord.get_coord_delta(&another_point.point_coord)
    }

    fn get_color_delta(&self, another_point: &Point) -> f32 {
        self.point_color.get_color_delta(&another_point.point_color)
    }

    ///penalization
    // #[inline(always)]
    fn get_difference(
        &self,
        another_point: &Point,
        penalize_coor: f32,
        penalize_col: f32,
        another_point_mapping: u16,
        penalize_mapped: f32,
    ) -> f32 {
        let max_coor: f32 = 3.0 * 512.0 * 512.0;
        let scale_coor = max_coor.sqrt();

        let max_col: f32 = (100.0 * 100.0) + 2.0 * (256.0 * 256.0);
        let scale_col = max_col.sqrt();

        self.get_coord_delta(another_point) * penalize_coor / scale_coor
            + self.get_color_delta(another_point) * penalize_col / scale_col
            + another_point_mapping as f32 * penalize_mapped
    }

    fn get_closest(
        &self,
        next_points: &Points,
        k_nearest_indices: Vec<usize>,
        penalize_coor: f32,
        penalize_col: f32,
        reference_frame: &mut Vec<Point>,
        penalize_mapped: f32,
        kd_tree: &KdTree<Point>,
        radius: f32,
    ) -> Point {
        let mut min: f32 = f32::MAX;
        let mut result: Point; // = Point::new_default();

        let mut result_idx = 0;
        for idx in k_nearest_indices {
            let cur = self.get_difference(
                &next_points.data[idx],
                penalize_coor,
                penalize_col,
                reference_frame[idx].mapping,
                penalize_mapped,
            );
            if cur < min {
                min = cur;
                result_idx = idx;
            }
        }

        result = next_points.data[result_idx].clone();
        result.point_density =
            kd_tree.within_radius(&result, radius).len() as f32 / (radius.powf(2.0) * PI);
        reference_frame[result_idx].mapping += 1;
        result
    }

    fn get_average_closest(
        &self,
        next_points: &Points,
        k_nearest_indices: Vec<usize>,
        penalize_coor: f32,
        penalize_col: f32,
        reference_frame: &mut Vec<Point>,
        penalize_mapped: f32,
        kd_tree: &KdTree<Point>,
        radius: f32,
    ) -> Point {
        let p = &self.get_closest(
            next_points,
            k_nearest_indices,
            penalize_coor,
            penalize_col,
            reference_frame,
            penalize_mapped,
            kd_tree,
            radius,
        );
        self.get_average(p)
    }

    fn get_average_closest_from_kdtree(
        &self,
        next_points: &Points,
        kd_tree: &KdTree<Point>,
        penalize_coor: f32,
        penalize_col: f32,
        reference_frame: &mut Vec<Point>,
        penalize_mapped: f32,
        radius: f32,
        options_for_nearest: usize,
    ) -> Point {
        self.get_average_closest(
            next_points,
            self.get_nearests(kd_tree, options_for_nearest),
            penalize_coor,
            penalize_col,
            reference_frame,
            penalize_mapped,
            kd_tree,
            radius,
        )
    }
}

impl ply::PropertyAccess for Point {
    fn new() -> Self {
        Point::new_default()
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.point_coord.x = v,
            ("y", ply::Property::Float(v)) => self.point_coord.y = v,
            ("z", ply::Property::Float(v)) => self.point_coord.z = v,
            ("red", ply::Property::UChar(v)) => self.point_color.red = v,
            ("green", ply::Property::UChar(v)) => self.point_color.green = v,
            ("blue", ply::Property::UChar(v)) => self.point_color.blue = v,
            (k, _) => panic!("Vertex: Unexpected key/value combination: key: {}", k),
        }
    }
}

impl KdPoint for Point {
    type Scalar = f32;
    type Dim = typenum::U3; // 3 dimensional tree.
    fn at(&self, k: usize) -> f32 {
        match k {
            0 => self.point_coord.x,
            1 => self.point_coord.y,
            2 => self.point_coord.z,
            _ => panic!("Oh no, don't have {}", k),
        }
    }
}
