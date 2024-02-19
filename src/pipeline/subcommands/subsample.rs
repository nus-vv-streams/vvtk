use clap::Parser;

use crate::{
    pipeline::{channel::Channel, PipelineMessage},
    subsample::random_sampler::subsample,
};

use super::Subcommand;

/// Subsamples a pointcloud into a list of pointclouds with different number of points.
#[derive(Parser)]
pub struct Args {
    #[clap(
        short = 'p',
        long = "proportions",
        num_args = 1..,
        value_delimiter = ',',
        help = "Set the proportions as a list of usize values"
    )]
    proportions: Vec<usize>,
    #[clap(
        short = 't',
        long = "threshold",
        help = "points per voxel threshold",
        default_value = "20"
    )]
    points_per_voxel_threshold: usize,
}

pub struct Subsampler {
    proportions: Vec<usize>,
    points_per_voxel_threshold: usize,
}

impl Subsampler {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Subsampler {
            proportions: args.proportions,
            points_per_voxel_threshold: args.points_per_voxel_threshold,
        })
    }
}

impl Subcommand for Subsampler {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let point_clouds = subsample(
                        &pc,
                        self.proportions.clone(),
                        self.points_per_voxel_threshold,
                    );
                    for (resolution, pc) in point_clouds.into_iter().enumerate() {
                        channel.send(PipelineMessage::IndexedPointCloudWithResolution(
                            pc,
                            i,
                            resolution as u32,
                        ));
                    }
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::IndexedPointCloudWithResolution(_, _, _)
                | PipelineMessage::IndexedPointCloudNormal(_, _)
                | PipelineMessage::ManifestInformation(_, _, _, _)
                | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
