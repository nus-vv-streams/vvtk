use clap::Parser;

use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::reconstruct::poisson_reconstruct::reconstruct;

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
                PipelineMessage::IndexedPointCloud(pc, i, _) => {
                    let (reconstructed_pc, triangle_faces) = reconstruct(pc);
                    channel.send(PipelineMessage::IndexedPointCloud(
                        reconstructed_pc,
                        i,
                        Some(triangle_faces),
                    ));
                }
                PipelineMessage::Metrics(_) | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
