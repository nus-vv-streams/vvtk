use cgmath::Point3;
use clap::Parser;
use log::{debug, warn};
use lru::LruCache;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::abr::quetra::Quetra;
use vivotk::abr::RateAdapter;
use vivotk::codec::decoder::{DracoDecoder, MultiplaneDecodeReq, MultiplaneDecoder, NoopDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::{FetchResult, Fetcher};
use vivotk::formats::pointxyzrgba::PointXyzRgba;
use vivotk::formats::PointCloud;
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
    buffer_capacity: Option<usize>,
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct BufferCacheKey {
    pub object_id: u8,
    pub frame_offset: u64,
}

impl From<FrameRequest> for BufferCacheKey {
    fn from(req: FrameRequest) -> Self {
        Self {
            object_id: req.object_id,
            frame_offset: req.frame_offset,
        }
    }
}

impl From<PCMetadata> for BufferCacheKey {
    fn from(metadata: PCMetadata) -> Self {
        Self {
            object_id: metadata.object_id,
            frame_offset: metadata.frame_offset,
        }
    }
}

/// Buffer Manager handles 2 interactions:
/// 1. Fetcher & Decoder: buffer manager sends request to source data (either from the network or from the local filesystem).
/// It expects to get a PointCloud back, which it will put into its buffer until the renderer is ready to consume it.
/// 2. Renderer: buffer manager receives request for point cloud from the renderer and returns the (assembled) point cloud to the renderer.
///
/// The interaction flow:
/// - Buffer manager receives a request from the renderer.
/// - Buffer manager checks if the requested point cloud is in its buffer.
///     - If it is, buffer manager returns the point cloud to the renderer.
///     - If it is not, buffer manager sends a request to the source data (either from the network or from the local filesystem).
/// - Buffer manager receives the point cloud from the decoder and puts it into its buffer.
/// - Buffer manager returns the point cloud to the renderer.
struct BufferManager {
    to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
    buf_in_sx: tokio::sync::mpsc::UnboundedSender<FetchRequest>,
    buf_out_sx: std::sync::mpsc::Sender<(FrameRequest, PointCloud<PointXyzRgba>)>,
    pending_frame_req: VecDeque<FrameRequest>,
    /// How does the cache work?
    ///
    /// The cache is essentially a hashmap. The key contains the object_id and frame_offset. The value is a channel that the decoder will send the point cloud to.
    /// Everytime a point cloud is received from the channel, the cache will increment the key's frame_offset and store it back into the cache.
    /// When the channel returns None, it means that the decoder has finished decoding the point cloud and we will remove the entry from the cache.
    ///
    /// Note that the cache prioritizes updates made by BufMsg::PointCloud compared to updates done by incrementing the key's frame_offset.
    /// This is because BufMsg::PointCloud contains the new channel that the decoder promised to send the point cloud to, whereas our update is a bet that the channel is not empty yet.
    cache: LruCache<BufferCacheKey, tokio::sync::mpsc::UnboundedReceiver<PointCloud<PointXyzRgba>>>,
    total_frames: usize,
    segment_size: u64,
}

impl BufferManager {
    fn new(
        to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
        buf_in_sx: tokio::sync::mpsc::UnboundedSender<FetchRequest>,
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

    async fn run(&mut self) {
        loop {
            match self.to_buf_rx.recv().await.unwrap() {
                BufMsg::FrameRequest(renderer_req) => {
                    // First, attempt to fulfill the request from the buffer.
                    // Check in cache whether it exists
                    if let Some(mut rx) = self.cache.pop(&BufferCacheKey::from(renderer_req)) {
                        // send to the renderer
                        match rx.recv().await {
                            Some(pc) => {
                                self.buf_out_sx.send((renderer_req, pc)).unwrap();
                                let mut next_key = BufferCacheKey::from(renderer_req);
                                next_key.frame_offset += 1;
                                if !self.cache.contains(&next_key) {
                                    self.cache.put(next_key, rx);
                                }
                            }
                            None => {
                                // channel is empty, so we discard this channel and enqueue the request into the pending queue
                                // so that the next iteration knows to send the response to renderer.
                                self.pending_frame_req.push_back(renderer_req);
                            }
                        }
                    } else {
                        // It doesn't exist in cache, so we send a request to the fetcher to fetch the data
                        self.buf_in_sx
                            .send(FetchRequest::new(renderer_req, self.cache.len()))
                            .unwrap();
                        self.pending_frame_req.push_back(renderer_req);
                    }

                    // NOTE(9Mar23): Although this bit of code looks spammy (we send a predictive request to the fetcher every time we receive a request from the renderer),
                    // the overhead is only in sending the FetchRequest. The fetcher will only send a request to the network if it isn't recently requested.
                    // However, this behaviour might change in the future.
                    if self.cache.len() < self.cache.cap().get() {
                        debug!("Cache length: {:}", self.cache.len());
                        // If the cache is not full, we send a request to the fetcher to fetch the next frame
                        let mut next_frame_req = renderer_req;
                        next_frame_req.frame_offset = (next_frame_req.frame_offset
                            + self.segment_size)
                            % self.total_frames as u64;
                        self.buf_in_sx
                            .send(FetchRequest::new(next_frame_req, self.cache.len()))
                            .unwrap();
                        // we don't store this in the pending frame request because it is a preemptive request, not a request by the renderer.
                    }
                }
                BufMsg::PointCloud((mut metadata, mut rx)) => {
                    if !self.pending_frame_req.is_empty()
                        && metadata.frame_offset
                            == self.pending_frame_req.front().unwrap().frame_offset
                    {
                        let pc = rx.recv().await.unwrap();
                        // send results to the renderer
                        self.buf_out_sx.send((metadata.into(), pc)).unwrap();
                        self.pending_frame_req.pop_front();
                        metadata.frame_offset += 1;
                    }

                    // cache the point cloud
                    self.cache.push(metadata.into(), rx);
                }
                BufMsg::Fov(_) => {}
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FetchRequest {
    pub object_id: u8,
    // pub quality: u8,
    /// Frame offset from the start of the video.
    ///
    /// To get the frame number, add the offset to the frame number of the first frame in the video.
    pub frame_offset: u64,
    /// The camera position when the frame was requested.
    pub camera_pos: Option<Point3<f32>>,
    buffer_occupancy: usize,
}

impl FetchRequest {
    fn new(req: FrameRequest, buffer_occupancy: usize) -> Self {
        FetchRequest {
            object_id: req.object_id,
            frame_offset: req.frame_offset,
            camera_pos: req.camera_pos,
            buffer_occupancy,
        }
    }
}

impl Into<PCMetadata> for FetchRequest {
    fn into(self) -> PCMetadata {
        PCMetadata {
            object_id: self.object_id,
            frame_offset: self.frame_offset,
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
    // TODO: fix this. This is a hack to pass information from the fetcher to this main thread.
    let (total_frames_tx, total_frames_rx) = tokio::sync::oneshot::channel();

    let buffer_capacity = args.buffer_capacity.unwrap_or(60);

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
                    .send((fetcher.total_frames(), fetcher.segment_duration()))
                    .expect("sent total frames");

                let mut frame_range = (0, 0);
                let abr = Quetra::new(buffer_capacity as u64);

                loop {
                    let req: FetchRequest = buf_in_rx.recv().await.unwrap();
                    debug!("got fetch requests {:?}", &req);

                    // The request has just been recently fetched (or in the buffer), so we can skip it to avoid some redundant requests to the server.
                    // However,
                    if frame_range.0 < req.frame_offset && req.frame_offset < frame_range.1 {
                        tokio::task::yield_now().await;
                        continue;
                    }
                    // we should probably do *bounded* retry here
                    loop {
                        // TODO: find which quality to download based on the current camera position + network bandwidth.
                        let _camera_pos = req.camera_pos;

                        let quality = abr.select_quality(
                            req.buffer_occupancy as u64,
                            fetcher.stats.avg_bitrate.get() as f64,
                            &fetcher.available_bitrates(req.object_id, req.frame_offset, None),
                        );

                        let p = fetcher
                            .download(req.object_id, req.frame_offset, Some(quality as u8 + 1))
                            .await;

                        match p {
                            Ok(res) => {
                                in_dec_sx.send((req, res)).unwrap();
                                frame_range.0 = req.frame_offset;
                                frame_range.1 = req.frame_offset + fetcher.segment_duration();
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
                    let (output_sx, output_rx) = tokio::sync::mpsc::unbounded_channel();

                    let req: FetchRequest = buf_in_rx.recv().await.unwrap();
                    debug!("got frame requests {:?}", req);
                    let pcd =
                        read_file_to_point_cloud(ply_files.get(req.frame_offset as usize).unwrap())
                            .expect("read file to point cloud failed");
                    output_sx.send(pcd).unwrap();
                    to_buf_sx
                        .send(BufMsg::PointCloud((req.into(), output_rx)))
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
                    let (output_sx, output_rx) = tokio::sync::mpsc::unbounded_channel();
                    to_buf_sx
                        .send(BufMsg::PointCloud((
                            PCMetadata {
                                frame_offset: req.frame_offset,
                                object_id: req.object_id,
                            },
                            output_rx,
                        )))
                        .unwrap();
                    while let Some(pcd) = decoder.poll() {
                        output_sx.send(pcd).unwrap();
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
        // 2s buffer
        buffer_capacity,
        total_frames,
        segment_size,
    );
    rt.spawn(async move { buffer.run().await });

    // let mut pcd_reader = PcdAsyncReader::new(buf_out_rx, out_buf_sx, args.buffer_size);
    let mut pcd_reader = PcdAsyncReader::new(buf_out_rx, to_buf_sx);
    // set the reader max length
    pcd_reader.set_len(total_frames);

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
