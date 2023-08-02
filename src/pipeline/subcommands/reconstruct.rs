use clap::Parser;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::reconstruct::poisson_reconstruct::reconstruct;
use std::time::Instant;

#[derive(Parser)]
#[clap(
    about = "Performs poisson reconstruction or screened poisson reconstruction. Point normals must be included in input stream"
)]
pub struct Args {
    #[clap(short, long, default_value_t = 0.0)]
    screening: f64,
    #[clap(short, long, default_value_t = 6)]
    density_estimation_depth: usize,
    #[clap(long, default_value_t = 6)]
    max_depth: usize,
    #[clap(long, default_value_t = 10)]
    max_relaxation_iters: usize,
}

pub struct Reconstructer {
    screening: f64,
    density_estimation_depth: usize,
    max_depth: usize,
    max_relaxation_iters: usize,
}

impl Reconstructer {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Reconstructer {
            screening: args.screening,
            density_estimation_depth: args.density_estimation_depth,
            max_depth: args.max_depth,
            max_relaxation_iters: args.max_relaxation_iters,
        })
    }
}

impl Subcommand for Reconstructer {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let start = Instant::now();
                    let (reconstructed_pc, triangle_faces) = reconstruct(
                        pc,
                        self.screening,
                        self.density_estimation_depth,
                        self.max_depth,
                        self.max_relaxation_iters,
                        true, // TODO add pipeline arg
                    );
                    let duration = start.elapsed();
                    println!(
                        "Time elapsed in whole poisson reconstruct is: {:?}",
                        duration
                    );
                    channel.send(PipelineMessage::IndexedPointCloudWithTriangleFaces(
                        reconstructed_pc,
                        i,
                        Some(triangle_faces),
                    ));
                }
                PipelineMessage::Metrics(_)
                | PipelineMessage::DummyForIncrement
                | PipelineMessage::IndexedPointCloudWithTriangleFaces(_, _, _) => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
