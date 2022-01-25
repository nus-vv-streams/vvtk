use crate::errors::*;

use image::{imageops::flip_vertical, ImageBuffer, Rgb};
use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::point_renderer::PointRenderer;
use kiss3d::window::CanvasSetup;
use kiss3d::window::NumSamples;
use kiss3d::window::Window;
use nalgebra::Point3;

use crate::reader::read;
use std::sync::mpsc::channel;
use std::sync::Arc;

use kiss3d::camera::Camera;

use crate::ply::Ply;
use crate::ply_dir::PlyDir;
use crate::points::PointCloud;
use std::path::Path;
use std::path::PathBuf;

use kiss3d::conrod::event::{Event, Input};
use kiss3d::conrod::input::{Button, Key};

use crate::gui;

const DEFAULT_EYE: Point3<f32> = Point3::new(0.0f32, 500.0, 1969.0);
const DEFAULT_AT: Point3<f32> = Point3::new(300.0f32, 500.0, 200.0);

/// The default width of the canvas
pub static DEFAULT_WIDTH: u32 = 800u32;

/// The default height of the canvas
pub static DEFAULT_HEIGHT: u32 = 600u32;

/// The default width of PNG file
pub static DEFAULT_WIDTH_PNG: u32 = 1600u32;

/// The default height of PNG file
pub static DEFAULT_HEIGHT_PNG: u32 = 1200u32;

/// The default corner's coordinate of the canvas
pub static DEFAULT_CORNER: u32 = 0u32;

/// The default name of the canvas
pub static DEFAULT_TITLE: &str = "In Summer We Render";

/// The default output of method save_to_png
pub static DEFAULT_PNG_OUTPUT: &str = "output.png";

pub static DEFAULT_EPSILON: f32 = 10.0f32;
// let mut current_eye_try: Point3<f32> = DEFAULT_EYE;

/// Structure representing a window and a camera.
pub struct Renderer {
    first_person: ArcBall,
    pub(crate) window: Window,
    current_eye: Point3<f32>,
    current_at: Point3<f32>,
}

impl Renderer {
    /// Create a new Renderer with specific name and default window's and camera's configuration.
    pub fn new(title: Option<&str>, width: Option<u32>, height: Option<u32>) -> Self {
        let mut window = Window::new_with_setup(
            title.unwrap_or(DEFAULT_TITLE),
            width.unwrap_or(DEFAULT_WIDTH),
            height.unwrap_or(DEFAULT_HEIGHT),
            CanvasSetup {
                vsync: true,
                samples: NumSamples::Four,
            },
        );

        window.set_light(Light::StickToCamera);
        window.set_point_size(1.0); // <-- change here

        Renderer {
            first_person: default_camera(),
            window,
            current_eye: DEFAULT_EYE,
            current_at: DEFAULT_AT,
        }
    }

    /// Render with default camera
    pub fn render(&mut self) -> bool {
        self.window.render_with_camera(&mut self.first_person)
    }

    /// Set the size of points
    pub fn set_point_size(&mut self, point_size: f32) {
        self.window.set_point_size(point_size)
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

    /// Config the background color
    pub fn config_background_color(&mut self, background_color: Option<Point3<f32>>) {
        let color = background_color.unwrap_or_else(Point3::origin);
        self.window
            .set_background_color(color.x / 256.0, color.y / 256.0, color.z / 256.0);
    }

    /// Set title of the window
    pub fn set_title(&mut self, new_title: Option<&str>) {
        self.window.set_title(new_title.unwrap_or(DEFAULT_TITLE))
    }

    /// Open the window and render the frame
    pub fn render_frame(&mut self, data: &PointCloud, ids: &gui::Ids, app: &mut gui::InfoBar) {
        for point in &data.data {
            self.window.draw_point_with_size(
                &point.get_coord().get_point3(),
                &point.get_color().get_point3(),
                point.point_size,
            );
        }
        // self.update_and_print_info();
        // let mut ui = self.window.conrod_ui_mut().set_widgets();
        gui::gui(ids, app, self);
    }

    /// Open the window and render the frame many times
    pub fn render_image(&mut self, data: &PointCloud) {
        self.window.conrod_ui_mut().theme = gui::theme();
        let ids = gui::Ids::new(self.window.conrod_ui_mut().widget_id_generator());
        let mut app = gui::InfoBar::new_closed_state();
        while self.render() {
            self.render_frame(data, &ids, &mut app);
        }
    }

    pub fn render_video(&mut self, ply_dir: PlyDir) -> Result<()> {
        let len = ply_dir.count();
        let paths = Arc::new(ply_dir.get_paths());

        let (tx, rx) = channel();
        let (paths_clone, tx) = (paths, tx);

        std::thread::spawn(move || {
            let mut index: usize = 0;
            loop {
                index += 1;
                let frame = read(paths_clone[index % len].to_str());
                tx.send(frame).unwrap();
            }
        });

        let mut frame = Ok(Ply::nothing());
        self.window.conrod_ui_mut().theme = gui::theme();
        let ids = gui::Ids::new(self.window.conrod_ui_mut().widget_id_generator());
        let mut app = gui::InfoBar::new_closed_state();

        let mut is_stop = false;

        while self.render() {
            for event in self.window.conrod_ui().global_input().events() {
                match *event {
                    Event::Raw(Input::Press(Button::Keyboard(Key::Space))) => {
                        if is_stop {
                            is_stop = false
                        } else {
                            is_stop = true
                        }
                    }
                    _ => {}
                }
            }

            if !is_stop {
                frame = rx.recv().unwrap();
            }

            match &frame {
                Ok(f) => {
                    self.render_frame(f.get_points_as_ref(), &ids, &mut app);
                }
                Err(e) => {
                    eprintln!("Problem with reading file:\n    {}", e);
                    continue;
                }
            }
        }

        Ok(())
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
        ply: &mut Ply,
        x: Option<u32>,
        y: Option<u32>,
        width: Option<u32>,
        height: Option<u32>,
        output: Option<&str>,
    ) -> Result<()> {
        use kiss3d::renderer::Renderer;

        let data = ply.get_points_as_ref();

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
            width.unwrap_or(DEFAULT_WIDTH_PNG) as usize,
            height.unwrap_or(DEFAULT_HEIGHT_PNG) as usize,
        );

        let img_opt = ImageBuffer::from_vec(
            width.unwrap_or(DEFAULT_WIDTH_PNG),
            height.unwrap_or(DEFAULT_HEIGHT_PNG),
            buf,
        );

        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            img_opt.chain_err(|| "Buffer created from window was not big enough for image")?;
        let img = flip_vertical(&img);

        let mut path_in_ply: Option<&Path> = None;
        let mut p: PathBuf;
        if ply.get_title().is_some() {
            p = PathBuf::from(ply.get_title().unwrap());
            p.set_extension("png");
            path_in_ply = Some(p.as_path());
        }

        let img_path = output
            .map(|p| Path::new(p))
            .or(path_in_ply)
            .unwrap_or_else(|| Path::new(DEFAULT_PNG_OUTPUT));

        img.save(img_path)
            .map(|_| println!("Image saved to {:?}", img_path))
            .chain_err(|| "Cannot save image")?;

        Ok(())
    }

    pub fn get_eye_at_info(&self) -> (Point3<f32>, Point3<f32>) {
        (self.first_person.eye(), self.first_person.at())
    }

    /// Print out curr
    pub fn update_and_print_info(&mut self) {
        let runtime_eye = self.first_person.eye();
        let runtime_at = self.first_person.at();

        if self.is_update(&runtime_eye, &runtime_at) {
            println!(
                "===================\n    Update detected!\n    The eye's position is {}\n    looking at {}",
                runtime_eye, runtime_at
            );
            self.update_eye_at(runtime_eye, runtime_at);
        }
    }

    fn is_update(&self, runtime_eye: &Point3<f32>, runtime_at: &Point3<f32>) -> bool {
        self.is_eye_update(runtime_eye) || self.is_at_update(runtime_at)
    }

    fn is_eye_update(&self, runtime_eye: &Point3<f32>) -> bool {
        !is_relative_eq(runtime_eye, &self.current_eye)
    }

    fn is_at_update(&self, runtime_at: &Point3<f32>) -> bool {
        !is_relative_eq(runtime_at, &self.current_at)
    }

    fn update_eye_at(&mut self, runtime_eye: Point3<f32>, runtime_at: Point3<f32>) {
        self.current_eye = runtime_eye;
        self.current_at = runtime_at;
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

fn is_relative_eq(point1: &Point3<f32>, point2: &Point3<f32>) -> bool {
    relative_eq!(point1.x, point2.x, epsilon = DEFAULT_EPSILON)
        && relative_eq!(point1.y, point2.y, epsilon = DEFAULT_EPSILON)
        && relative_eq!(point1.z, point2.z, epsilon = DEFAULT_EPSILON)
}
