use clap::Parser;
use image::buffer;
use log::{debug, warn};
use lru::LruCache;
use std::cmp::Reverse;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::codec::decoder::{DracoDecoder, MultiplaneDecodeReq, MultiplaneDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::{FetchResult, Fetcher};
use vivotk::formats::pointxyzrgba::PointXyzRgba;
use vivotk::formats::PointCloud;
use vivotk::quetra::quetracalc::QuetraCalc;
use vivotk::render::wgpu::{
    builder::RenderBuilder,
    camera::Camera,
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{FrameRequest, PcdAsyncReader, RenderReader},
    renderer::Renderer,
};
use vivotk::utils::read_file_to_point_cloud;
use vivotk::{BufMsg, PCMetadata};

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
    buffer_size: Option<usize>,
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

/// Buffer Manager handles 2 interactions:
/// 1. Fetcher & Decoder: buffer manager sends request to source data (either from the network or from the local filesystem).
/// It expects to get a PointCloud back, which it will buffers until the renderer is ready to consume it.
/// 2. Renderer: buffer manager receives request for point cloud from the renderer and returns the point cloud to the renderer.
struct BufferManager {
    to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
    buf_in_sx: tokio::sync::mpsc::UnboundedSender<FrameRequest>,
    buf_out_sx: std::sync::mpsc::Sender<(FrameRequest, PointCloud<PointXyzRgba>)>,
    pending_frame_req: VecDeque<FrameRequest>,
    cache: LruCache<FrameRequest, PointCloud<PointXyzRgba>>,
    total_frames: usize,
    segment_size: u64,
}

impl BufferManager {
    fn new(
        to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
        buf_in_sx: tokio::sync::mpsc::UnboundedSender<FrameRequest>,
        buf_out_sx: std::sync::mpsc::Sender<(FrameRequest, PointCloud<PointXyzRgba>)>,
        buffer_size: usize,
        total_frames: usize,
        segment_size: u64,
    ) -> Self {
        BufferManager {
            to_buf_rx,
            buf_in_sx,
            buf_out_sx,
            cache: LruCache::new(std::num::NonZeroUsize::new(buffer_size).unwrap()),
            pending_frame_req: VecDeque::new(),
            total_frames,
            segment_size,
        }
    }

    async fn run(&mut self) -> ! {
        loop {
            match self.to_buf_rx.recv().await.unwrap() {
                BufMsg::FrameRequest(renderer_req) => {
                    debug!("renderer sent a frame request {:?}", &renderer_req);

                    // First, attempt to fulfill the request from the buffer.
                    // TODO: find which quality to download based on the current camera position + network bandwidth.

                    // TODO: replace currently hardcoded values with values exposed from buffer (i.e. k, r_vec, b, buffer_occupancy)
                    // pass parameters into quetra
                    let r_vec_from_buffer: Vec<f64> = vec![100.0f64, 200.0f64, 300.0f64];

                    let qc = QuetraCalc {
                        name: "quetra".to_owned(),
                        k: 3,
                        r_vec: r_vec_from_buffer,
                        b: 150.0f64,
                        buffer_occupancy: 2,
                        segment_frequency: 1,
                        segment_size: 1,
                    };

                    // call fn to select the best bitrate according to quetra
                    let selected_bitrate = qc.select_bitrate();

                    // Check in cache whether it exists
                    if let Some(pc) = self.cache.pop(&renderer_req) {
                        // send to the renderer
                        self.buf_out_sx.send((renderer_req, pc)).unwrap();
                    } else {
                        // It doesn't exist in cache, so we send a request to the fetcher to fetch the data
                        self.buf_in_sx.send(renderer_req).unwrap();
                        self.pending_frame_req.push_back(renderer_req);
                    }

                    if self.cache.len() < self.cache.cap().get() {
                        debug!("Cache length: {:}", self.cache.len());
                        // If the cache is not full, we send a request to the fetcher to fetch the next frame
                        let mut next_frame_req = renderer_req;
                        next_frame_req.frame_offset = (next_frame_req.frame_offset
                            + self.segment_size)
                            % self.total_frames as u64;
                        self.buf_in_sx.send(next_frame_req).unwrap();
                        // we don't store this in the pending frame request because it is a preemptive request, not a request by the renderer.
                    }
                }
                BufMsg::PointCloud((req, pc)) => {
                    debug!("received a point cloud result {:?}", &req);
                    if !self.pending_frame_req.is_empty()
                        && req.frame_offset == self.pending_frame_req.front().unwrap().frame_offset
                    {
                        // send results to the renderer
                        self.buf_out_sx.send((req.into(), pc)).unwrap();
                        self.pending_frame_req.pop_front();
                    } else {
                        // cache the point cloud
                        self.cache.push(req.into(), pc);
                    }
                }
            }
        }
    }
}

fn main() {
    // initialize logger
    env_logger::init();
    let args: Args = Args::parse();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .unwrap();
    // important to use tokio::mpsc here instead of std because it is bridging from sync -> async
    // the content is produced by the renderer and consumed by the fetcher
    let (buf_in_sx, mut buf_in_rx) = tokio::sync::mpsc::unbounded_channel();
    let (in_dec_sx, mut in_dec_rx) = tokio::sync::mpsc::unbounded_channel();
    let (to_buf_sx, to_buf_rx) = tokio::sync::mpsc::unbounded_channel();
    // this buffer is used to store the fetched data. It is a bounded buffer. It will store the data in segments.
    // the content is produced by the fetcher and consumed by the decoder thread.
    // let (dec_to_buf, decoder_rx) = Buffer::new(args.buffer_size.unwrap_or(10) as usize);
    // the content is produced by the decode or the local file reader and consumed by the renderer
    let (buf_out_sx, buf_out_rx) = std::sync::mpsc::channel();
    // the total frame number we are expecting. This is for display purposes in the renderer only.
    let (total_frames_tx, total_frames_rx) = tokio::sync::oneshot::channel();

    // copy variables to be moved into the async block
    let src = args.src.clone();
    let decoder_type = args.decoder_type;
    let decoder_path = args.decoder_path.clone();

    // We run the fetcher as a separate tokio task. Although it is an infinite loop, it has a lot of await breakpoints.
    // Fetcher will fetch data and send it over to the buffer.
    {
        let to_buf_sx = to_buf_sx.clone();
        rt.spawn(async move {
            if src.starts_with("http") {
                let tmpdir = tempdir().expect("created temp dir to store files");
                let path = tmpdir.path();
                debug!("Downloading files to {}", path.to_str().unwrap());

                let mut fetcher = Fetcher::new(&src, path).await;
                total_frames_tx
                    .send((fetcher.total_frames(), fetcher.segment_size()))
                    .expect("sent total frames");

                let mut frame_range = (0, 0);
                loop {
                    let req: FrameRequest = buf_in_rx.recv().await.unwrap();
                    debug!("got frame requests {:?}", &req);

                    // The request has just been recently fetched (or in the buffer), so we can skip it to avoid some redundant requests to the server.
                    // However,
                    if frame_range.0 < req.frame_offset && req.frame_offset < frame_range.1 {
                        tokio::task::yield_now().await;
                        continue;
                    }
                    // we should probably do *bounded* retry here
                    loop {
                        let p = fetcher.download(req.object_id, req.frame_offset).await;

                        match p {
                            Ok(res) => {
                                in_dec_sx.send((req, res)).unwrap();
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
                    .send((ply_files.len(), 1))
                    .expect("sent total frames");
                ply_files.sort();

                loop {
                    let req: FrameRequest = buf_in_rx.recv().await.unwrap();
                    debug!("got frame requests {:?}", req);
                    let pcd =
                        read_file_to_point_cloud(ply_files.get(req.frame_offset as usize).unwrap())
                            .expect("read file to point cloud failed");
                    to_buf_sx
                        .send(BufMsg::PointCloud((req.into(), pcd)))
                        .unwrap();
                }
            }
        });
    }

    // We run the decoder as a separate tokio task.
    // Decoder will read the buffer and send it over to the renderer.
    {
        let to_buf_sx = to_buf_sx.clone();
        rt.spawn(async move {
            loop {
                let (
                    req,
                    FetchResult {
                        paths: mut p,
                        last5_avg_bitrate,
                    },
                ) = in_dec_rx.recv().await.unwrap();
                debug!("got fetch result {:?}", req);
                let decoder_path = decoder_path.clone();
                let to_buf_sx = to_buf_sx.clone();
                tokio::task::spawn_blocking(move || {
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
                    let now = std::time::Instant::now();
                    decoder.start().unwrap();
                    let mut i = 0;
                    while let Some(pcd) = decoder.poll() {
                        let pc_metadata = PCMetadata {
                            last5_avg_bitrate,
                            frame_offset: req.frame_offset + i,
                            object_id: req.object_id,
                        };
                        i += 1;
                        to_buf_sx
                            .send(BufMsg::PointCloud((pc_metadata, pcd)))
                            .unwrap();
                    }
                    let elapsed = now.elapsed();
                    dbg!(elapsed);
                })
                .await
                .unwrap();
            }
        });
    }

    let (total_frames, segment_size) = total_frames_rx.blocking_recv().unwrap();

    let mut buffer = BufferManager::new(
        to_buf_rx,
        buf_in_sx,
        buf_out_sx,
        args.buffer_size.unwrap_or(60), // 2s buffer
        total_frames,
        segment_size,
    );
    rt.spawn(async move { buffer.run().await });

    // let mut pcd_reader = PcdAsyncReader::new(buf_out_rx, out_buf_sx, args.buffer_size);
    let mut pcd_reader = PcdAsyncReader::new(buf_out_rx, to_buf_sx);
    // set the reader max length
    pcd_reader.set_len(total_frames);
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
