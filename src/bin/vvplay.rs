use clap::Parser;
use std::ffi::OsString;
use std::path::Path;

use vivotk::render::wgpu::{
    builder::RenderBuilder, camera::Camera, controls::Controller, metrics_reader::MetricsReader,
    render_manager::AdaptiveUpsamplingManager, renderer::Renderer,
};

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// src can be:
    /// 1. Directory with all the pcd files in lexicographical order
    /// 2. location of the mpd file
    src: String,

    #[clap(short, long, default_value_t = 0)]
    quality: u8,

    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,

    #[clap(
        short = 'x',
        long,
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    camera_x: f32,

    #[clap(
        short = 'y',
        long,
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    camera_y: f32,

    #[clap(
        short = 'z',
        long,
        default_value_t = 1.3,
        allow_negative_numbers = true
    )]
    camera_z: f32,

    #[clap(long = "yaw", default_value_t = -90.0, allow_negative_numbers = true)]
    camera_yaw: f32,

    #[clap(long = "pitch", default_value_t = 0.0, allow_negative_numbers = true)]
    camera_pitch: f32,

    #[clap(short = 'W', long, default_value_t = 1600)]
    width: u32,

    #[clap(short = 'H', long, default_value_t = 900)]
    height: u32,

    #[clap(long = "controls", default_value_t = true)]
    show_controls: bool,

    #[clap(short, long)]
    buffer_size: Option<u8>,

    #[clap(short, long)]
    metrics: Option<OsString>,

    #[clap(long = "decoder", value_enum, default_value_t = DecoderType::Noop)]
    decoder_type: DecoderType,

    #[clap(long)]
    decoder_path: Option<OsString>,

    #[clap(long, default_value = "rgb(255,255,255)")]
    bg_color: OsString,

    #[clap(long, default_value = "false")]
    adaptive_upsampling: bool,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum DecoderType {
    Noop,
    Draco,
}

fn main() {
    let args: Args = Args::parse();
    let adaptive_manager = AdaptiveUpsamplingManager::new(&args.src, args.adaptive_upsampling);

    let camera = Camera::new(
        (args.camera_x, args.camera_y, args.camera_z),
        cgmath::Deg(args.camera_yaw),
        cgmath::Deg(args.camera_pitch),
    );
    let metrics = args
        .metrics
        .map(|os_str| MetricsReader::from_directory(Path::new(&os_str)));
    let mut builder = RenderBuilder::default();
    let slider_end = adaptive_manager.len() - 1;
    let render = builder.add_window(Renderer::new(
        adaptive_manager,
        args.fps,
        camera,
        (args.width, args.height),
        metrics,
        args.bg_color.to_str().unwrap(),
    ));

    if args.show_controls {
        let controls = builder.add_window(Controller { slider_end });
        builder
            .get_windowed_mut(render)
            .unwrap()
            .add_output(controls);
        builder
            .get_windowed_mut(controls)
            .unwrap()
            .add_output(render);
    }

    // In MacOS, renderer must run in main thread.
    builder.run();
}
