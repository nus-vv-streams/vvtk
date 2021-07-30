use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::window::Window;
use nalgebra::Point3;

use crate::points::Points;
use std::path::Path;

const DEFAULT_EYE: Point3<f32> = Point3::new(0.0f32, 500.0, 2500.0);
const DEFAULT_AT: Point3<f32> = Point3::new(300.0f32, 800.0, 200.0);

pub struct Renderer {
    first_person: ArcBall,
    pub(crate) window: Window,
}

impl Renderer {
    pub fn new() -> Self {
        let mut window = Window::new("In Summer We Render");
        window.set_light(Light::StickToCamera);
        window.set_point_size(1.0); // <-- change here

        Renderer {
            first_person: ArcBall::new_with_frustrum(
                std::f32::consts::PI / 4.0,
                0.1,
                10000.0,
                DEFAULT_EYE,
                DEFAULT_AT,
            ),
            window,
        }
    }

    pub fn set_point_size(&mut self, point_size: f32) {
        self.window.set_point_size(point_size)
    }

    pub fn render(&mut self) -> bool {
        self.window.render_with_camera(&mut self.first_person)
    }

    pub fn config_camera(&mut self, eye: Option<Point3<f32>>, at: Option<Point3<f32>>) {
        self.first_person = ArcBall::new_with_frustrum(
            std::f32::consts::PI / 4.0,
            0.1,
            4000.0,
            eye.unwrap_or(DEFAULT_EYE),
            at.unwrap_or(DEFAULT_AT),
        );
    }

    // pub fn render_with_camera(&mut self, eye: Point3<f32>, at: Point3<f32>) -> bool {
    //     self.first_person =
    //         ArcBall::new_with_frustrum(std::f32::consts::PI / 4.0, 0.1, 4000.0, eye, at);

    //     self.window.render_with_camera(&mut self.first_person)
    // }

    pub fn render_frame(&mut self, data: &Points) {
        for point in &data.data {
            self.window.draw_point_with_size(
                &point.get_coord().get_point3(),
                &point.get_color().get_point3(),
                point.point_size,
            );
        }
    }

    pub fn render_image(&mut self, data: &Points) {
        while self.render() {
            self.render_frame(data);
        }
    }

    pub(crate) fn screenshoot_to_path(&mut self, path: &str) {
        let img = self.window.snap_image();
        let img_path = Path::new(path);
        img.save(img_path).unwrap();
        println!("Screeshot saved to {}", path);
    }
}
