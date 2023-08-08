use clap::ArgAction;
use clap::Parser;
use std::time::Instant;

use crate::{
    pipeline::{channel::Channel, PipelineMessage},
    upsample::{interpolate::upsample, upsample_methods::UpsampleMethod},
    reconstruct::poisson_reconstruct::reconstruct,
};

use super::Subcommand;

/// Upsamples a pointcloud from the stream.
#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value_t = 0)]
    factor: usize,
    #[clap(short, long, default_value = "default")]
    method: UpsampleMethod,
    #[clap(short, long, default_value_t = 0.0)]
    screening: f64,
    #[clap(short, long, default_value_t = 6)]
    density_estimation_depth: usize,
    #[clap(long, default_value_t = 6)]
    max_depth: usize,
    #[clap(long, default_value_t = 10)]
    max_relaxation_iters: usize,
    #[clap(long, short, action=ArgAction::SetFalse)]
    colour: bool,
    #[clap(long, short, action=ArgAction::SetTrue)]
    faces: bool,
}

pub struct Upsampler {
    factor: usize,
}

pub struct Reconstructer {
    screening: f64,
    density_estimation_depth: usize,
    max_depth: usize,
    max_relaxation_iters: usize,
    with_colour: bool,
    with_faces: bool,
}

impl Upsampler {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        match args.method {
            UpsampleMethod::Default => {
                Box::new(Upsampler {
                    factor: args.factor,
                })
            }
            UpsampleMethod::Spsr => {
                Box::new(Reconstructer {
                    screening: args.screening,
                    density_estimation_depth: args.density_estimation_depth,
                    max_depth: args.max_depth,
                    max_relaxation_iters: args.max_relaxation_iters,
                    with_colour: args.colour,
                    with_faces: args.faces,
                })
            }
        }
    }
}

impl Subcommand for Upsampler {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    println!("Doing upsample");
                    let upsampled_pc = upsample(pc, self.factor);
                    channel.send(PipelineMessage::IndexedPointCloud(upsampled_pc, i));
                }
                PipelineMessage::Metrics(_) | PipelineMessage::IndexedPointCloudNormal(_, _) | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
                PipelineMessage::DummyForIncrement
                | PipelineMessage::IndexedPointCloudWithTriangleFaces(_, _, _)
                | PipelineMessage::IndexedPointCloudNormal(_, _)=> {}
            };
        }
    }
}

impl Subcommand for Reconstructer {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let start = Instant::now();
                    println!("Doing psr");
                    let (reconstructed_pc, triangle_faces) = reconstruct(
                        pc,
                        self.screening,
                        self.density_estimation_depth,
                        self.max_depth,
                        self.max_relaxation_iters,
                        self.with_colour,
                        self.with_faces,
                    );
                    let duration = start.elapsed();
                    println!(
                        "Time elapsed in whole poisson reconstruct is: {:?}",
                        duration
                    );
                    channel.send(PipelineMessage::IndexedPointCloudWithTriangleFaces(
                        reconstructed_pc,
                        i,
                        triangle_faces,
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
