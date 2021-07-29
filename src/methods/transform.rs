// use nalgebra::Point3;

use crate::points::{Point, Points};

use crate::color::PointColor;
use std::collections::HashMap;

pub type TransformFn = Box<dyn Fn(&Point) -> Point>;
pub type TransformProducer = Box<dyn Fn(&Points) -> TransformFn>;
pub const DEFAULT_KEY: &str = "default";

pub fn do_nothing() -> TransformProducer {
    Box::new(move |_points: &Points| Box::new(move |point: &Point| point.clone()))
}

pub fn all_green() -> TransformProducer {
    Box::new(move |_points: &Points| {
        Box::new(move |point: &Point| {
            let mut res = point.clone();
            res.point_color = PointColor::new_default();
            res.point_color.green = 255;

            res
        })
    })
}

pub fn all_red() -> TransformProducer {
    Box::new(move |_points: &Points| {
        Box::new(move |point: &Point| {
            let mut res = point.clone();
            res.point_color = PointColor::new_default();
            res.point_color.red = 255;

            res
        })
    })
}

pub fn point_size_2() -> TransformProducer {
    Box::new(move |_points: &Points| {
        Box::new(move |point: &Point| {
            let mut res = point.clone();
            res.point_size = 2.0;
            res
        })
    })
}

pub fn get_collection() -> HashMap<String, TransformProducer> {
    let mut transform_methods = HashMap::new();
    transform_methods.insert(DEFAULT_KEY.to_string(), do_nothing());
    transform_methods.insert("all_green".to_string(), all_green());
    transform_methods.insert("all_red".to_string(), all_red());
    transform_methods.insert("point_size_2".to_string(), point_size_2());
    transform_methods
}
