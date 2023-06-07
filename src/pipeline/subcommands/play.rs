use clap::Parser;
use std::ffi::OsString;
use std::path::Path;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;

#[derive(Parser, Debug)]
pub struct Args {
    /// Directory with all the pcd files in lexicographical order
    directory: String,
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
    #[clap(long, default_value_t = 900)]
    height: u32,
    #[clap(long = "controls")]
    show_controls: bool,
    #[clap(short, long, default_value_t = 1)]
    buffer_size: usize,
    #[clap(short, long)]
    metrics: Option<OsString>,
    #[clap(long, default_value = "infer")]
    play_format: String,
}

pub struct Play {
    args: Args,
}

impl Play {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Play {
            args: Args::parse_from(args),
        })
    }

    fn infer_format(path: &str, play_format: &str) -> String {
        if play_format.eq("pcd") || play_format.eq("ply") {
            return play_format.to_string();
        }

        let path = Path::new(path);
        // infer by counting extension numbers (pcd count and ply count)
        // if pcd count > ply count, then pcd
        let mut pcd_count = 0;
        let mut ply_count = 0;
        for file_entry in path.read_dir().unwrap() {
            match file_entry {
                Ok(entry) => {
                    if let Some(ext) = entry.path().extension() {
                        if ext.eq("pcd") {
                            pcd_count += 1;
                        } else if ext.eq("ply") {
                            ply_count += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{e}")
                }
            }
        }
        if pcd_count > ply_count {
            "pcd".to_string()
        } else {
            "ply".to_string()
        }
    }

    // TODO
    fn render_point_cloud() {}
}

impl Subcommand for Play {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        // !! if no messages, then read from directory and render it to screen
        if messages.is_empty() {
            let play_format = Play::infer_format(&self.args.directory, &self.args.play_format);
            println!("Playing files in {} with format {}", self.args.directory, play_format);
            
        } else {
            // !! also accept messages(PipelineMessage::PointCloud) from other subcommands
            // !! and then render it to screen
        }
        channel.send(PipelineMessage::End);
    }

}

