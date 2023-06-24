use clap::Parser;
use std::ffi::OsString;
use std::path::Path;

use vivotk::render::wgpu::{
    builder::RenderBuilder,
    camera::Camera,
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{PointCloudFileReader, RenderReader},
    renderer::Renderer,
};

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
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum DecoderType {
    Noop,
    Draco,
}

fn infer_format(src: &String) -> String {
    let choices = ["pcd", "ply", "bin", "http"];
    const PCD: usize = 0;
    const PLY: usize = 1;
    const BIN: usize = 2;

    if choices.contains(&src.as_str()) {
        return src.clone();
    }

    let path = Path::new(src);
    // infer by counting extension numbers (pcd ply and bin)

    let mut choice_count = [0, 0, 0];
    for file_entry in path.read_dir().unwrap() {
        match file_entry {
            Ok(entry) => {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq("pcd") {
                        choice_count[PCD] += 1;
                    } else if ext.eq("ply") {
                        choice_count[PLY] += 1;
                    } else if ext.eq("bin") {
                        choice_count[BIN] += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}")
            }
        }
    }

    let max_index = choice_count
        .iter()
        .enumerate()
        .max_by_key(|(_, &item)| item)
        .map(|(index, _)| index);
    choices[max_index.unwrap()].to_string()
}

fn main() {
    let args: Args = Args::parse();
    let play_format = infer_format(&args.src);
    let path = Path::new(&args.src);

    // println!("Playing files in {:?} with format {}", path, play_format);

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
