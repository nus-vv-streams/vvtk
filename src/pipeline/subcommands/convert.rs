use crate::pcd::PCDDataType;
use clap::Parser;
use kdam::tqdm;
use std::ffi::OsString;
use std::path::Path;

use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::pipeline::Subcommand;

use crate::utils::{
    find_all_files, pcd_to_pcd, pcd_to_ply, ply_to_pcd, ply_to_ply, velodyne_bin_to_pcd,
    velodyne_bin_to_ply, ConvertOutputFormat,
};

#[derive(Parser, Debug)]
#[clap(
    about = "Converts a pointcloud file from one format to another.\nSupported formats are .pcd and .ply.\nSupported storage types are binary and ascii."
)]
pub struct Args {
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
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        if messages.is_empty() {
            // println!("Start converting...");
            let mut files = find_all_files(&self.args.input);
            files.sort();

            // create output dir
            let output_path = Path::new(&self.args.output);
            std::fs::create_dir_all(output_path).expect("Failed to create output directory");

            for file in tqdm!(files.into_iter()) {
                let current_file_type = file.extension().unwrap();
                let target_file_type = self.args.output_format.to_string();

                match (
                    current_file_type.to_str().unwrap(),
                    target_file_type.as_str(),
                ) {
                    ("ply", "ply") => ply_to_ply(output_path, self.args.storage_type, file),
                    ("ply", "pcd") => ply_to_pcd(output_path, self.args.storage_type, file),
                    ("pcd", "ply") => pcd_to_ply(output_path, self.args.storage_type, file),
                    ("pcd", "pcd") => pcd_to_pcd(output_path, self.args.storage_type, file),
                    ("bin", "ply") => {
                        velodyne_bin_to_ply(output_path, self.args.storage_type, file)
                    }
                    ("bin", "pcd") => {
                        velodyne_bin_to_pcd(output_path, self.args.storage_type, file)
                    }
                    _ => eprintln!("unsupported file type"),
                }

                channel.send(PipelineMessage::DummyForIncrement);
            }

            channel.send(PipelineMessage::End);
        } else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}
