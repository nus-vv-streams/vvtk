use clap::Parser;

use crate::reconstruct::poisson_reconstruct::reconstruct;
use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;

#[derive(Parser)]
pub struct Args {
    //Future implementation
}

pub struct Reconstructer {

}

impl Reconstructer {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::new(Reconstructer {
        })
    }
}

impl Subcommand for Reconstructer {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let reconstructed_pc = reconstruct(pc);
                    channel.send(PipelineMessage::IndexedPointCloud(reconstructed_pc, i));
                }
                PipelineMessage::Metrics(_) | PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}