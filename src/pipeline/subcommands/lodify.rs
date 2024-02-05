use clap::Parser;

use crate::{
    lodify::lodify::{lodify, partition},
    pipeline::{channel::Channel, PipelineMessage},
    utils::{get_pc_bound, weighted_centroid},
};

use super::Subcommand;

/// Partition and LODifies a pointcloud into pointclouds with different resolutions.
#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value = "2")]
    x_partition: usize,
    #[clap(short, long, default_value = "2")]
    y_partition: usize,
    #[clap(short, long, default_value = "2")]
    z_partition: usize,
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

pub struct Lodifier {
    partitions: (usize, usize, usize),
    proportions: Vec<usize>,
    points_per_voxel_threshold: usize,
}

impl Lodifier {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Lodifier {
            partitions: (args.x_partition, args.y_partition, args.z_partition),
            proportions: args.proportions,
            points_per_voxel_threshold: args.points_per_voxel_threshold,
        })
    }
}

impl Subcommand for Lodifier {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let point_clouds = lodify(
                        &pc,
                        self.partitions,
                        self.proportions.clone(),
                        self.points_per_voxel_threshold,
                    );

                    let base_pc = point_clouds.first().unwrap().clone();

                    for (resolution, pc) in point_clouds.into_iter().enumerate() {
                        channel.send(PipelineMessage::IndexedPointCloudWithResolution(
                            pc,
                            i,
                            resolution as u32,
                        ));
                    }

                    let bounds = get_pc_bound(&pc).partition(self.partitions);
                    let partitioned_base_pc = partition(&base_pc, self.partitions);
                    let centroids = partitioned_base_pc
                        .segments
                        .iter()
                        .map(|points| weighted_centroid(&points.points))
                        .collect();

                    channel.send(PipelineMessage::ManifestInformation(
                        bounds,
                        centroids,
                        self.proportions.len(),
                        self.partitions,
                    ));
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
