use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::Fetcher;
use vivotk::formats::pointxyzrgba::PointXyzRgba;
use vivotk::formats::PointCloud;
use vivotk::render::wgpu::{
    builder::RenderBuilder,
    camera::Camera,
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{FrameRequest, PcdAsyncReader, RenderReader, PointCloudFileReader},
    renderer::Renderer,
};
use vivotk::utils::read_file_to_point_cloud;

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// src can be:
    /// 1. Directory with all the pcd files in lexicographical order
    /// 2. location of the mpd file
    src: String,
    #[clap(short = 'q', long, default_value_t = 0)]
    quality: u8,
    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0, allow_negative_numbers = true)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0, allow_negative_numbers = true)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.3, allow_negative_numbers = true)]
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
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum DecoderType {
    Noop,
    Draco,
}

fn infer_format(src: &String) -> String {
    let choices = ["pcd", "ply", "http"];
    if choices.contains(&src.as_str()) {
        return src.clone();
    }

    let path = Path::new(src);
    // infer by counting extension numbers (pcd count and ply count)
    // if pcd count > ply count, then pcd
    let mut pcd_count = 0;
    let mut ply_count = 0;
    for file_entry in path.read_dir().unwrap() {
        match file_entry {
            Ok(entry) => {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq("pcd") {
                        pcd_count += 1;
                    } else if ext.eq("ply") {
                        ply_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}")
            }
        }
    }
    if pcd_count > ply_count {
        "pcd".to_string()
    } else {
        "ply".to_string()
    }
}

fn main() {
    let args: Args = Args::parse();
    let play_format = infer_format(&args.src);
    let path = Path::new(&args.src);

    println!("Playing files in {:?} with format {}", path, play_format);

    let reader = PointCloudFileReader::from_directory(path, &play_format);

    if reader.len() == 0 {
        eprintln!("Must provide at least one file!");
        return;
    }

    let camera = Camera::new(
        (args.camera_x, args.camera_y, args.camera_z),
        cgmath::Deg(args.camera_yaw),
        cgmath::Deg(args.camera_pitch),
    );
    let metrics = args
        .metrics
        .map(|os_str| MetricsReader::from_directory(Path::new(&os_str)));
    let mut builder = RenderBuilder::default();
    let slider_end = reader.len() - 1;
    let render = builder.add_window(Renderer::new(
            reader,
            args.fps,
            camera,
            (args.width, args.height),
            metrics,
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
