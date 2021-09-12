// use nalgebra::Point3;

use crate::point::Point;
use crate::points::Points;

use crate::color::PointColor;
use std::collections::HashMap;

/// The function object that transform one point to another
pub type TransformFn = Box<dyn Fn(&Point) -> Point>;

/// The function object that produce the `TransformFn`
pub type TransformProducer = Box<dyn Fn(&Points) -> TransformFn>;

/// The default key of the hashmap of `TransformFn`
/// Return a key `do_nothing`
pub const DEFAULT_KEY: &str = "default";

/// The `TransformProducer` producing the `TransformFn` that doesn't change the point
pub fn do_nothing() -> TransformProducer {
    Box::new(move |_points: &Points| Box::new(move |point: &Point| point.clone()))
}

/// The `TransformProducer` producing the `TransformFn` that make the point green
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

/// The `TransformProducer` producing the `TransformFn` that make the point red
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

/// The `TransformProducer` producing the `TransformFn` that make the point larger (point's size = 2)
pub fn point_size_2() -> TransformProducer {
    Box::new(move |_points: &Points| {
        Box::new(move |point: &Point| {
            let mut res = point.clone();
            res.point_size = 2.0;
            res
        })
    })
}

/// Return the Hashmap of all `TransformProducer`
pub fn get_collection() -> HashMap<String, TransformProducer> {
    let mut transform_methods = HashMap::new();
    transform_methods.insert(DEFAULT_KEY.to_string(), do_nothing());
    transform_methods.insert("all_green".to_string(), all_green());
    transform_methods.insert("all_red".to_string(), all_red());
    transform_methods.insert("point_size_2".to_string(), point_size_2());
    transform_methods
}
