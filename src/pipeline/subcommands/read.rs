use std::ffi::OsString;
use std::sync::mpsc::Sender;

use clap::Parser;

use super::Subcommand;
use crate::pipeline::{PipelineMessage, Progress};
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
    fn handle(
        &mut self,
        message: PipelineMessage,
        out: &Sender<PipelineMessage>,
        progress: &Sender<Progress>,
    ) {
        if let PipelineMessage::End = message {
            let mut files = find_all_files(&self.args.files);
            progress
                .send(Progress::Length(files.len()))
                .expect("should be able to send");
            files.sort();
            for file in files {
                let point_cloud = read_file_to_point_cloud(&file);
                if let Some(pc) = point_cloud {
                    out.send(PipelineMessage::PointCloud(pc))
                        .expect("should be able to send");
                }
                progress
                    .send(Progress::Incr)
                    .expect("should be able to send");
            }
            progress
                .send(Progress::Completed)
                .expect("should be able to send");
            out.send(PipelineMessage::End)
                .expect("should be able to send");
        } else {
            out.send(message).expect("should be able to send");
        }
    }
}
