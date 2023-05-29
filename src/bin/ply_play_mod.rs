use clap::Parser;
use std::ffi::OsString;
use std::path::Path;
use vivotk::render::wgpu::builder::RenderBuilder;
use vivotk::render::wgpu::camera::Camera;
use vivotk::render::wgpu::controls::Controller;
use vivotk::render::wgpu::metrics_reader::MetricsReader;
use vivotk::render::wgpu::reader::{BufRenderReader, RenderReader, PointCloudFileReader};
use vivotk::render::wgpu::renderer::Renderer;
/// Plays a folder of pcd files in lexicographical order
#[derive(Parser, Debug)]
pub struct Args {
    /// Directory with all the pcd files in lexicographical order
    directory: String,
    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.3)]
    camera_z: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    camera_yaw: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(short, long, default_value_t = 1600)]
    width: u32,
    #[clap(long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls")]
    show_controls: bool,
    #[clap(short, long, default_value_t = 1)]
    buffer_size: usize,
    #[clap(short, long)]
    metrics: Option<OsString>,
    #[clap(long, default_value = "infer")]
    play_format: String,
}

fn infer_format(path: &Path) -> String {
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

pub fn play(args: Args) {
    // let args: Args = Args::parse();
    let path = Path::new(&args.directory);
    let play_format = if args.play_format.eq("infer") {
        println!("Inferring format...");
        infer_format(path)
    } else {
        args.play_format
    };
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
    let render = if args.buffer_size > 1 {
        builder.add_window(Renderer::new(
            BufRenderReader::new(args.buffer_size, reader),
            args.fps,
            camera,
            (args.width, args.height),
            metrics,
        ))
    } else {
        builder.add_window(Renderer::new(
            reader,
            args.fps,
            camera,
            (args.width, args.height),
            metrics,
        ))
    };
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
    builder.run();
}
