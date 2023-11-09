use cgmath::Point3;
use clap::Parser;
use log::{debug, info, trace, warn};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use vivotk::abr::quetra::{Quetra, QuetraMultiview};
use vivotk::abr::{RateAdapter, MCKP};
use vivotk::codec::decoder::{DracoDecoder, NoopDecoder, Tmc2rsDecoder};
use vivotk::codec::Decoder;
use vivotk::dash::fetcher::{FetchResult, Fetcher};
use vivotk::dash::{ThroughputPrediction, ViewportPrediction};
use vivotk::render::wgpu::{
    builder::{EventType, RenderBuilder, RenderEvent},
    camera::{Camera, CameraPosition},
    controls::Controller,
    metrics_reader::MetricsReader,
    reader::{FrameRequest, PcdAsyncReader, RenderReader},
    renderer::Renderer,
};
use vivotk::utils::{
    get_cosines, predict_quality, ExponentialMovingAverage, LastValue, SimpleRunningAverage, GAEMA,
    LPEMA,
};
use vivotk::{BufMsg, PCMetadata};
use vivotk::vvplay_async_prefetch::network_trace::NetworkTrace;
use vivotk::vvplay_async_prefetch::camera_trace::CameraTrace;
use vivotk::vvplay_async_prefetch::fetch_request::FetchRequest;
use vivotk::vvplay_async_prefetch::buffer_manager::BufferManager;
use vivotk::vvplay_async_prefetch::args::Args;
use vivotk::vvplay_async_prefetch::args::ViewportPredictionType;
use vivotk::vvplay_async_prefetch::args::ThroughputPredictionType;
use vivotk::vvplay_async_prefetch::args::AbrType;
use vivotk::vvplay_async_prefetch::args::DecoderType;

/// Plays a folder of pcd files in lexicographical order

//t: what is this for?
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


/// Returns if the source file is remote
fn is_remote_src(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
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

//this is the main code for vvplay_async
fn main() {
    // initialize logger for trace!()
    env_logger::init();
    let args: Args = Args::parse();
    let play_format = infer_format(&args.src);
    
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
    //t: why is this not used?
    let buffer_capacity = args.buffer_capacity.unwrap_or(11);
    //t: hard coded the buffer size
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

                //t: predict quality here

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
                    if !f.extension().map(|f| play_format.as_str().eq(f)).unwrap_or(false) {
                        continue;
                    }
                    ply_files.push(f);
                }
                total_frames_tx
                    .send((ply_files.len(), (1, 30)))
                    .expect("sent total frames");
                ply_files.sort();
                //t: this is where the ply file fetch request started 
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
                        let now = std::time::Instant::now();
                        tokio::task::spawn_blocking(move || {
                            let mut decoder: Box<dyn Decoder> = match decoder_type {
                                DecoderType::Draco => { 
                                    Box::new(DracoDecoder::new(
                                    decoder_path
                                        .as_ref()
                                        .expect("must provide decoder path for Draco")
                                        .as_os_str(),
                                    paths[0].take().unwrap().as_os_str(),
                                )) },
                                DecoderType::Tmc2rs => {
                                    let paths = paths.into_iter().flatten().collect::<Vec<_>>();
                                    Box::new(Tmc2rsDecoder::new(&paths))
                                }
                                _ =>{ 
                                    Box::new(NoopDecoder::new(paths[0].take().unwrap().as_os_str()))
                                },
                            };
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
                            println!("Decoding took {:?}", elapsed);
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
    //t: store in buf_our_rx and out_buf_sx, buffer size then read using PcdAsyncReader?
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
