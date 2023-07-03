use clap::Parser;
use std::ffi::OsString;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::utils::{find_all_files, read_file_to_point_cloud};

#[derive(clap::ValueEnum, Clone, Copy)]
enum FileType {
    All,
    Ply,
    Pcd,
    Bin,
}

#[derive(Parser)]
#[clap(
    about = "Reads in one of our supported file formats. \nFiles can be of the type .pcd .ply. \nThe path can be a file path or a directory path contains these files.",
    override_usage = format!("\x1B[1m{}\x1B[0m [OPTIONS] <FILES>... +output=plys", "read")
)]
pub struct Args {
    #[clap(short = 't', long, value_enum, default_value_t = FileType::All)]
    filetype: FileType,
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
        println!("Reading files");
        if messages.is_empty() {
            let mut files = find_all_files(&self.args.files);
            files.sort();
            for (i, file) in files.iter().enumerate() {
                match &self.args.filetype {
                    FileType::All => {}
                    FileType::Pcd => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("pcd") {
                            continue;
                        }
                    }
                    FileType::Ply => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("ply") {
                            continue;
                        }
                    }
                    FileType::Bin => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("bin") {
                            continue;
                        }
                    }
                }

                let point_cloud = read_file_to_point_cloud(file);
                if let Some(pc) = point_cloud {
                    channel.send(PipelineMessage::IndexedPointCloud(pc, i as u32));
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
