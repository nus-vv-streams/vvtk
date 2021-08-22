use crate::errors::*;

use image::{imageops::flip_vertical, ImageBuffer, Rgb};
use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::point_renderer::PointRenderer;
use kiss3d::window::Window;
use nalgebra::Point3;

use crate::points::Points;
use std::path::Path;

const DEFAULT_EYE: Point3<f32> = Point3::new(0.0f32, 500.0, 2500.0);
const DEFAULT_AT: Point3<f32> = Point3::new(300.0f32, 800.0, 200.0);

/// The default width of the canvas
pub static DEFAULT_WIDTH: u32 = 1600u32;

/// The default height of the canvas
pub static DEFAULT_HEIGHT: u32 = 1200u32;

/// The default corner's coordinate of the canvas
pub static DEFAULT_CORNER: u32 = 0u32;

/// The default name of the canvas
pub static DEFAULT_TITLE: &str = "In Summer We Render";

/// Structure representing a window and a camera.
pub struct Renderer {
    first_person: ArcBall,
    pub(crate) window: Window,
}

impl Renderer {
    /// Create a new Renderer with specific name and default window's and camera's configuration.
    pub fn new(title: Option<&str>) -> Self {
        let mut window = Window::new(title.unwrap_or(DEFAULT_TITLE));

        window.set_light(Light::StickToCamera);
        window.set_point_size(1.0); // <-- change here

        Renderer {
            first_person: default_camera(),
            window,
        }
    }

    /// Set the size of points
    pub fn set_point_size(&mut self, point_size: f32) {
        self.window.set_point_size(point_size)
    }

    /// Render with default camera
    pub fn render(&mut self) -> bool {
        self.window.render_with_camera(&mut self.first_person)
    }

    /// Config the camera
    ///
    /// # Arguments
    /// * `eye` - the coordinate of the "eye"
    /// * `at` - the coordinate of where the "eye" look at
    pub fn config_camera(&mut self, eye: Option<Point3<f32>>, at: Option<Point3<f32>>) {
        self.first_person = ArcBall::new_with_frustrum(
            std::f32::consts::PI / 4.0,
            0.1,
            4000.0,
            eye.unwrap_or(DEFAULT_EYE),
            at.unwrap_or(DEFAULT_AT),
        );
    }

    pub fn config_background_color(&mut self, background_color: Option<Point3<f32>>) {
        let color = background_color.unwrap_or(Point3::origin());
        self.window.set_background_color(color.x, color.y, color.z);
    }

    /// Open the window and render the frame
    pub fn render_frame(&mut self, data: &Points) {
        for point in &data.data {
            self.window.draw_point_with_size(
                &point.get_coord().get_point3(),
                &point.get_color().get_point3(),
                point.point_size,
            );
        }
    }

    /// Open the window and render the frame many times
    pub fn render_image(&mut self, data: &Points) {
        while self.render() {
            self.render_frame(data);
        }
    }

    /// Render a ply file to png format
    ///
    /// # Arguments
    /// * `x` - the x-coordinate of the bottom left corner
    /// * `y` - the y-coordinate of the bottom left corner
    /// * `width` - the width of the png
    /// * `height` - the height of the png
    /// * `path` - the path to save the png file
    pub fn save_to_png(
        &mut self,
        data: &Points,
        x: Option<u32>,
        y: Option<u32>,
        width: Option<u32>,
        height: Option<u32>,
        path: Option<&str>,
    ) -> Result<()> {
        use kiss3d::renderer::Renderer;

        let mut pr = PointRenderer::new();
        for point in &data.data {
            pr.draw_point_with_size(
                point.get_coord().get_point3(),
                point.get_color().get_point3(),
                point.point_size,
            );
        }

        pr.render(0, &mut self.first_person);

        let mut buf = Vec::new();

        self.window.snap_rect(
            &mut buf,
            x.unwrap_or(DEFAULT_CORNER) as usize,
            y.unwrap_or(DEFAULT_CORNER) as usize,
            width.unwrap_or(DEFAULT_WIDTH) as usize,
            height.unwrap_or(DEFAULT_HEIGHT) as usize,
        );

        let img_opt = ImageBuffer::from_vec(
            width.unwrap_or(DEFAULT_WIDTH),
            height.unwrap_or(DEFAULT_HEIGHT),
            buf,
        );
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            img_opt.chain_err(|| "Buffer created from window was not big enough for image")?;
        let img = flip_vertical(&img);

        let img_path = Path::new(path.chain_err(|| "No output found")?);
        img.save(img_path)
            .map(|_| println!("Image saved to {}", path.unwrap()))
            .chain_err(|| "Cannot save image")?;

        Ok(())
    }
}

fn default_camera() -> ArcBall {
    ArcBall::new_with_frustrum(
        std::f32::consts::PI / 4.0,
        0.1,
        10000.0,
        DEFAULT_EYE,
        DEFAULT_AT,
    )
}
