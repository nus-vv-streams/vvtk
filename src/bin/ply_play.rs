use clap::Parser;
use std::ffi::OsString;
use std::path::Path;
use tempfile::tempdir;
use vivotk::codec::noop::NoopDecoder;
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::Fetcher;
use vivotk::render::wgpu::builder::RenderBuilder;
use vivotk::render::wgpu::camera::Camera;
use vivotk::render::wgpu::controls::Controller;
use vivotk::render::wgpu::metrics_reader::MetricsReader;
use vivotk::render::wgpu::reader::{BufRenderReader, PcdFileReader, RenderReader};
use vivotk::render::wgpu::renderer::Renderer;

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
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
    #[clap(short, long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls")]
    show_controls: bool,
    #[clap(short, long, default_value_t = 1)]
    buffer_size: usize,
    #[clap(short, long)]
    metrics: Option<OsString>,
}

fn main() {
    let args: Args = Args::parse();
    let tmpdir = tempdir().expect("created temp dir to store files");
    let path = if args.directory.starts_with("http") {
        println!("Files downloaded to {}", tmpdir.path().to_str().unwrap());

        let fetcher = Fetcher::new(&args.directory);
        fetcher
            .download_to(&tmpdir.path().to_path_buf())
            .expect("failed to download files");

        tmpdir.path()
    } else {
        Path::new(&args.directory)
    };

    println!("PATH IS {:?}", path);
    NoopDecoder::new()
        .decode_folder(path)
        .expect("decoding failed");
    let reader = PcdFileReader::from_directory(path);
    println!("There are {:} pcd files", reader.len());

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
    _ = tmpdir.close();
}
