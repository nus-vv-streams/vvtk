use nalgebra::Point3;

use crate::points::Point;
use crate::tool::renderer::Renderer;

pub fn all_green(renderer: &mut Renderer, point: &Point) {
    renderer
        .window
        .draw_point(&point.get_coord().get_point3(), &Point3::new(0.0, 1.0, 0.0))
}

pub fn all_red(renderer: &mut Renderer, point: &Point) {
    renderer
        .window
        .draw_point(&point.get_coord().get_point3(), &Point3::new(1.0, 0.0, 0.0))
}

pub fn pt_size_2(renderer: &mut Renderer, point: &Point) {
    renderer.window.draw_point_with_size(
        &point.get_coord().get_point3(),
        &point.get_color().get_point3(),
        2.0,
    )
}
