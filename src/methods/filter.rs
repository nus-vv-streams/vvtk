use crate::points::{Point, Points};

pub fn sep_by_y_coord(points: &Points) -> impl Fn(&Point) -> bool {
    let len = points.len() as f32;
    let sum: f32 = points
        .get_clone_data()
        .into_iter()
        .map(|point| point.point_coord.y)
        .sum();
    let mean = sum / len;

    move |point: &Point| (point.point_coord.y > mean)
}
