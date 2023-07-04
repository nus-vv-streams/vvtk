use cgmath::Point3;
use clap::Parser;
use log::{debug, info, trace, warn};
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::abr::quetra::{Quetra, QuetraMultiview};
use vivotk::abr::{RateAdapter, MCKP};
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder, Tmc2rsDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::buffer::{Buffer, FrameStatus};
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
    get_cosines, predict_quality, ExponentialMovingAverage, LastValue,
    SimpleRunningAverage, GAEMA, LPEMA,
};
use vivotk::{BufMsg, PCMetadata};

/// Plays a folder of pcd files in lexicographical order
#[derive(Parser)]
struct Args {
    /// src can be:
    ///
    /// 1. Directory with all the ply files
    /// 2. location of the mpd url (dash)
    src: String,
    #[clap(short, long, default_value_t = 30.0)]
    fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.5)]
    camera_z: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    camera_yaw: f32,
    /// Set the screen width.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short = 'W', long, default_value_t = 1600)]
    width: u32,
    /// Set the screen height.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short = 'H', long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls", action = clap::ArgAction::SetTrue, default_value_t = true)]
    show_controls: bool,
    /// buffer capacity in seconds
    #[clap(short, long)]
    buffer_capacity: Option<u64>,
    #[clap(short, long)]
    metrics: Option<OsString>,
    #[clap(long = "abr", value_enum, default_value_t = AbrType::Quetra)]
    abr_type: AbrType,
    #[clap(long = "decoder", value_enum, default_value_t = DecoderType::Noop)]
    decoder_type: DecoderType,
    /// Set this flag if each view is encoded separately, i.e. multiview
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
    /// Enable fetcher optimizations
    ///
    /// 1. Not fetching when file has been previously downloaded.
    #[clap(long, action = clap::ArgAction::SetTrue)]
    enable_fetcher_optimizations: bool,
    #[clap(long, default_value = "rgb(255,255,255)")]
    bg_color: OsString,
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
    /// frame_to_answer is the frame we are pending to answer to the renderer.
    /// Note(25Mar23): it is an option because we are only dealing with 1 object_id for now.
    frame_to_answer: Option<FrameRequest>,
    /// buffer stores all requests, it might be in fetching or decoding or ready state.
    buffer: Buffer,
    total_frames: usize,
    segment_size: u64,
    shutdown_recv: tokio::sync::watch::Receiver<bool>,
}

impl BufferManager {
    fn new(
        to_buf_rx: tokio::sync::mpsc::UnboundedReceiver<BufMsg>,
        buf_in_sx: tokio::sync::mpsc::UnboundedSender<FetchRequest>,
        buf_out_sx: std::sync::mpsc::Sender<(FrameRequest, PointCloud<PointXyzRgba>)>,
        buffer_size: u64,
        total_frames: usize,
        segment_size: (u64, u64),
        shutdown_recv: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        BufferManager {
            to_buf_rx,
            buf_in_sx,
            buf_out_sx,
            frame_to_answer: None,
            total_frames,
            segment_size: segment_size.0,
            shutdown_recv,
            // buffer size is given in seconds. however our frames are only segment_size.0 / segment_size.1 seconds long.
            buffer: Buffer::new((buffer_size * segment_size.1 / segment_size.0) as usize),
        }
    }

    /// Get next frame request assuming playback is continuous
    fn get_next_frame_req(&self, req: &FrameRequest) -> FrameRequest {
        FrameRequest {
            object_id: req.object_id,
            frame_offset: (req.frame_offset + self.segment_size) % self.total_frames as u64,
            camera_pos: req.camera_pos,
        }
    }

    fn prefetch_frame(&mut self, camera_pos: Option<CameraPosition>) {
        assert!(camera_pos.is_some());
        let last_req = FrameRequest {
            camera_pos,
            ..self.buffer.back().unwrap().req
        };
        let req = self.get_next_frame_req(&last_req);
        _ = self
            .buf_in_sx
            .send(FetchRequest::new(req, self.buffer.len()));

        self.buffer.add(req);
    }

    async fn run(
        &mut self,
        mut viewport_predictor: Box<dyn ViewportPrediction>,
        original_position: CameraPosition,
        camera_trace: Option<CameraTrace>,
        mut record_camera_trace: Option<CameraTrace>,
    ) {
        // Since we prefetch after a `FetchDone` event, once the buffer is full, we can't prefetch anymore.
        // So, we set this flag to true once the buffer is full, so that when the frames are consumed and the first channels are discarded, we can prefetch again.
        let mut is_desired_buffer_level_reached = false;
        loop {
            trace!("buffer: {:?}", &self.buffer);
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

                            // record camera trace
                            if record_camera_trace.is_some() && renderer_req.camera_pos.is_some() {
                                if let Some(ct) = record_camera_trace.as_mut() { ct.add(renderer_req.camera_pos.unwrap()) }
                            }

                            // If the camera trace is provided, we will use the camera trace to override the camera position for the next frame
                            // else we will feed this into the viewport predictor
                            if camera_trace.is_some() {
                                renderer_req.camera_pos = camera_trace.as_ref().map(|ct| ct.next());
                            } else {
                                viewport_predictor.add(renderer_req.camera_pos.unwrap_or_else(|| original_position));
                                renderer_req.camera_pos = viewport_predictor.predict();
                            }

                            // First, attempt to fulfill the request from the buffer.
                            // Check in cache whether it exists
                            if !self.buffer.is_empty() && self.buffer.front().unwrap().req.frame_offset == renderer_req.frame_offset {
                                let mut front = self.buffer.pop_front().unwrap();
                                match front.state {
                                    FrameStatus::Fetching | FrameStatus::Decoding => {
                                        // we update frame_to_answer to indicate that we are waiting to send back this data to renderer.
                                        self.frame_to_answer = Some(renderer_req);
                                        self.buffer.push_front(front);
                                    }
                                    FrameStatus::Ready(remaining_frames, mut rx) => {
                                        // send to the renderer
                                        match rx.recv().await {
                                            Some(pc) => {
                                                // if camera trace is not provided, we should not send camera_pos back to the renderer
                                                // as it is just a prediction, not an instruction to move to that position
                                                let original_camera_pos = if camera_trace.is_none() {
                                                    renderer_req.camera_pos.take()
                                                } else {
                                                    renderer_req.camera_pos
                                                };
                                                // send to point cloud to renderer
                                                _ = self.buf_out_sx.send((renderer_req, pc));
                                                self.frame_to_answer = None;

                                                front.req.frame_offset += 1;
                                                front.state = FrameStatus::Ready(remaining_frames - 1, rx);
                                                if remaining_frames > 1 {
                                                    // we only reinsert it if there are more frames to render
                                                    self.buffer.push_front(front);
                                                } else if is_desired_buffer_level_reached {
                                                    self.prefetch_frame(original_camera_pos);
                                                    is_desired_buffer_level_reached = false;
                                                }
                                            }
                                            None => {
                                                unreachable!("we should never have an empty channel");
                                                // channel is empty, so we discard this channel
                                                // we update frame_to_answer to indicate that we are waiting to send back this data to renderer.
                                                // self.frame_to_answer = Some(renderer_req);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // It has not been requested, so we send a request to the fetcher to fetch the data
                                _ = self.buf_in_sx.send(FetchRequest::new(renderer_req, self.buffer.len()));

                                // we update frame_to_answer to indicate that we are waiting to send back this data to renderer.
                                self.frame_to_answer = Some(renderer_req);

                                // we also update next_fetch_req so that when the fetcher returns the data, we can immediately send the next request to the fetcher
                                self.buffer.add(renderer_req);
                            }
                        }
                        BufMsg::FetchDone(req) => {
                            // upon receiving fetch result, immediately schedule the next fetch request
                            self.buffer.update_state(req, FrameStatus::Decoding);

                            if !self.buffer.is_full() {
                                // If the buffer is not full yet, we can send a request to the fetcher to fetch the next frame
                                self.prefetch_frame(req.camera_pos);
                            } else {
                                is_desired_buffer_level_reached = true;
                            }
                        }
                        BufMsg::PointCloud((mut metadata, mut rx)) => {
                            trace!("[buffer mgr] received a point cloud result {:?}", &metadata);
                            let orig_metadata = metadata.into();

                            let mut remaining = self.segment_size as usize;
                            if self.frame_to_answer.is_some()
                                && metadata.frame_offset
                                    == self.frame_to_answer.as_ref().unwrap().frame_offset
                            {
                                let pc = rx.recv().await.unwrap();
                                // send results to the renderer
                                _ = self.buf_out_sx.send((self.frame_to_answer.unwrap(), pc));
                                self.frame_to_answer = None;
                                metadata.frame_offset += 1;
                                remaining -= 1;
                            }

                            // cache the point cloud if there is still point clouds to render
                            self.buffer.update(orig_metadata, metadata.into(), FrameStatus::Ready(remaining, rx));
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

impl From<FetchRequest> for FrameRequest {
    fn from(val: FetchRequest) -> Self {
        FrameRequest {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
            camera_pos: val.camera_pos,
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

/// Returns if the source file is remote
fn is_remote_src(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
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
    let buffer_capacity = args.buffer_capacity.unwrap_or(2);
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
            if is_remote_src(&args.src) {
                let tmpdir = tempdir().expect("created temp dir to store files");
                let path = tmpdir.path();
                trace!("[fetcher] Downloading files to {}", path.to_str().unwrap());

                let mut fetcher = Fetcher::new(&src, path, args.enable_fetcher_optimizations).await;
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
                    AbrType::Quetra => Box::new(Quetra::new(buffer_capacity, args.fps)),
                    AbrType::Mckp => Box::new(MCKP::new(6, qualities)),
                    AbrType::QuetraMultiview => {
                        Box::new(QuetraMultiview::new(buffer_capacity, args.fps, 6, qualities))
                    }
                };

                loop {
                    tokio::select! {
                        _ = shutdown_recv.changed() => {
                            trace!("[fetcher] shutdown signal received");
                             _ = tmpdir.close();
                            break;
                        },
                        Some(req) = buf_in_rx.recv() => {
                            let camera_pos = req.camera_pos.expect("camera position is always provided");

                            // We start with a guess of 1Mbps network throughput.
                            let network_throughput = if simulated_network_trace.is_none() {
                                throughput_predictor.predict().unwrap_or(1_000_000.0)
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
                            info!("buffer_occupancy: {}, network: {}, cosines: {:?}", req.buffer_occupancy, network_throughput, &cosines);

                            // This is a retry loop, we should probably do *bounded* retry here instead of looping indefinitely.
                            loop {
                                trace!("[fetcher] trying request {:?}", &req);

                                let p = fetcher
                                    .download(req.object_id, req.frame_offset, &quality, args.multiview, if simulated_network_trace.is_some() { Some(network_throughput) } else { None })
                                    .await;

                                match p {
                                    Ok(res) => {
                                        // update throughput prediction
                                        throughput_predictor.add(res.throughput);
                                        // send the response to the decoder
                                        _ = in_dec_sx.send((req, res));
                                        // let buffer know that we are done fetching
                                        _ = to_buf_sx.send(BufMsg::FetchDone(req.into()));
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
                    if !f.extension().map(|f| "pcd".eq(f)).unwrap_or(false) {
                        continue;
                    }
                    ply_files.push(f);
                }
                total_frames_tx
                    .send((ply_files.len(), (1, 30)))
                    .expect("sent total frames");
                ply_files.sort();

                loop {
                    tokio::select! {
                        _ = shutdown_recv.changed() => {
                            trace!("[fetcher] shutdown signal received");
                            break;
                        },
                        Some(req) = buf_in_rx.recv() => {
                            trace!("[fetcher] got fetch request {:?}", req);
                            _ = in_dec_sx.send((req, FetchResult {
                                paths: [ply_files.get(req.frame_offset as usize).map(|p| p.to_path_buf()), None, None, None, None, None],
                                throughput: 0.0,
                            }));
                            // let buffer know that we are done fetching
                            _ = to_buf_sx.send(BufMsg::FetchDone(req.into()));
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
                            debug!("Decoding took {:?}", elapsed);
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
                CameraPosition {
                    position: Point3::new(args.camera_x, args.camera_y, args.camera_z),
                    yaw: cgmath::Deg(args.camera_yaw).into(),
                    pitch: cgmath::Deg(args.camera_pitch).into(),
                },
                simulated_camera_trace,
                record_camera_trace,
            )
            .await
    });

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
            args.bg_color.to_str().unwrap()
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
