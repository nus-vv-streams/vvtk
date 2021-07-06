use ply_rs::ply;
use kd_tree::{ KdPoint, KdTree };
use std::iter::Iterator;

use crate::color::{ Color, PointColor };
use crate::coordinate::{ Coordinate, PointCoordinate };
use crate::renderer;

#[derive(Clone)]
pub struct Points {
    pub data: Vec<Point>
}

impl Points {
    pub fn new() -> Self {
        Points {
            data: Vec::new()
        }
    }

    pub fn add(&mut self, elem: Point) {
        self.data.push(elem);
    }

    pub fn of(data: Vec<Point>) -> Self {
        Points {
            data: data
        }
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }

    pub fn get_data(self) -> Vec<Point> {
        self.data
    }

    pub fn get_colors(self) -> Color {
        Color::new(self.data.into_iter().map(|point| point.point_color).collect())
    }

    pub fn get_coords(self) -> Coordinate {
        Coordinate::new(self.data.into_iter().map(|point| point.point_coord).collect())
    }

    pub fn get_both(self) -> (Coordinate, Color) {
        let mut coords = Vec::new();
        let mut colors = Vec::new();
        for point in self.data {
            coords.push(point.point_coord);
            colors.push(point.point_color);
        }

        (Coordinate::new(coords), Color::new(colors))
    }

    pub fn render(&self) {
        let mut renderer = renderer::Renderer::new();
        while renderer.rendering() {
            renderer.render_frame(&self);
        }
    }

    pub fn to_kdtree(self) -> KdTree<Point>{
        KdTree::build_by_ordered_float(self.get_data())
    }

    pub fn average_points_recovery(self, points: Points) -> Points {
        let kd_tree = points.to_kdtree();

        Points::of(self.get_data().into_iter()
            .map(|point| point.get_average(&point.get_nearest(&kd_tree)))
            .collect())
    }

    pub fn closest_with_ratio_average_points_recovery(self, points: Points, ratio: f32) -> Points{
        let kd_tree = points.to_kdtree();

        Points::of(self.get_data().into_iter()
                    .map(|point| point.get_average_closest_from_kdtree(&kd_tree, ratio))
                    .collect())
    }
}

impl IntoIterator for Points {
    type Item = Point;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct Point {
    pub point_coord: PointCoordinate,
    pub point_color: PointColor
}

impl Point {
    pub fn new(point_coord: PointCoordinate, point_color: PointColor) -> Self {
        Point {
            point_coord: point_coord,
            point_color: point_color
        }
    }

    fn new_default() -> Self {
        Point {
            point_coord: PointCoordinate::new_default(),
            point_color: PointColor::new_default()
        }
    }

    pub fn get_coord(&self) -> &PointCoordinate {
        &self.point_coord
    }

    pub fn get_color(&self) -> &PointColor {
        &self.point_color
    }

    pub fn get_nearest(&self, kd_tree: &KdTree<Point>) -> Point {
        kd_tree.nearest(self).unwrap().item.clone()
    }

    pub fn get_nearests(&self, kd_tree: &KdTree<Point>, quantity: usize) -> Points {
        Points::of(kd_tree.nearests(self, quantity).into_iter().map(|found| found.item.clone()).collect())
    }

    pub fn get_average(&self, another_point: &Point) -> Point {
        Point::new(self.clone().get_coord().get_average(another_point.get_coord()), 
                    self.clone().get_color().get_average(another_point.get_color()))
    }

    fn get_coord_delta(&self, another_point: &Point) -> f32 {
        self.clone().get_coord().get_coord_delta(&another_point.clone().get_coord())
    }

    fn get_color_delta(&self, another_point: &Point) -> f32 {
        self.clone().get_color().get_color_delta(&another_point.clone().get_color())
    }

    fn get_difference(&self, another_point: &Point, ratio: f32) -> f32 {
        self.get_coord_delta(another_point) * ratio +
        self.get_color_delta(another_point) * (1.0 - ratio)
    }

    fn get_closest(&self, points: Points, ratio: f32) -> Point {
        let mut min: f32 = f32::MAX;
        let mut result: Point = Point::new_default();

        for point in points.data {
            let cur = self.get_difference(&point, ratio);
            if cur < min {
                min = cur;
                result = point
            }
        };
        result
    }

    fn get_average_closest(&self, points: Points, ratio: f32) -> Point {
        self.get_average(&self.get_closest(points, ratio))
    }

    fn get_average_closest_from_kdtree(&self, kd_tree: &KdTree<Point>, ratio: f32) -> Point {
        self.get_average_closest(self.get_nearests(kd_tree, 400), ratio)
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