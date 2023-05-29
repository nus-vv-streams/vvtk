use std::ffi::OsString;
use std::sync::mpsc::Sender;
use std::str::FromStr;
use super::super::super::pcd::PCDDataType;
use clap::Parser;
use super::Subcommand;
use crate::pipeline::{PipelineMessage, Progress};
use crate::pipeline::channel::Channel;
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
            println!("Message is empty");
        }
        else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}