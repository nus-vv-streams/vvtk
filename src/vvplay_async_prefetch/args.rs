use clap::Parser;
use std::ffi::OsString;
use std::path:: PathBuf;


use crate::vvplay_async_prefetch::enums::AbrType;
use crate::vvplay_async_prefetch::enums::DecoderType;
use crate::vvplay_async_prefetch::enums::ThroughputPredictionType;
use crate::vvplay_async_prefetch::enums::ViewportPredictionType;
/**
 * This file contains all the command line argumentfor vvplay_async_prefetch.rs
 */


#[derive(Parser)]
pub struct Args {
    /// src can be:
    ///
    /// 1. Directory with all the ply files
    /// 2. location of the mpd url (dash)
    pub src: String,
    #[clap(short, long, default_value_t = 30.0)]
    pub fps: f32,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    pub camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    pub camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.5)]
    pub camera_z: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    pub camera_pitch: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    pub camera_yaw: f32,
    /// Set the screen width.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short = 'W', long, default_value_t = 1600)]
    pub width: u32,
    /// Set the screen height.
    ///
    /// To enable rendering at full screen, compile with `--features fullscreen` (depends on device gpu support)
    #[clap(short = 'H', long, default_value_t = 900)]
    pub height: u32,
    #[clap(long = "controls", action = clap::ArgAction::SetTrue, default_value_t = true)]
    pub show_controls: bool,
    /// buffer capacity in seconds
    #[clap(short, long)]
    pub buffer_capacity: Option<u64>,
    #[clap(short, long)]
    pub metrics: Option<OsString>,
    #[clap(long = "abr", value_enum, default_value_t = AbrType::Quetra)]
    pub abr_type: AbrType,
    #[clap(long = "decoder", value_enum, default_value_t = DecoderType::Noop)]
    pub decoder_type: DecoderType,
    /// Set this flag if each view is encoded separately, i.e. multiview
    #[clap(long, action = clap::ArgAction::SetTrue)]
    pub multiview: bool,
    /// Path to the decoder binary (only for Draco)
    #[clap(long)]
    pub decoder_path: Option<PathBuf>,
    #[clap(long = "tp", value_enum, default_value_t = ThroughputPredictionType::Last)]
    pub throughput_prediction_type: ThroughputPredictionType,
    /// Alpha for throughput prediction. Only used for EMA, GAEMA, and LPEMA
    #[clap(long, default_value_t = 0.1)]
    pub throughput_alpha: f64,
    #[clap(long = "vp", value_enum, default_value_t = ViewportPredictionType::Last)]
    pub viewport_prediction_type: ViewportPredictionType,
    /// Path to network trace for repeatable simulation. Network trace is expected to be given in Kbps
    #[clap(long)]
    pub network_trace: Option<PathBuf>,
    /// Path to camera trace for repeatable simulation. Camera trace is expected to be given in (pos_x, pos_y, pos_z, rot_pitch, rot_yaw, rot_roll).
    /// Rotation is in degrees
    #[clap(long)]
    pub camera_trace: Option<PathBuf>,
    /// Path to record camera trace from the player.
    #[clap(long)]
    pub record_camera_trace: Option<PathBuf>,
    /// Enable fetcher optimizations
    ///
    /// 1. Not fetching when file has been previously downloaded.
    #[clap(long, action = clap::ArgAction::SetTrue)]
    pub enable_fetcher_optimizations: bool,
    #[clap(long, default_value = "rgb(255,255,255)")]
    pub bg_color: OsString,
}
