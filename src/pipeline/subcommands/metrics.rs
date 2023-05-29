use std::{
    ffi::OsString,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use clap::Parser;

use crate::{
    metrics::calculate_metrics,
    pipeline::{channel::Channel, PipelineMessage},
};

use super::Subcommand;

#[derive(Parser)]
struct Args {}

pub struct MetricsCalculator;

impl MetricsCalculator {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let _args: Args = Args::parse_from(args);
        Box::new(MetricsCalculator {})
    }
}

impl Subcommand for MetricsCalculator {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        let mut messages_iter = messages.into_iter();
        let message_one = messages_iter
            .next()
            .expect("Expecting two input streams for metrics");
        let message_two = messages_iter
            .next()
            .expect("Expecting two input streams for metrics");

        match (&message_one, &message_two) {
            (PipelineMessage::PointCloud(original), PipelineMessage::PointCloud(reconstructed)) => {
                let metrics = calculate_metrics(original, reconstructed);
                channel.send(PipelineMessage::Metrics(metrics));
            }
            (PipelineMessage::End, _) | (_, PipelineMessage::End) => {
                channel.send(PipelineMessage::End);
            }
            (_, _) => {}
        }
    }
}
