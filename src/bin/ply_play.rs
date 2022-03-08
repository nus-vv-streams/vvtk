use std::ffi::OsString;
use std::path::Path;
use clap::Parser;
use vivotk::render::wgpu::camera::Camera;
use vivotk::render::wgpu::reader::PcdFileReader;
use vivotk::render::wgpu::renderer::RenderBuilder;

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
    let camera = Camera::new((args.camera_x, args.camera_y, args.camera_z), cgmath::Deg(args.camera_yaw), cgmath::Deg(args.camera_pitch));
    let builder = RenderBuilder::new(reader, args.fps, camera, (args.width, args.height));
    pollster::block_on(builder.play(args.show_controls));

}