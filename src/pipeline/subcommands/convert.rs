use std::ffi::OsString;
use std::str::FromStr;
use std::path::{Path, PathBuf};
use kdam::tqdm;
use clap::Parser;
use crate::pcd::PCDDataType;

use crate::pipeline::Subcommand;
use crate::pipeline::PipelineMessage;
use crate::pipeline::channel::Channel;

use crate::utils::{find_all_files, pcd_to_pcd, pcd_to_ply, ply_to_pcd, ply_to_ply};
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ConvertOutputFormat {
    PLY,
    PCD,
    PNG,
    MP4,
}

impl ToString for ConvertOutputFormat {
    fn to_string(&self) -> String {
        match self {
            ConvertOutputFormat::PLY => "ply",
            ConvertOutputFormat::PCD => "pcd",
            ConvertOutputFormat::PNG => "png",
            ConvertOutputFormat::MP4 => "mp4",
        }
        .to_string()
    }
}

impl FromStr for ConvertOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ply" => Ok(ConvertOutputFormat::PLY),
            "pcd" => Ok(ConvertOutputFormat::PCD),
            "png" => Ok(ConvertOutputFormat::PNG),
            "mp4" => Ok(ConvertOutputFormat::MP4),
            _ => Err(format!("{} is not a valid output format", s)),
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    output: String,

    #[clap(long, default_value = "pcd")]
    output_format: ConvertOutputFormat, 

    #[clap(short, long, default_value = "binary")]
    storage_type: PCDDataType,

    #[clap(short, long)]
    input: Vec<OsString>,
}

pub struct Convert {
    args: Args,
}

impl Convert {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Convert {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for Convert {
    fn handle(
        &mut self,
        messages: Vec<PipelineMessage>,
        channel: &Channel,
    ) {
        if messages.is_empty() {
            println!("Start converting...");
            let mut files = find_all_files(&self.args.input);
            files.sort();

            // create output dir
            let output_path = Path::new(&self.args.output);
            std::fs::create_dir_all(output_path).expect("Failed to create output directory");

            for file in tqdm!(files.into_iter()) {
                let current_file_type = file.extension().unwrap();
                let target_file_type = self.args.output_format.to_string();

                match (current_file_type.to_str().unwrap(), target_file_type.as_str()) {
                    ("ply", "ply") => ply_to_ply(output_path, self.args.storage_type, file),
                    ("ply", "pcd") => ply_to_pcd(output_path, self.args.storage_type, file),
                    ("pcd", "ply") => pcd_to_ply(output_path, self.args.storage_type, file),
                    ("pcd", "pcd") => pcd_to_pcd(output_path, self.args.storage_type, file),
                    _ => println!("unsupported file type"),
                }

            } 

            channel.send(PipelineMessage::End);
        }
        else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}

