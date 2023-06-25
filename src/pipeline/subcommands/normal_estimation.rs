use clap::Parser;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "Performs normal estimation on point clouds.",
)]
pub struct Args {
    
}

pub struct NormalEstimation {
    args: Args,
}

impl NormalEstimation {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(NormalEstimation {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for NormalEstimation {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        // Implementations goes here
    }
}
