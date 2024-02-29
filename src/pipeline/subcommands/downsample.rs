use clap::Parser;
use std::time::Instant;

use crate::{
    downsample::octree::downsample,
    pipeline::{channel::Channel, PipelineMessage},
};

use super::Subcommand;

/// Downsample a pointcloud from the stream.
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    points_per_voxel: usize,
}

pub struct Downsampler {
    points_per_voxel: usize,
}

impl Downsampler {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Downsampler {
            points_per_voxel: args.points_per_voxel,
        })
    }
}

impl Subcommand for Downsampler {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let now = Instant::now();
                    let downsampled_pc = downsample(pc, self.points_per_voxel);
                    let elapsed = now.elapsed();
                    //println!("Elapsed for vv native downsample: {:.2?}", elapsed);
                    //println!("result pc is {:?}", downsampled_pc);
                    println!("{:2?}", elapsed);
                    channel.send(PipelineMessage::IndexedPointCloud(downsampled_pc, i));
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::IndexedPointCloudNormal(_, _)
                | PipelineMessage::DummyForIncrement 
                | PipelineMessage::SubcommandMessage(_, _, _) => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
