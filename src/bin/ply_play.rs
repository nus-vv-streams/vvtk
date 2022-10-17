use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::Fetcher;
use vivotk::render::wgpu::builder::RenderBuilder;
use vivotk::render::wgpu::camera::Camera;
use vivotk::render::wgpu::controls::Controller;
use vivotk::render::wgpu::metrics_reader::MetricsReader;
use vivotk::render::wgpu::reader::{
    BufRenderReader, PcdAsyncReader, RenderReader,
};
use vivotk::render::wgpu::renderer::Renderer;
use vivotk::utils::read_file_to_point_cloud;

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// Directory with all the pcd files in lexicographical order
    directory: String,
    #[clap(short = 'q', long, default_value_t = 0)]
    quality: u8,
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
    #[clap(long, value_enum, default_value_t = DecoderType::Noop)]
    decoder_type: DecoderType,
    #[clap(long)]
    decoder_path: Option<OsString>,
}

#[derive(clap::ValueEnum, Clone)]
enum DecoderType {
    Noop,
    Draco,
}

fn main() {
    let args: Args = Args::parse();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (req_tx, req_rx) = std::sync::mpsc::channel();
    let (resp_tx, resp_rx) = std::sync::mpsc::channel();
    let reader = PcdAsyncReader::new(resp_rx, req_tx);

    // We run a tokio runtime on a separate thread
    std::thread::spawn(move || {
        rt.block_on(async {
            if args.directory.starts_with("http") {
                let tmpdir = tempdir().expect("created temp dir to store files");
                let path = tmpdir.path();
                println!("Downloading files to {}", path.to_str().unwrap());
                let fetcher = Fetcher::new(&args.directory, path).await;

                loop {
                    let req = req_rx.recv().unwrap();
                    // TODO: handle errors gracefully.
                    let p = fetcher.download(req.object_id, req.quality, req.frame_offset).await.unwrap();
                    // TODO: move this to non-blocking
                    let a = match &args.decoder_type {
                        DecoderType::Draco => {
                            DracoDecoder::new(args.decoder_path.as_ref().unwrap().as_os_str())
                                .decode(p.as_os_str())
                        }
                        _ => NoopDecoder::new().decode(&p.as_os_str()),
                    };
                    let resp_tx = resp_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let pcd = read_file_to_point_cloud(&a.get(0).unwrap()).unwrap();
                        resp_tx.send(pcd).unwrap();
                    }).await.unwrap();
                }

                _ = tmpdir.close();
            } else {
                let path = Path::new(&args.directory);
                let mut ply_files: Vec<PathBuf> = vec![];
                println!("1. Finished downloading to / reading from {:?}", path);

                let mut dir = tokio::fs::read_dir(path).await.unwrap();
                while let Some(entry) = dir.next_entry().await.unwrap() {
                    let f = entry.path();
                    // TODO: change to is_ply_file function
                    if !f.extension().map(|f| "ply".eq(f)).unwrap_or(false) {
                        continue;
                    }
                    ply_files.push(f);
                }
                ply_files.sort();
                
                loop {
                    let req = req_rx.recv().unwrap();
                    println!("still here! request: {:?}", req);
                    let pcd = read_file_to_point_cloud(&ply_files.get(req.frame_offset as usize).unwrap()).unwrap();
                    resp_tx.send(pcd).unwrap();
                }
            };

        });
    });

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
    let render = 
    // if args.buffer_size > 1 {
    //     builder.add_window(Renderer::new(
    //         BufRenderReader::new(args.buffer_size, reader),
    //         args.fps,
    //         camera,
    //         (args.width, args.height),
    //         metrics,
    //     ))
    // } else {
        builder.add_window(Renderer::new(
            reader,
            args.fps,
            camera,
            (args.width, args.height),
            metrics,
        ));
    // };
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
