use std::ffi::OsString;

use clap::Parser;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::utils::{find_all_files, read_file_to_point_cloud};

#[derive(Parser)]
struct Args {
    /// Files, glob patterns, directories
    files: Vec<OsString>,
}

pub struct Read {
    args: Args,
}

impl Read {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Read {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for Read {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        if messages.is_empty() {
            let mut files = find_all_files(&self.args.files);
            files.sort();
            for file in files {
                let point_cloud = read_file_to_point_cloud(&file);
                if let Some(pc) = point_cloud {
                    channel.send(PipelineMessage::PointCloud(pc));
                }
            }
            channel.send(PipelineMessage::End);
        } else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}
