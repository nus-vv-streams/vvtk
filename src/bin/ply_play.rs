use clap::Parser;
use log::{debug, warn};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::codec::decoder::{DracoDecoder, MultiplaneDecodeReq, MultiplaneDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::{
    buffer::Buffer,
    fetcher::{FetchResult, Fetcher},
};
use vivotk::render::wgpu::{
    builder::RenderBuilder,
    camera::Camera,
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{FrameRequest, PcdAsyncReader, RenderReader},
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
    #[clap(long = "controls", default_value_t = true)]
    show_controls: bool,
    #[clap(short, long)]
    buffer_size: Option<u8>,
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
    Multiplane,
}

fn main() {
    // initialize logger
    env_logger::init();
    let args: Args = Args::parse();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    // important to use tokio::mpsc here instead of std because it is bridging from sync -> async
    // the content is produced by the renderer and consumed by the fetcher
    let (frame_req_tx, mut frame_req_rx) = tokio::sync::mpsc::unbounded_channel();
    // this buffer is used to store the fetched data.
    // the content is produced by the fetcher and consumed by the decoder thread.
    let (buffer, mut decoder_rx) = Buffer::new(args.buffer_size.unwrap_or(10) as usize);
    // the content is produced by the decode or the local file reader and consumed by the renderer
    let (pc_tx, pc_rx) = std::sync::mpsc::channel();
    // the total frame number we are expecting. This is for display purposes in the renderer only.
    let (total_frames_tx, total_frames_rx) = tokio::sync::oneshot::channel();

    // copy variables to be moved into the async block
    let src = args.src.clone();
    let decoder_type = args.decoder_type;
    let decoder_path = args.decoder_path.clone();
    let pc_tx2 = pc_tx.clone();

    // We run the fetcher as a separate tokio task. Although it is an infinite loop, it has a lot of await breakpoints.
    // Fetcher will fetch data and send it over to the buffer.
    rt.spawn(async move {
        if src.starts_with("http") {
            let tmpdir = tempdir().expect("created temp dir to store files");
            let path = tmpdir.path();
            debug!("Downloading files to {}", path.to_str().unwrap());

            let mut fetcher = Fetcher::new(&src, path).await;
            total_frames_tx
                .send(fetcher.total_frames())
                .expect("sent total frames");

            let mut frame_range = (0, 0);

            loop {
                let req: FrameRequest = frame_req_rx.recv().await.unwrap();
                debug!("got frame requests {:?}", req);

                if frame_range.0 < req.frame_offset && req.frame_offset < frame_range.1 {
                    continue;
                }

                // we should probably do *bounded* retry here
                loop {
                    let p = fetcher.download(req.object_id, req.frame_offset).await;

                    match p {
                        Ok(res) => {
                            _ = buffer.push((req.clone(), res)).await;
                            frame_range.0 = req.frame_offset;
                            frame_range.1 = req.frame_offset + fetcher.segment_size();
                            break;
                        }
                        Err(e) => {
                            warn!("Error downloading file: {}", e)
                        }
                    }
                }
            }

            // use tmpdir here so it is not dropped before
            _ = tmpdir.close();
        } else {
            let path = Path::new(&args.src);
            let mut ply_files: Vec<PathBuf> = vec![];
            debug!("1. Finished downloading to / reading from {:?}", path);

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
                let req: FrameRequest = frame_req_rx.recv().await.unwrap();
                debug!("got frame requests {:?}", req);
                let pcd =
                    read_file_to_point_cloud(ply_files.get(req.frame_offset as usize).unwrap())
                        .unwrap();
                pc_tx2.send((req, pcd)).unwrap();
            }
        }
    });

    // We run the decoder as a separate tokio task.
    // Decoder will read the buffer and send it over to the renderer.
    rt.spawn(async move {
        loop {
            let (req, FetchResult(mut p)) = decoder_rx.recv().await;
            let decoder_path = decoder_path.clone();
            let pc_tx2 = pc_tx.clone();
            _ = tokio::task::spawn_blocking(move || {
                let mut decoder: Box<dyn Decoder> = match decoder_type {
                    DecoderType::Draco => Box::new(DracoDecoder::new(
                        decoder_path
                            .as_ref()
                            .expect("must provide decoder path for Draco")
                            .as_os_str(),
                        p[0].take().unwrap().as_os_str(),
                    )),
                    DecoderType::Multiplane => {
                        Box::new(MultiplaneDecoder::new(MultiplaneDecodeReq {
                            top: p[0].take().unwrap(),
                            bottom: p[1].take().unwrap(),
                            left: p[2].take().unwrap(),
                            right: p[3].take().unwrap(),
                            front: p[4].take().unwrap(),
                            back: p[5].take().unwrap(),
                        }))
                    }
                    _ => Box::new(NoopDecoder::new(p[0].take().unwrap().as_os_str())),
                };
                decoder.start().unwrap();
                let mut i = 0;
                while let Some(pcd) = decoder.poll() {
                    let mut req = req.clone();
                    // update the frame_offset
                    req.frame_offset += i;
                    i += 1;
                    dbg!(req.frame_offset);
                    pc_tx2.send((req, pcd)).unwrap();
                }
            })
            .await
            .unwrap();
        }
    });

    let mut pcd_reader = PcdAsyncReader::new(pc_rx, frame_req_tx, args.buffer_size);
    // set the reader max length
    pcd_reader.set_len(total_frames_rx.blocking_recv().unwrap());
    dbg!(pcd_reader.len());

    let camera = Camera::new(
        (args.camera_x, args.camera_y, args.camera_z),
        cgmath::Deg(args.camera_yaw),
        cgmath::Deg(args.camera_pitch),
    );
    let metrics = args
        .metrics
        .map(|os_str| MetricsReader::from_directory(Path::new(&os_str)));

    let mut builder = RenderBuilder::default();
    let slider_end = pcd_reader.len() - 1;

    // This is the main window that renders the point cloud
    let render_window_id =
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
            pcd_reader,
            args.fps,
            camera,
            (args.width, args.height),
            metrics,
        ));
    // };
    if args.show_controls {
        let controls_window_id = builder.add_window(Controller { slider_end });
        builder
            .get_windowed_mut(render_window_id)
            .unwrap()
            .add_output(controls_window_id);
        builder
            .get_windowed_mut(controls_window_id)
            .unwrap()
            .add_output(render_window_id);
    }
    builder.run();
}
