use clap::Parser;

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
                    let downsampled_pc = downsample(pc, self.points_per_voxel);
                    channel.send(PipelineMessage::IndexedPointCloud(downsampled_pc, i));
                }
                PipelineMessage::SubcommandMessage(subcommand_object, i) => {
                    // Only vv extend will send SubcommandMessage, other subcommand will send IndexedPointCloud to make sure the other command will
                    // continue to be compatible by receiving IndexedPointCloud
                    let downsampled_pc = downsample(
                        subcommand_object.get_content().clone(),
                        self.points_per_voxel,
                    );
                    channel.send(PipelineMessage::IndexedPointCloud(downsampled_pc, i));
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::IndexedPointCloudNormal(_, _)
                | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
