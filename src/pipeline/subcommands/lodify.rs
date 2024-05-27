use clap::Parser;

use crate::{
    lodify::lodify::lodify,
    pipeline::{channel::Channel, PipelineMessage},
    utils::get_pc_bound,
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
        short = 'b',
        long = "base-proportion",
        default_value = "30",
        help = "Set the proportion of points of the base point cloud. Should lie between 0 and 100."
    )]
    base_proportion: usize,
    #[clap(
        short = 't',
        long = "threshold",
        help = "points per voxel threshold",
        default_value = "10"
    )]
    points_per_voxel_threshold: usize,
}

pub struct Lodifier {
    partitions: (usize, usize, usize),
    base_proportion: usize,
    points_per_voxel_threshold: usize,
}

impl Lodifier {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Lodifier {
            partitions: (args.x_partition, args.y_partition, args.z_partition),
            base_proportion: args.base_proportion,
            points_per_voxel_threshold: args.points_per_voxel_threshold,
        })
    }
}

impl Subcommand for Lodifier {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let (base_pc, pc_by_segment, base_point_nums, additional_point_nums) = lodify(
                        &pc,
                        self.partitions,
                        self.base_proportion,
                        self.points_per_voxel_threshold,
                    );

                    channel.send(PipelineMessage::IndexedPointCloudWithName(
                        base_pc.clone(),
                        i,
                        "base".to_string(),
                        true,
                    ));

                    for (segment, pc) in pc_by_segment.into_iter().enumerate() {
                        channel.send(PipelineMessage::IndexedPointCloudWithName(
                            pc,
                            i,
                            format!("{}", segment),
                            false, // don't need headers for additional point clouds
                        ));
                    }

                    let bound = get_pc_bound(&pc);

                    channel.send(PipelineMessage::MetaData(
                        bound,
                        base_point_nums,
                        additional_point_nums,
                        self.partitions,
                    ));
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::IndexedPointCloudWithName(_, _, _, _)
                | PipelineMessage::IndexedPointCloudNormal(_, _)
                | PipelineMessage::MetaData(_, _, _, _)
                | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::SubcommandMessage(_, _) => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
