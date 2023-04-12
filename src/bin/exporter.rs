//! This binary is used to export pngs from a video playback session.
//!
//! The camera trace file is a csv file that contains the camera position and orientation for each frame.
//! The format of the csv file is (x, y, z, pitch, yaw, roll):
//! ```csv
//! 0,0,1.5,0,-90,0.0
//! 0,0,1.5,0,-90,0.0
//! ```
//!
//! The quality trace file is a csv file that contains the quality for each frame.
//! The format of the csv file is qualities for each views (view_0, view_1, view_2, view_3, view_4, view_5, view_6).
//! You can also provide only 1 quality. In this case, this number will be the quality for all views.
//! ```csv
//! 0,0,0,5,5,5
//! 3
//! ```
//!
//! # Usage
//!
//! For remote files
//! ```bash
//! cargo run --bin exporter --release -- --url http://localhost:3000/longdress.mpd --quality quality_trace.csv [camera_trace.csv] [output_folder]
//! ```
//!
//! For local files
//! ```bash
//! cargo run --bin exporter --release -- --ply_folder [ply_folder] [camera_trace.csv] [output_folder]
//! ```
//!

use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use vivotk::codec::{decoder::Tmc2rsDecoder, Decoder};
use vivotk::dash::fetcher::Fetcher;
use vivotk::render::wgpu::png::PngWriter;
use vivotk::simulation::CameraTrace;
use vivotk::utils::read_file_to_point_cloud;

#[derive(Parser)]
struct Args {
    /// Camera trace file
    camera_trace: PathBuf,
    /// Output folder for pngs
    output_folder: PathBuf,
    /// Quality trace file
    #[clap(long)]
    quality: Option<PathBuf>,
    /// Remote url that points to MPD file
    #[clap(long)]
    url: Option<String>,
    /// Folder containing original ply files
    #[clap(long)]
    ply_folder: Option<PathBuf>,
    /// Total number of frames in original video
    #[clap(long, default_value_t = 300)]
    frame_count: usize,
    /// number of frames in each segment. By default, it is set to 30 frames (1 second)
    #[clap(long, default_value_t = 30)]
    segment_size: usize,
    /// Set screen width.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short, long, default_value_t = 1600)]
    width: u32,
    /// Set screen height.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short, long, default_value_t = 900)]
    height: u32,
}

struct QualityTrace {
    data: Vec<Vec<usize>>,
}

impl QualityTrace {
    fn new(path: &Path) -> Self {
        use std::io::BufRead;

        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data = reader
            .lines()
            .map(|line| {
                line.unwrap()
                    .trim()
                    .split(',')
                    .map(|x| x.parse::<usize>().unwrap())
                    .collect()
            })
            .collect();
        QualityTrace { data }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let args: Args = Args::parse();
    let mut png_writer = PngWriter::new(
        args.output_folder.clone().into_os_string(),
        0.0,
        0.0,
        0.0,
        cgmath::Deg(0.0).into(),
        cgmath::Deg(0.0).into(),
        args.width,
        args.height,
    );
    png_writer.set_background_color(wgpu::Color {
        r: 0.0,
        g: 0.44,
        b: 0.09,
        a: 1.0,
    });
    let camera_trace = CameraTrace::new(&args.camera_trace, false);

    if let Some(quality) = args.quality {
        let quality_trace = QualityTrace::new(&quality);
        let mut fetcher = Fetcher::new(
            &args.url.expect("url must be provided"),
            &args.output_folder,
            true,
        )
        .await;
        let mut frame_number = 0;
        for quality in quality_trace.data.iter() {
            let res = fetcher
                .download(
                    0,
                    frame_number % args.frame_count as u64,
                    quality,
                    true,
                    None,
                )
                .await
                .unwrap();
            let paths = res.paths.into_iter().flatten().collect::<Vec<_>>();
            let mut decoder = Tmc2rsDecoder::new(&paths);
            decoder.start().unwrap();
            for _ in 0..args.segment_size {
                let cam_pos = camera_trace.next();
                png_writer.update_camera_pos(cam_pos);
                let pc = decoder.poll().unwrap();
                png_writer.write_to_png(&pc);
            }
            frame_number += 30;
        }
    } else if let Some(ply_folder) = args.ply_folder {
        let mut ply_files: Vec<PathBuf> = vec![];

        let mut dir = tokio::fs::read_dir(ply_folder).await.unwrap();
        while let Some(entry) = dir.next_entry().await.unwrap() {
            let f = entry.path();
            if !f.extension().map(|f| "ply".eq(f)).unwrap_or(false) {
                continue;
            }
            ply_files.push(f);
        }

        ply_files.sort();
        for frame_number in 0..64 * args.segment_size {
            let cam_pos = camera_trace.next();
            png_writer.update_camera_pos(cam_pos);
            let pc =
                read_file_to_point_cloud(ply_files.get(frame_number % args.frame_count).unwrap())
                    .unwrap();
            png_writer.write_to_png(&pc);
        }
    } else {
        unreachable!("Either quality or ply_folder must be specified")
    }
}
