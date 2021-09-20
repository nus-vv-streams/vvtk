use crate::errors::*;

use image::{imageops::flip_vertical, ImageBuffer, Rgb};
use kiss3d::camera::ArcBall;
use kiss3d::light::Light;
use kiss3d::point_renderer::PointRenderer;
use kiss3d::window::Window;
use nalgebra::Point3;

use crate::reader::read;
use std::sync::mpsc::channel;
use std::sync::Arc;

use kiss3d::camera::Camera;

use crate::ply::Ply;
use crate::ply_dir::PlyDir;
use crate::points::Points;
use std::path::Path;
use std::path::PathBuf;

use kiss3d::conrod;
use kiss3d::conrod::color::Color;
use kiss3d::conrod::position::Positionable;
use kiss3d::conrod::widget_ids;

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
        let mut window = Window::new_with_size(
            title.unwrap_or(DEFAULT_TITLE),
            width.unwrap_or(DEFAULT_WIDTH),
            height.unwrap_or(DEFAULT_HEIGHT),
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
    pub fn render_frame(&mut self, data: &Points, ids: &Ids, app: &mut DemoApp) {
        for point in &data.data {
            self.window.draw_point_with_size(
                &point.get_coord().get_point3(),
                &point.get_color().get_point3(),
                point.point_size,
            );
        }
        // self.update_and_print_info();
        // let mut ui = self.window.conrod_ui_mut().set_widgets();
        gui(ids, app, self);
    }

    /// Open the window and render the frame many times
    pub fn render_image(&mut self, data: &Points) {
        self.window.conrod_ui_mut().theme = theme();
        let ids = Ids::new(self.window.conrod_ui_mut().widget_id_generator());
        let mut app = DemoApp::new();
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

        let mut frame;
        self.window.conrod_ui_mut().theme = theme();
        let ids = Ids::new(self.window.conrod_ui_mut().widget_id_generator());
        let mut app = DemoApp::new();

        while self.render() {
            frame = rx.recv().unwrap();
            match frame {
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
            .unwrap_or(Path::new(DEFAULT_PNG_OUTPUT));

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

/// =======================================================================================

/*
 *
 * This is he example taken from conrods' repository.
 *
 */
/// A set of reasonable stylistic defaults that works for the `gui` below.
pub fn theme() -> conrod::Theme {
    use conrod::position::{Align, Direction, Padding, Position, Relative};
    conrod::Theme {
        name: "Demo Theme".to_string(),
        padding: Padding::none(),
        x_position: Position::Relative(Relative::Align(Align::Start), None),
        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color: conrod::color::DARK_CHARCOAL,
        shape_color: conrod::color::LIGHT_CHARCOAL,
        border_color: conrod::color::BLACK,
        border_width: 0.0,
        label_color: conrod::color::WHITE,
        font_id: None,
        font_size_large: 26,
        font_size_medium: 18,
        font_size_small: 12,
        widget_styling: conrod::theme::StyleMap::default(),
        mouse_drag_threshold: 0.0,
        double_click_threshold: std::time::Duration::from_millis(500),
    }
}

// Generate a unique `WidgetId` for each widget.
widget_ids! {
    pub struct Ids {
        canvas,
        toggle,
        text_edit,
    }
}

pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 420;

/// A demonstration of some application state we want to control with a conrod GUI.
pub struct DemoApp {
    information_button_color: conrod::Color,
    canvas_h: conrod::Scalar,
    text_edit: String,
}

impl DemoApp {
    /// Simple constructor for the `DemoApp`.
    pub fn new() -> Self {
        DemoApp {
            information_button_color: conrod::color::BLACK,
            canvas_h: 70.0,
            text_edit: "".to_string(),
        }
    }
}

/// Instantiate a GUI demonstrating every widget available in conrod.
pub fn gui(ids: &Ids, app: &mut DemoApp, renderer: &mut Renderer) {
    use conrod::{widget, Colorable, Labelable, Sizeable, Widget};

    let (eye_pos, at_pos) = renderer.get_eye_at_info();
    app.text_edit = format!("The eye's position is {}\nlooking at {}", eye_pos, at_pos);

    let ui = &mut renderer.window.conrod_ui_mut().set_widgets();

    const MARGIN: conrod::Scalar = 10.0;
    const INFO_SIZE: conrod::Scalar = 40.0;
    const TITLE_SIZE: conrod::FontSize = 42;

    widget::Canvas::new()
        .pad(MARGIN)
        .align_right()
        .align_top()
        .w(300.0)
        .h(app.canvas_h)
        .scroll_kids_vertically()
        .set(ids.canvas, ui);

    let is_white = app.information_button_color == conrod::color::WHITE;
    let label = if is_white { "info" } else { "info" };
    for is_white in widget::Toggle::new(is_white)
        .label(label)
        .label_color(if is_white {
            conrod::color::WHITE
        } else {
            conrod::color::LIGHT_CHARCOAL
        })
        .top_right_with_margin_on(ids.canvas, 0.0)
        .w_h(INFO_SIZE, INFO_SIZE)
        .set(ids.toggle, ui)
    {
        app.information_button_color = if is_white {
            conrod::color::WHITE
        } else {
            conrod::color::BLACK
        };

        app.canvas_h = if is_white {
            300.0
        } else {
            70.0
        };
    }

    for string in widget::TextEdit::new(&app.text_edit)
        .down_from(ids.toggle, 60.0)
        .align_middle_x_of(ids.canvas)
        .padded_w_of(ids.canvas, MARGIN)
        .h(100.0)
        .set(ids.text_edit, ui)
    {
        app.text_edit = string;
    }
}
