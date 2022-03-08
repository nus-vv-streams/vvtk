use std::ffi::OsString;
use std::path::Path;
use clap::Parser;
use vivotk::render::wgpu::builder::RenderBuilder;
use vivotk::render::wgpu::camera::Camera;
use vivotk::render::wgpu::controls::Controller;
use vivotk::render::wgpu::reader::{PcdFileReader, RenderReader};
use vivotk::render::wgpu::renderer::Renderer;

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// Directory with all the pcd files in lexicographical order
    directory: OsString,
    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 0.0)]
    camera_z: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    camera_yaw: f32,
    #[clap(long = "pitch", default_value_t = -20.0)]
    camera_pitch: f32,
    #[clap(short, long, default_value_t = 1600)]
    width: u32,
    #[clap(short, long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls")]
    show_controls: bool
}

fn main() {
    let args: Args = Args::parse();
    let path = Path::new(&args.directory);
    let reader = PcdFileReader::from_directory(path);

    if reader.len() == 0 {
        eprintln!("Must provide at least one file!");
        return;
    }

    let camera = Camera::new((args.camera_x, args.camera_y, args.camera_z), cgmath::Deg(args.camera_yaw), cgmath::Deg(args.camera_pitch));
    let mut builder = RenderBuilder::new();
    let slider_end = reader.len() - 1;
    let render = builder.add_window(Renderer::new(reader, args.fps, camera, (args.width, args.height)));
    if args.show_controls {
        let controls = builder.add_window(Controller { slider_end });
        controls.borrow_mut().add_listener(render.borrow().id());
        render.borrow_mut().add_listener(controls.borrow().id());
    }
    builder.run();

}