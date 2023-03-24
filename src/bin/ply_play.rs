use cgmath::Point3;
use clap::Parser;
use log::{debug, trace, warn};
use lru::LruCache;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::abr::quetra::{Quetra, QuetraMultiview};
use vivotk::abr::{RateAdapter, MCKP};
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder, Tmc2rsDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::{FetchResult, Fetcher};
use vivotk::dash::{ThroughputPrediction, ViewportPrediction};
use vivotk::formats::pointxyzrgba::PointXyzRgba;
use vivotk::formats::PointCloud;
use vivotk::render::wgpu::{
    builder::{EventType, RenderBuilder, RenderEvent},
    camera::{Camera, CameraPosition},
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{FrameRequest, PcdAsyncReader, RenderReader},
    renderer::Renderer,
};
use vivotk::utils::{
    get_cosines, predict_quality, read_file_to_point_cloud, ExponentialMovingAverage, LastValue,
    SimpleRunningAverage, GAEMA, LPEMA,
};
use vivotk::{BufMsg, PCMetadata};

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// src can be:
    /// 1. Directory with all the pcd files in lexicographical order
    /// 2. location of the mpd file
    src: String,
    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.3)]
    camera_z: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    camera_yaw: f32,
    #[clap(short, long, default_value_t = 1600)]
    width: u32,
    #[clap(short, long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls", action = clap::ArgAction::SetTrue, default_value_t = true)]
    show_controls: bool,
    #[clap(short, long)]
    buffer_capacity: Option<usize>,
    #[clap(short, long)]
    metrics: Option<OsString>,
    #[clap(long = "abr", value_enum, default_value_t = AbrType::Quetra)]
    abr_type: AbrType,
    #[clap(long = "decoder", value_enum, default_value_t = DecoderType::Tmc2rs)]
    decoder_type: DecoderType,
    #[clap(long, action = clap::ArgAction::SetTrue)]
    multiview: bool,
    /// Path to the decoder binary (only for Draco)
    #[clap(long)]
    decoder_path: Option<PathBuf>,
    #[clap(long = "tp", value_enum, default_value_t = ThroughputPredictionType::Last)]
    throughput_prediction_type: ThroughputPredictionType,
    /// Alpha for throughput prediction. Only used for EMA, GAEMA, and LPEMA
    #[clap(long, default_value_t = 0.1)]
    throughput_alpha: f64,
    #[clap(long = "vp", value_enum, default_value_t = ViewportPredictionType::Last)]
    viewport_prediction_type: ViewportPredictionType,
    /// Path to network trace for repeatable simulation. Network trace is expected to be given in Kbps
    #[clap(long)]
    network_trace: Option<PathBuf>,
    /// Path to camera trace for repeatable simulation. Camera trace is expected to be given in (pos_x, pos_y, pos_z, rot_pitch, rot_yaw, rot_roll).
    /// Rotation is in degrees
    #[clap(long)]
    camera_trace: Option<PathBuf>,
    /// Path to record camera trace from the player.
    #[clap(long)]
    record_camera_trace: Option<PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum DecoderType {
    Noop,
    Draco,
    Tmc2rs,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum AbrType {
    Quetra,
    QuetraMultiview,
    Mckp,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum ThroughputPredictionType {
    /// Last throughput
    Last,
    /// Average of last 3 throughput
    Avg,
    /// ExponentialMovingAverage,
    Ema,
    /// Gradient Adaptive Exponential Moving Average
    Gaema,
    /// Low Pass Exponential Moving Average
    Lpema,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum ViewportPredictionType {
    /// Last viewport
    Last,
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
    shutdown_recv: tokio::sync::watch::Receiver<bool>,
}

impl BufferManager {
    fn new(
        to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
        buf_in_sx: tokio::sync::mpsc::UnboundedSender<FetchRequest>,
        buf_out_sx: std::sync::mpsc::Sender<(FrameRequest, PointCloud<PointXyzRgba>)>,
        buffer_size: usize,
        total_frames: usize,
        segment_size: u64,
        shutdown_recv: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        BufferManager {
            to_buf_rx,
            buf_in_sx,
            buf_out_sx,
            cache: LruCache::new(std::num::NonZeroUsize::new(buffer_size).unwrap()),
            pending_frame_req: VecDeque::new(),
            total_frames,
            segment_size,
            shutdown_recv,
        }
    }

    async fn run(
        &mut self,
        mut viewport_predictor: Box<dyn ViewportPrediction>,
        camera_trace: Option<CameraTrace>,
        mut record_camera_trace: Option<CameraTrace>,
    ) {
        loop {
            tokio::select! {
                _ = self.shutdown_recv.changed() => {
                    trace!("[buffer mgr] received shutdown signal");
                    break;
                }
                Some(msg) = self.to_buf_rx.recv() => {
                    match msg {
                        BufMsg::FrameRequest(mut renderer_req) => {
                            trace!(
                                "[buffer mgr] renderer sent a frame request {:?}",
                                &renderer_req
                            );

                            if record_camera_trace.is_some() && renderer_req.camera_pos.is_some() {
                                if let Some(ct) = record_camera_trace.as_mut() { ct.add(renderer_req.camera_pos.unwrap()) }
                            }

                            // If the camera trace is provided, we will use the camera trace to override the camera position for the next frame
                            // else we will feed this into the viewport predictor
                            if camera_trace.is_some() {
                                renderer_req.camera_pos = camera_trace.as_ref().map(|ct| ct.next());
                            } else {
                                viewport_predictor.add(renderer_req.camera_pos);
                            }
                            // First, attempt to fulfill the request from the buffer.
                            // Check in cache whether it exists
                            if let Some(mut rx) = self.cache.pop(&BufferCacheKey::from(renderer_req)) {
                                // send to the renderer
                                match rx.recv().await {
                                    Some(pc) => {
                                        // if camera trace is not provided, we should not send camera_pos back to the renderer
                                        // as it is just a prediction, not an instruction to move to that position
                                        if camera_trace.is_none() {
                                            renderer_req.camera_pos = None;
                                        }
                                        _ = self.buf_out_sx.send((renderer_req, pc));
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
                                // First we change the camera_pos as predicted by the viewport predictor, if camera trace is not provided
                                if camera_trace.is_none() {
                                    renderer_req.camera_pos = viewport_predictor.predict();
                                }
                                _ = self.buf_in_sx.send(FetchRequest::new(renderer_req, self.cache.len()));
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
                                _ = self.buf_in_sx.send(FetchRequest::new(next_frame_req, self.cache.len()));
                                // we don't store this in the pending frame request because it is a preemptive request, not a request by the renderer.
                            }
                        }
                        BufMsg::PointCloud((mut metadata, mut rx)) => {
                            trace!("[buffer mgr] received a point cloud result {:?}", &metadata);

                            if !self.pending_frame_req.is_empty()
                                && metadata.frame_offset
                                    == self.pending_frame_req.front().unwrap().frame_offset
                            {
                                let pc = rx.recv().await.unwrap();
                                // send results to the renderer
                                _ = self.buf_out_sx.send((metadata.into(), pc));
                                self.pending_frame_req.pop_front();
                                metadata.frame_offset += 1;
                            }

                            // cache the point cloud
                            self.cache.push(metadata.into(), rx);
                        }
                    }
                }
                else => break,
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
    pub camera_pos: Option<CameraPosition>,
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

impl From<FetchRequest> for PCMetadata {
    fn from(val: FetchRequest) -> Self {
        PCMetadata {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
        }
    }
}

struct NetworkTrace {
    data: Vec<f64>,
    index: RefCell<usize>,
}

impl NetworkTrace {
    /// The network trace file to contain the network bandwidth in Kbps, each line representing 1 bandwidth sample.
    /// # Arguments
    ///
    /// * `path` - The path to the network trace file.
    fn new(path: &Path) -> Self {
        use std::io::BufRead;

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data = reader
            .lines()
            .map(|line| line.unwrap().trim().parse::<f64>().unwrap())
            .collect();
        NetworkTrace {
            data,
            index: RefCell::new(0),
        }
    }

    // Get the next bandwidth sample
    fn next(&self) -> f64 {
        let idx = *self.index.borrow();
        let next_idx = (idx + 1) % self.data.len();
        *self.index.borrow_mut() = next_idx;
        self.data[idx]
    }
}

struct CameraTrace {
    data: Vec<CameraPosition>,
    index: RefCell<usize>,
    path: PathBuf,
}

impl CameraTrace {
    /// The network trace file to contain the network bandwidth in Kbps, each line representing 1 bandwidth sample.
    /// # Arguments
    ///
    /// * `path` - The path to the network trace file.
    fn new(path: &Path, is_record: bool) -> Self {
        use std::io::BufRead;
        match File::open(path) {
            Err(err) => {
                if !is_record {
                    panic!("Failed to open camera trace file: {err:?}");
                }
                Self {
                    data: Vec::new(),
                    index: RefCell::new(0),
                    path: path.to_path_buf(),
                }
            }
            Ok(file) => {
                if is_record {
                    panic!("Camera trace file already exists: {path:?}");
                }
                let reader = BufReader::new(file);
                let data = reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        let mut it = line.trim().split(',').map(|s| s.parse::<f32>().unwrap());
                        let position =
                            Point3::new(it.next().unwrap(), it.next().unwrap(), it.next().unwrap());
                        let pitch = cgmath::Deg(it.next().unwrap()).into();
                        let yaw = cgmath::Deg(it.next().unwrap()).into();
                        CameraPosition {
                            position,
                            pitch,
                            yaw,
                        }
                    })
                    .collect();
                Self {
                    data,
                    index: RefCell::new(0),
                    path: path.to_path_buf(),
                }
            }
        }
    }

    /// Get the next bandwidth sample. Used when playing back a camera trace.
    fn next(&self) -> CameraPosition {
        let idx = *self.index.borrow();
        let next_idx = (idx + 1) % self.data.len();
        *self.index.borrow_mut() = next_idx;
        self.data[idx]
    }

    /// Add a new position to the trace. Used when recording a camera trace.
    fn add(&mut self, pos: CameraPosition) {
        self.data.push(pos);
    }
}

impl Drop for CameraTrace {
    fn drop(&mut self) {
        use std::io::BufWriter;
        use std::io::Write;

        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
        {
            Ok(mut file) => {
                let mut writer = BufWriter::new(&mut file);
                for pos in &self.data {
                    writeln!(
                        writer,
                        "{},{},{},{},{},0.0",
                        pos.position.x,
                        pos.position.y,
                        pos.position.z,
                        pos.pitch.0.to_degrees(),
                        pos.yaw.0.to_degrees()
                    )
                    .unwrap();
                }
            }
            Err(_) => {
                warn!("Camera trace file already exists, not writing");
            }
        }
    }
}

fn main() {
    // initialize logger
    env_logger::init();
    let args: Args = Args::parse();
    dbg!(args.multiview);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .unwrap();
    let (shutdown_send, shutdown_recv) = tokio::sync::watch::channel(false);
    // important to use tokio::mpsc here instead of std because it is bridging from sync -> async
    // the content is produced by the renderer and consumed by the fetcher
    let (buf_in_sx, mut buf_in_rx) = tokio::sync::mpsc::unbounded_channel::<FetchRequest>();
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

    // initialize variables based on args
    let buffer_capacity = args.buffer_capacity.unwrap_or(4);
    let simulated_network_trace = args.network_trace.map(|path| NetworkTrace::new(&path));
    let simulated_camera_trace = args.camera_trace.map(|path| CameraTrace::new(&path, false));
    let record_camera_trace = args
        .record_camera_trace
        .map(|path| CameraTrace::new(&path, true));

    // copy variables to be moved into the async block
    let src = args.src.clone();
    let decoder_type = args.decoder_type;
    let decoder_path = args.decoder_path.clone();

    // We run the fetcher as a separate tokio task. Although it is an infinite loop, it has a lot of await breakpoints.
    // Fetcher will fetch data and send it over to the buffer.
    {
        let to_buf_sx = to_buf_sx.clone();
        let mut shutdown_recv = shutdown_recv.clone();
        let mut throughput_predictor: Box<dyn ThroughputPrediction> =
            match args.throughput_prediction_type {
                ThroughputPredictionType::Last => Box::new(LastValue::new()),
                ThroughputPredictionType::Avg => Box::new(SimpleRunningAverage::<f64, 3>::new()),
                ThroughputPredictionType::Ema => {
                    Box::new(ExponentialMovingAverage::new(args.throughput_alpha))
                }
                ThroughputPredictionType::Gaema => Box::new(GAEMA::new(args.throughput_alpha)),
                ThroughputPredictionType::Lpema => Box::new(LPEMA::new(args.throughput_alpha)),
            };

        rt.spawn(async move {
            if src.starts_with("http") {
                let tmpdir = tempdir().expect("created temp dir to store files");
                let path = tmpdir.path();
                trace!("[fetcher] Downloading files to {}", path.to_str().unwrap());

                let mut fetcher = Fetcher::new(&src, path).await;
                total_frames_tx
                    .send((
                        fetcher.mpd_parser.total_frames(),
                        fetcher.mpd_parser.segment_duration(),
                    ))
                    .expect("sent total frames");

                let qualities = fetcher
                    .mpd_parser
                    .get_qp()
                    .into_iter()
                    .map(|x| -> f32 {
                        if let (Some(geo_qp), Some(attr_qp)) = x {
                            predict_quality(geo_qp as f32, attr_qp as f32)
                        } else {
                            0.0
                        }
                    })
                    .collect();

                let abr: Box<dyn RateAdapter> = match args.abr_type {
                    AbrType::Quetra => Box::new(Quetra::new(buffer_capacity as u64, args.fps)),
                    AbrType::Mckp => Box::new(MCKP::new(6, qualities)),
                    AbrType::QuetraMultiview => {
                        Box::new(QuetraMultiview::new(buffer_capacity as u64, args.fps, 6, qualities))
                    }
                };

                let mut frame_range = (0, 0);
                loop {
                    tokio::select! {
                        _ = shutdown_recv.changed() => {
                            trace!("[fetcher] shutdown signal received");
                             _ = tmpdir.close();
                            break;
                        },
                        Some(req) = buf_in_rx.recv() => {
                            // The request has just been recently fetched (or in the buffer), so we can skip it to avoid some redundant requests to the server.
                            // However,
                            if frame_range.0 < req.frame_offset && req.frame_offset < frame_range.1 {
                                tokio::task::yield_now().await;
                                continue;
                            }

                            let camera_pos = req.camera_pos.unwrap_or(CameraPosition {
                                position: Point3::new(args.camera_x, args.camera_y, args.camera_z),
                                yaw: cgmath::Deg(args.camera_yaw).into(),
                                pitch: cgmath::Deg(args.camera_pitch).into(),
                            });

                            // We start with a guess of 1Mbps network throughput.
                            let network_throughput = if simulated_network_trace.is_none() {
                                throughput_predictor.predict().unwrap_or(10_000_000.0)
                            } else {
                                simulated_network_trace.as_ref().unwrap().next() * 1024.0
                            };

                            let mut available_bitrates = vec![];
                            if args.multiview {
                                for i in 0..6 {
                                    available_bitrates.push(fetcher.available_bitrates(
                                        req.object_id,
                                        req.frame_offset,
                                        Some(i),
                                    ));
                                }
                            } else {
                                available_bitrates.push(fetcher.available_bitrates(
                                    req.object_id,
                                    req.frame_offset,
                                    None,
                                ));
                            }

                            let cosines = get_cosines(camera_pos);

                            let quality = abr.select_quality(
                                req.buffer_occupancy as u64,
                                network_throughput,
                                &available_bitrates,
                                &cosines,
                            );

                            // This is a retry loop, we should probably do *bounded* retry here instead of looping indefinitely.
                            loop {
                                trace!("[fetcher] trying request {:?}", &req);

                                let p = fetcher
                                    .download(req.object_id, req.frame_offset, &quality, args.multiview)
                                    .await;

                                match p {
                                    Ok(res) => {
                                        // update throughput prediction
                                        throughput_predictor.add(res.throughput);
                                        _ = in_dec_sx.send((req, res));
                                        frame_range.0 = req.frame_offset;
                                        frame_range.1 =
                                            req.frame_offset + fetcher.mpd_parser.segment_duration();
                                        break;
                                    }
                                    Err(e) => {
                                        warn!("Error downloading file: {}", e)
                                    }
                                }
                            }
                        }
                        else => {
                            _ = tmpdir.close();
                            break;
                        }
                    }
                }
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
                    tokio::select! {
                        _ = shutdown_recv.changed() => {
                            trace!("[fetcher] shutdown signal received");
                            break;
                        },
                        Some(req) = buf_in_rx.recv() => {
                            let (output_sx, output_rx) = tokio::sync::mpsc::unbounded_channel();
                            trace!("[fetcher] got fetch request {:?}", req);
                            let pcd =
                                read_file_to_point_cloud(ply_files.get(req.frame_offset as usize).unwrap())
                                    .expect("read file to point cloud failed");
                            _ = output_sx.send(pcd);
                            // ignore if failed to send to renderer
                            _ = to_buf_sx.send(BufMsg::PointCloud((req.into(), output_rx)));
                        }
                        else => break,
                    }
                }
            }
        });
    }

    // We run the decoder as a separate tokio task.
    // Decoder will read the buffer and send it over to the renderer.
    {
        let to_buf_sx = to_buf_sx.clone();
        let mut shutdown_recv = shutdown_recv.clone();
        rt.spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_recv.changed() => {
                        trace!("[decoder] shutdown signal received");
                        break;
                    },
                    Some((req, FetchResult {
                        mut paths,
                        throughput: _,
                    })) = in_dec_rx.recv() => {
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
                                    paths[0].take().unwrap().as_os_str(),
                                )),
                                DecoderType::Tmc2rs => {
                                    let paths = paths.into_iter().flatten().collect::<Vec<_>>();
                                    Box::new(Tmc2rsDecoder::new(&paths))
                                }
                                _ => Box::new(NoopDecoder::new(paths[0].take().unwrap().as_os_str())),
                            };
                            let now = std::time::Instant::now();
                            decoder.start().unwrap();
                            let (output_sx, output_rx) = tokio::sync::mpsc::unbounded_channel();
                            _ = to_buf_sx
                                .send(BufMsg::PointCloud((
                                    PCMetadata {
                                        frame_offset: req.frame_offset,
                                        object_id: req.object_id,
                                    },
                                    output_rx,
                                )));
                            while let Some(pcd) = decoder.poll() {
                                _ = output_sx.send(pcd);
                            }
                            let elapsed = now.elapsed();
                            dbg!(elapsed);
                        })
                        .await
                        .unwrap();
                    }
                    else => break,
                }
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
        shutdown_recv,
    );
    let viewport_predictor: Box<dyn ViewportPrediction> = match args.viewport_prediction_type {
        ViewportPredictionType::Last => Box::new(LastValue::new()),
    };
    rt.spawn(async move {
        buffer
            .run(
                viewport_predictor,
                simulated_camera_trace,
                record_camera_trace,
            )
            .await
    });

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

    {
        // We run the shutdown signal listener as a separate tokio task.
        let event_proxy = builder.get_proxy();
        let window_ids = builder.get_window_ids();
        rt.spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    for window_id in window_ids {
                        event_proxy
                            .send_event(RenderEvent::new(window_id, EventType::Shutdown))
                            .unwrap();
                    }
                    shutdown_send.send(true).unwrap();
                }
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {err}");
                    // we also shut down in case of error
                }
            }
        });
    }

    // In MacOS, renderer must run in main thread.
    builder.run();
}
