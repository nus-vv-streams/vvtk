use clap::Parser;

use crate::{
    pipeline::{channel::Channel, PipelineMessage},
    upsample::interpolate::upsample,
};

use super::Subcommand;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    factor: usize,
}

pub struct Upsampler {
    factor: usize,
}

impl Upsampler {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        Box::new(Upsampler {
            factor: args.factor,
        })
    }
}

impl Subcommand for Upsampler {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        for message in messages {
            match message {
                PipelineMessage::PointCloud(pc) => {
                    let upsampled_pc = upsample(pc, self.factor);
                    channel.send(PipelineMessage::PointCloud(upsampled_pc));
                }
                PipelineMessage::Metrics(_) => {}
                PipelineMessage::End => {
                    channel.send(message);
                }
            };
        }
    }
}
