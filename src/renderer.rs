extern crate nalgebra as na;
use na::Point3;

extern crate kiss3d;
use kiss3d::camera::{ArcBall};
use kiss3d::light::Light;
use kiss3d::window::Window;

use crate::color;
use crate::coordinate;
use crate::points::{ Points, Point};

pub struct Renderer {
    first_person: ArcBall,
    window: Window
}

impl Renderer {
    pub fn new() -> Self {
        let eye = Point3::new(0.0f32, 500.0, 2500.0);
        let at = Point3::new(300.0f32, 400.0, 200.0);

        Renderer {
            first_person: ArcBall::new_with_frustrum(std::f32::consts::PI / 4.0, 0.1, 4000.0, eye, at),
            window: Window::new("In Summer We Render"),
        }
    }

    pub fn render_image(&mut self, data: &Points) {
        while self.window.render_with_camera(&mut self.first_person) {
            for point in &data.data {
                self.window.draw_point(&point.point_coord.get(), &point.point_color.get());
            }
        }
    }

    pub fn rendering(&mut self) -> bool {
        self.window.render_with_camera(&mut self.first_person)
    }

    pub fn render_frame(&mut self, data: &Points){
        self.window.draw_point(&Point3::new(0.0, 0.0, 0.0), &Point3::new(1.0, 1.0, 1.0));
        self.window.draw_point(&Point3::new(1000.0, 0.0, 0.0), &Point3::new(1.0, 0.0, 0.0));
        self.window.draw_point(&Point3::new(0.0, 1000.0, 0.0), &Point3::new(0.0, 1.0, 0.0));
        self.window.draw_point(&Point3::new(0.0, 0.0, 1000.0), &Point3::new(0.0, 0.0, 1.0));

        for point in &data.data {
            self.window.draw_point(&point.point_coord.get(), &point.point_color.get());
        }
    }
}
