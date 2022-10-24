use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::Fetcher;
use vivotk::render::wgpu::{
    builder::RenderBuilder,
    camera::Camera,
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{PcdAsyncReader, RenderReader},
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

#[derive(clap::ValueEnum, Clone, Copy)]
enum DecoderType {
    Noop,
    Draco,
}

fn main() {
    let args: Args = Args::parse();
    let rt = tokio::runtime::Runtime::new().unwrap();
    // important to use tokio::mpsc here instead of std because it is bridging from sync -> async
    let (req_tx, mut req_rx) = tokio::sync::mpsc::unbounded_channel();
    let (resp_tx, resp_rx) = std::sync::mpsc::channel();
    let (total_frames_tx, total_frames_rx) = tokio::sync::oneshot::channel();
    let mut reader = PcdAsyncReader::new(resp_rx, req_tx);

    // copy variables to be moved into the async block
    let src = args.src.clone();
    let decoder_type = args.decoder_type;
    let decoder_path = args.decoder_path.clone();

    // We run a tokio runtime on a separate thread
    std::thread::spawn(move || {
        rt.block_on(async {
            if src.starts_with("http") {
                let tmpdir = tempdir().expect("created temp dir to store files");
                let path = tmpdir.path();
                println!("Downloading files to {}", path.to_str().unwrap());

                let fetcher = Fetcher::new(&src, path).await;
                total_frames_tx
                    .send(fetcher.get_total_frames())
                    .expect("sent total frames");
                loop {
                    let req = req_rx.recv().await.unwrap();
                    println!("got frame requests {:?}", req);

                    let fetcher = fetcher.clone();
                    let decoder_path = decoder_path.clone();
                    let resp_tx = resp_tx.clone();
                    _ = tokio::spawn(async move {
                        let p = fetcher
                            .download(req.object_id, req.quality, req.frame_offset)
                            .await;
                        let p = match p {
                            Ok(p) => p,
                            Err(e) => {
                                println!("Error downloading file: {}", e);
                                return;
                            }
                        };

                        println!("Downloaded {} successfully", p.to_str().unwrap());
                        // Run decoder if needed
                        let decoded_files =
                            tokio::task::spawn_blocking(move || match decoder_type {
                                DecoderType::Draco => {
                                    DracoDecoder::new(decoder_path.as_ref().unwrap().as_os_str())
                                        .decode(p.as_os_str())
                                }
                                _ => NoopDecoder::new().decode(&p.as_os_str()),
                            })
                            .await
                            .unwrap();
                        println!("Decoded {:?} successfully", decoded_files);

                        for f in decoded_files {
                            let pcd = read_file_to_point_cloud(&f).unwrap();
                            println!(
                                "reading decoded file {} and sending pcd...",
                                f.to_str().unwrap()
                            );
                            resp_tx.send(pcd).unwrap();
                        }
                    })
                    .await;
                }

                // use tmpdir here so it is not dropped before
                _ = tmpdir.close();
            } else {
                let path = Path::new(&args.src);
                let mut ply_files: Vec<PathBuf> = vec![];
                println!("1. Finished downloading to / reading from {:?}", path);

                let mut dir = tokio::fs::read_dir(path).await.unwrap();
                while let Some(entry) = dir.next_entry().await.unwrap() {
                    let f = entry.path();
                    if !f.extension().map(|f| "ply".eq(f)).unwrap_or(false) {
                        continue;
                    }
                    ply_files.push(f);
                }
                total_frames_tx
                    .send(ply_files.len())
                    .expect("sent total frames");
                ply_files.sort();

                loop {
                    let req = req_rx.recv().await.unwrap();
                    let pcd = read_file_to_point_cloud(
                        &ply_files.get(req.frame_offset as usize).unwrap(),
                    )
                    .unwrap();
                    resp_tx.send(pcd).unwrap();
                }
            };
        });
    });

    // set the reader max length
    reader.set_len(total_frames_rx.blocking_recv().unwrap());

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
