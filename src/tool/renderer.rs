use nalgebra::Point3;
use kiss3d::camera::{ArcBall};
use kiss3d::light::Light;
use kiss3d::window::Window;

use crate::points::{ Points };

use std::time::{Duration, Instant};
use std::path::Path;

pub struct Renderer {
    first_person: ArcBall,
    window: Window
}

impl Renderer {
    pub fn new() -> Self {
        let eye = Point3::new(0.0f32, 500.0, 2500.0);
        let at = Point3::new(300.0f32, 800.0, 200.0);
        let mut window = Window::new("In Summer We Render");
        window.set_light(Light::StickToCamera);
        window.set_point_size(10.0);
        
        Renderer {
            first_person: ArcBall::new_with_frustrum(std::f32::consts::PI / 4.0, 0.1, 4000.0, eye, at),
            window: window,
        }
    }

    pub fn rendering(&mut self) -> bool {
        self.window.render_with_camera(&mut self.first_person)
    }

    pub fn render_frame(&mut self, data: &Points){
        for point in &data.data {
            self.window.draw_point(&point.clone().get_coord().get_point3(), &point.clone().get_color().get_point3());
        }
    }

    pub fn render_image(&mut self, data: &Points) {
        while self.rendering() {
            self.render_frame(data);
        }
    }

    pub fn screenshoot(&mut self) {
        self.screenshoot_to_path("screenshot.png");
    }

    pub fn screenshoot_to_path(&mut self, path: &str) {
        let img = self.window.snap_image();
        let img_path = Path::new(path);
        img.save(img_path).unwrap();
        println!("Screeshot saved to {}", path);
    }
}
