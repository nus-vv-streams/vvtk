use clap::Parser;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::reconstruct::poisson_reconstruct::reconstruct;
use std::time::Instant;

#[derive(Parser)]
pub struct Args {
    //Future implementation
}

pub struct Reconstructer {}

impl Reconstructer {
    pub fn from_args(_args: Vec<String>) -> Box<dyn Subcommand> {
        Box::new(Reconstructer {})
    }
}

impl Subcommand for Reconstructer {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let start = Instant::now();
                    let (reconstructed_pc, triangle_faces) = reconstruct(pc);
                    let duration = start.elapsed();
                    println!("Time elapsed in expensive_function() is: {:?}", duration);
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
