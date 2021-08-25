use crate::errors::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::mpsc;
use std::thread;

use kiddo::distance::squared_euclidean;
use kiddo::KdTree;

use ply_rs::ply;
use std::iter::Iterator;

use crate::params::Params;

use nalgebra::Point3;
// use std::any::type_name;
use std::cmp::Ordering;

use crate::color::{Color, PointColor};
use crate::coordinate::{Coordinate, PointCoordinate};
use crate::filter::FilterProducer;
use crate::tool::renderer::Renderer;
use crate::transform::TransformProducer;
use crate::Instant;

use ply_rs::ply::{
    Addable, DefaultElement, ElementDef, Encoding, Ply, Property, PropertyDef, PropertyType,
    ScalarType,
};

use ply_rs::writer::Writer;
use std::f32::consts::PI;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

// fn type_of<T>(_: T) -> &'static str {
//     type_name::<T>()
// }

pub fn inf_norm(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let max: f32;
    let dx = (a[0] - b[0]).abs();
    let dy = (a[1] - b[1]).abs();
    let dz = (a[2] - b[2]).abs();
    if dx > dy {
        max = dx;
    } else {
        max = dy;
    }
    if max > dz {
        max
    } else {
        dz
    }
}

pub fn setup_run_indiv_thread_closest_points(
    tx: std::sync::mpsc::Sender<Vec<Point>>,
    slices: &mut std::slice::Chunks<Point>,
    kd_tree: std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    options_for_nearest: usize,
    next_points: std::sync::Arc<Points>,
    params: std::sync::Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> std::thread::JoinHandle<()> {
    // let kd = kd_tree.clone();
    let slice = slices.next().unwrap().to_owned();

    // let now = Instant::now();
    let mut refer = reference_frame.clone();
    // println!("cloning time: {}", now.elapsed().as_millis());

    // let now = Instant::now();
    thread::spawn(move || {
        // let kd_arc_clone = kd_tree.clone();
        // let next_points_clone = next_points.clone();
        // let params_clone = params.clone();
        // let mut nearests: Vec<usize> = Vec::new();
        let mut closests: Vec<Point> = Vec::with_capacity(100);
        for s in &slice {
            let nearests = s.method_of_neighbour_query(&kd_tree, options_for_nearest);
            let p = s.get_average_closest(&next_points, &nearests, &mut refer, &params);
            closests.push(p);
        }
        // println!("time for 1 thread to finish: {}", now.elapsed().as_millis());
        tx.send(closests).unwrap();
    })
}

pub fn run_threads(
    threads: usize,
    slices: &mut std::slice::Chunks<Point>,
    kd_tree: &std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    options_for_nearest: usize,
    next_points: std::sync::Arc<Points>,
    params: &std::sync::Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> Vec<Point> {
    let mut vrx: Vec<std::sync::mpsc::Receiver<Vec<Point>>> = Vec::with_capacity(12);
    let mut vhandle: Vec<std::thread::JoinHandle<()>> = Vec::with_capacity(12);

    // let now = Instant::now();

    for _i in 0..threads {
        let (tx, rx): (
            std::sync::mpsc::Sender<Vec<Point>>,
            std::sync::mpsc::Receiver<Vec<Point>>,
        ) = mpsc::channel();
        vrx.push(rx);
        let handle = setup_run_indiv_thread_closest_points(
            tx,
            slices,
            kd_tree.clone(),
            options_for_nearest,
            next_points.clone(),
            params.clone(),
            reference_frame,
        );
        vhandle.push(handle);
    }

    // println!("time to spawn threads: {}", now.elapsed().as_millis());

    for handle in vhandle {
        handle.join().unwrap();
    }

    // println!(
    //     "closest point computation time: {}",
    //     now.elapsed().as_millis()
    // );

    let mut result: Vec<Point> = Vec::with_capacity(100000);

    for rx in vrx {
        result.extend(rx.recv().unwrap());
    }
    // println!(
    //     "full closest comp and vector extension: {}",
    //     now.elapsed().as_millis()
    // );
    result
}
pub fn parallel_query_closests(
    data_copy: &[Point],
    kd_tree: &std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    threads: usize,
    options_for_nearest: usize,
    next_points: std::sync::Arc<Points>,
    params: &std::sync::Arc<Params>,
    reference_frame: &mut Vec<Point>,
) -> Vec<Point> {
    let mut slices = data_copy.chunks((data_copy.len() as f32 / threads as f32).ceil() as usize);

    run_threads(
        threads,
        &mut slices,
        kd_tree,
        options_for_nearest,
        next_points,
        params,
        reference_frame,
    )
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
        self.do_render(None, None)
    }

    pub fn do_render(&self, eye: Option<Point3<f32>>, at: Option<Point3<f32>>) {
        let mut renderer = Renderer::new(None);

        renderer.config_camera(eye, at);

        renderer.render_image(&self);
    }

    pub fn save_to_png(
        &self,
        eye: Option<Point3<f32>>,
        at: Option<Point3<f32>>,
        x: Option<u32>,
        y: Option<u32>,
        width: Option<u32>,
        height: Option<u32>,
        path: Option<&str>,
    ) -> Result<()> {
        let mut renderer = Renderer::new(None);

        renderer.config_camera(eye, at);

        renderer.save_to_png(&self, x, y, width, height, path)?;

        Ok(())
    }

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

    pub fn mark_unmapped_points(
        &mut self,
        kd_tree: std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    ) {
        let mut mapped_points = 0;
        let mut all_unmapped: bool = true;

        for point in self.reference_frame.clone().iter_mut() {
            if point.mapping == 0 {
                let k_nearest_indices = point.get_nearest_neighbours(kd_tree.clone(), 3);
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

    pub fn closest_with_ratio_average_points_recovery(
        &mut self,
        next_points: Points,
        params: Params,
        exists_output_dir: bool
    ) -> (Points, Points, Points) {
        //start time
        let now = Instant::now();
        self.reference_frame = next_points.data.clone();
        // println!("ref frame cloning: {}", now.elapsed().as_millis());
        let kd_tree = next_points.clone().to_kdtree();

        //    println!("kd tree constrcution: {}", now.elapsed().as_millis());

        // let mutex_tree = std::sync::Mutex::new(kd_tree);
        let arc_tree = std::sync::Arc::new(kd_tree);
        // let kd = 'static kd_tree;
        let arc_next_points = std::sync::Arc::new(next_points);
        let arc_params = std::sync::Arc::new(params);
        // println!("arc cloning: {}", now.elapsed().as_millis());
        let data_copy = self.data.clone();
        let interpolated_points = Vec::new();

        if data_copy.len() != 0 {
            let interpolated_points = parallel_query_closests(
                &data_copy,
                &arc_tree,
                params.threads,
                arc_params.options_for_nearest,
                arc_next_points,
                &arc_params,
                &mut self.reference_frame,
            );
        }

        // println!("PRE INTERPOLATION RUNTIME: {}", now.elapsed().as_millis());

        // let mut point_data = Points::of(
        //     data_copy
        //         .into_iter()
        //         .map(|point| {
        //             point.get_average_closest(
        //                 next_points,
        //                 &all_nearests[point.index],
        //                 &mut self.reference_frame,
        //                 params,
        //             )
        //         })
        //         .collect(),
        // );

        // let point_data = parallel_compute_closest(data_copy, next_points, &all_nearests, &mut self.reference_frame, params, threads);

        if exists_output_dir{
            println!("interpolation time: {}", now.elapsed().as_millis());
        }

        let mut point_data = Points::of(interpolated_points);
        if arc_params.compute_frame_delta {
            self.frame_delta(point_data.clone());
        }

        if arc_params.show_unmapped_points {
            self.mark_unmapped_points(arc_tree);
        }

        /////////////
        //point_data.render(); //original interpolated frame
        /////////////

        if arc_params.resize_near_cracks {
            point_data.adjust_point_sizes(arc_params.radius);
        }

        let marked_interpolated_frame = Points::new();
        if arc_params.resize_near_cracks && arc_params.mark_enlarged {
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

    pub fn get_delta_pos_vector(&self) -> Vec<Point3<f32>> {
        self.delta_pos_vector.clone()
    }

    pub fn get_delta_colours(&self) -> Vec<Point3<f32>> {
        self.delta_colours.clone()
    }

    pub fn fat(
        &self,
        filter_producer: Option<&FilterProducer>,
        transform_producer: Option<&TransformProducer>,
        transform_producer_remain: Option<&TransformProducer>,
    ) -> Result<Points> {
        let mut res = Points::new();
        let filter = filter_producer.chain_err(|| "Filter method not found")?(self);
        let change = transform_producer.chain_err(|| "Transform method not found")?(self);
        let change_remain =
            transform_producer_remain.chain_err(|| "Transform method for remain not found")?(self);

        for point in &self.data {
            if filter(point) {
                res.add(change(point))
            } else {
                res.add(change_remain(point))
            }
        }
        Ok(res)
    }

    pub fn read(input: Option<&str>) -> std::io::Result<()> {
        // match input {
        //     Some(path) => {
        //         File::open(Path::new(path)).unwrap();
        //     }
        //     None => {}
        // };
        if let Some(path) = input {
            File::open(Path::new(path)).unwrap();
        };

        Ok(())
    }

    pub fn write(self, form: Option<&str>, output: Option<&str>) -> Result<()> {
        let encoding = match form {
            Some("ascii") => Some(Encoding::Ascii),
            Some("binary") => Some(Encoding::BinaryLittleEndian),
            Some(&_) => None,
            None => Some(Encoding::Ascii),
        };

        let mut buf = Vec::<u8>::new();

        let mut ply = {
            let mut ply = Ply::<DefaultElement>::new();
            ply.header.encoding = encoding.chain_err(|| "Invalid ply encoding form")?;
            ply.header.comments.push("A beautiful comment!".to_string());

            let mut point_element = ElementDef::new("vertex".to_string());
            let p = PropertyDef::new("x".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("y".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("z".to_string(), PropertyType::Scalar(ScalarType::Float));
            point_element.properties.add(p);
            let p = PropertyDef::new("red".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("green".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            let p = PropertyDef::new("blue".to_string(), PropertyType::Scalar(ScalarType::UChar));
            point_element.properties.add(p);
            ply.header.elements.add(point_element);

            let mut points = Vec::new();

            for entry in self.get_data() {
                let coord = entry.get_coord();
                let color = entry.get_color();

                let mut point = DefaultElement::new();
                point.insert("x".to_string(), Property::Float(coord.x));
                point.insert("y".to_string(), Property::Float(coord.y));
                point.insert("z".to_string(), Property::Float(coord.z));
                point.insert("red".to_string(), Property::UChar(color.red));
                point.insert("green".to_string(), Property::UChar(color.green));
                point.insert("blue".to_string(), Property::UChar(color.blue));
                points.push(point);
            }

            ply.payload.insert("vertex".to_string(), points);
            ply.make_consistent().unwrap();
            ply
        };

        let w = Writer::new();
        w.write_ply(&mut buf, &mut ply).unwrap();

        match output {
            Some(path) => {
                File::create(Path::new(path))
                    .chain_err(|| "Cannot create path")?
                    .write_all(&buf)?;
            }
            None => {
                io::stdout().write_all(&buf)?;
            }
        };

        Ok(())
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
        let mut first_dist_from_ori = f32::powi(self.point_coord.x, 2)
            + f32::powi(self.point_coord.y, 2)
            + f32::powi(self.point_coord.z, 2);
        first_dist_from_ori = first_dist_from_ori.sqrt();

        let next_dist_from_ori = f32::powi(other.point_coord.x, 2)
            + f32::powi(other.point_coord.y, 2)
            + f32::powi(other.point_coord.z, 2);

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

    //penalization
    //update count in kdtree point
    pub fn get_radius_neghbours(
        &self,
        kd_tree: &std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
    ) -> Vec<usize> {
        kd_tree
            .within_unsorted(
                &[self.point_coord.x, self.point_coord.y, self.point_coord.z],
                2.0,
                &inf_norm,
            )
            .unwrap()
            .into_iter()
            .map(|found| *found.1)
            .collect()
    }
    pub fn get_nearest_neighbours(
        &self,
        kd_tree: std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
        quantity: usize,
    ) -> Vec<usize> {
        kd_tree
            .nearest(
                &[self.point_coord.x, self.point_coord.y, self.point_coord.z],
                quantity,
                &squared_euclidean,
            )
            .unwrap()
            .into_iter()
            .map(|found| *found.1)
            .collect()
    }

    pub fn get_average(&self, another_point: &Point) -> Point {
        Point::new(
            self.point_coord.get_average(&another_point.point_coord),
            self.point_color.get_average(&another_point.point_color),
            0,
            another_point.index,
            another_point.point_density,
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
    fn get_difference(
        &self,
        another_point: &Point,
        another_point_mapping: u16,
        params: &std::sync::Arc<Params>,
    ) -> f32 {
        let max_coor: f32 = 3.0 * 512.0 * 512.0;
        let scale_coor = max_coor.sqrt();

        let max_col: f32 = (100.0 * 100.0) + 2.0 * (256.0 * 256.0);
        let scale_col = max_col.sqrt();

        self.get_coord_delta(another_point) * params.penalize_coor / scale_coor
            + self.get_color_delta(another_point) * params.penalize_col / scale_col
            + another_point_mapping as f32 * params.penalize_mapped
    }

    fn get_closest(
        &self,
        next_points: &std::sync::Arc<Points>,
        k_nearest_indices: &[usize],
        reference_frame: &mut Vec<Point>,
        params: &std::sync::Arc<Params>,
    ) -> Point {
        let mut min: f32 = f32::MAX;
        let mut result: Point;

        let mut result_idx = 0;
        for idx in k_nearest_indices {
            let cur = self.get_difference(
                &next_points.data[*idx],
                reference_frame[*idx].mapping,
                params,
            );
            if cur < min {
                min = cur;
                result_idx = *idx;
            }
        }

        result = next_points.data[result_idx].clone();

        //This is point density in t0
        result.point_density = k_nearest_indices.len() as f32 / (params.radius.powi(2) * PI);
        reference_frame[result_idx].mapping += 1;
        result
    }

    fn get_average_closest(
        &self,
        next_points: &std::sync::Arc<Points>,
        k_nearest_indices: &[usize],
        reference_frame: &mut Vec<Point>,
        params: &std::sync::Arc<Params>,
    ) -> Point {
        if k_nearest_indices.is_empty() {
            return self.clone();
        }

        let p = &self.get_closest(next_points, k_nearest_indices, reference_frame, params);
        self.get_average(p)
    }

    #[cfg(feature = "by_knn")]
    pub fn method_of_neighbour_query(
        &self,
        kd_tree: &KdTree<f32, usize, 3>,
        options_for_nearest: usize,
    ) -> Vec<usize> {
        self.get_nearest_neighbours(&kd_tree, options_for_nearest)
    }

    #[cfg(feature = "by_radius")]
    pub fn method_of_neighbour_query(
        &self,
        kd_tree: &std::sync::Arc<kiddo::KdTree<f32, usize, 3_usize>>,
        _options_for_nearest: usize,
    ) -> Vec<usize> {
        self.get_radius_neghbours(kd_tree)
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
