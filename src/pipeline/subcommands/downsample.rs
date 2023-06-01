use clap::Parser;

use crate::{
    downsample::octree::downsample,
    pipeline::{channel::Channel, PipelineMessage},
};

use super::Subcommand;

#[derive(Parser)]
struct Args {
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
                PipelineMessage::PointCloud(pc) => {
                    let downsampled_pc = downsample(pc, self.points_per_voxel);
                    channel.send(PipelineMessage::PointCloud(downsampled_pc));
                }
                PipelineMessage::Metrics(_) | PipelineMessage::DummyForIncrement=> {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
