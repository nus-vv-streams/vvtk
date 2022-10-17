use crate::pipeline::channel::Channel;
use crate::pipeline::{PipelineMessage, Progress};
use crate::render::wgpu::png::PngWriter;
use clap::Parser;
use std::ffi::OsString;
use std::sync::mpsc::Sender;

use super::Subcommand;

/// Converts a folder of .pcd files to a folder of .png images
#[derive(Parser)]
struct Args {
    /// Directory to store output png images
    #[clap(short, long)]
    output_dir: OsString,
    /// Number of pcd files to convert
    #[clap(short = 'n', long)]
    frames: Option<usize>,
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
}

pub struct ToPng<'a> {
    writer: PngWriter<'a>,
}

impl<'a> ToPng<'a> {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let Args {
            output_dir,
            frames,
            camera_x,
            camera_y,
            camera_z,
            camera_yaw,
            camera_pitch,
            width,
            height,
        }: Args = Args::parse_from(args);

        Box::from(ToPng {
            writer: PngWriter::new(
                output_dir,
                frames,
                camera_x,
                camera_y,
                camera_z,
                camera_yaw,
                camera_pitch,
                width,
                height,
            ),
        })
    }
}

impl Subcommand for ToPng<'_> {
    fn handle(
        &mut self,
        messages: Vec<PipelineMessage>,
        channel: &Channel,
        progress: &Sender<Progress>,
    ) {
        for message in messages {
            match &message {
                PipelineMessage::PointCloud(pc) => {
                    self.writer.write_to_png(pc);
                    progress.send(Progress::Incr);
                }
                PipelineMessage::End => {
                    progress.send(Progress::Completed);
                }
            }
            channel.send(message);
        }
    }
}
