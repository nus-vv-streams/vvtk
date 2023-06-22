use clap::Parser;
use std::ffi::OsString;

use crate::{
    metrics::calculate_metrics,
    pipeline::{channel::Channel, PipelineMessage},
};

use super::Subcommand;

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum SupoportedMetrics {
    Acd,
    Cd,
    CdPsnr,
}

#[derive(Parser)]
#[clap(
    about = "Calculates the metrics given two input streams.\nFirst input stream is the original.\nSecond is the reconstructed.\nThen uses write command to write the metrics into a text file.",
    override_usage = format!("\x1B[1m{}\x1B[0m [OPTIONS] +input=original,reconstructure +output=metrics", "metrics")
)]
pub struct Args {
    #[clap(short, long, value_enum, default_value_t = SupoportedMetrics::CdPsnr)]
    metric: SupoportedMetrics,

    #[clap(long, num_args = 1.., value_delimiter = ' ', default_value = "all")]
    metrics: Vec<OsString>,
}

pub struct MetricsCalculator {
    metrics: Vec<OsString>,
}

impl MetricsCalculator {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        println!("args: {:?}", args);
        let args: Args = Args::parse_from(args);
        let metrics = args.metrics;
        println!("metrics: {:?}", metrics);

        Box::new(MetricsCalculator { metrics })
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
            (
                PipelineMessage::IndexedPointCloud(original, _),
                PipelineMessage::IndexedPointCloud(reconstructed, _),
            ) => {
                let metrics = calculate_metrics(original, reconstructed, &self.metrics);
                channel.send(PipelineMessage::Metrics(metrics));
            }
            (PipelineMessage::End, _) | (_, PipelineMessage::End) => {
                channel.send(PipelineMessage::End);
                // println!("Get `End` message, Closing metrics calculator channel");
            }
            (_, _) => {}
        }
    }
}
