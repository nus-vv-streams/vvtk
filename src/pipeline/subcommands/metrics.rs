use clap::Parser;

use crate::{
    metrics::{calculate_metrics, SupoportedMetrics},
    pipeline::{channel::Channel, PipelineMessage},
};

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "Calculates the metrics given two input streams.\nFirst input stream is the original.\nSecond is the reconstructed.\nThen uses write command to write the metrics into a text file.",
    override_usage = format!("\x1B[1m{}\x1B[0m [OPTIONS] +input=original,reconstructure +output=metrics", "metrics")
)]
pub struct Args {
    #[clap(short, long, num_args = 1.., value_delimiter = ',', default_value = "all")]
    metrics: Vec<SupoportedMetrics>,
}

pub struct MetricsCalculator {
    metrics: Vec<SupoportedMetrics>,
}

impl MetricsCalculator {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        let metrics = args.metrics;
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
                PipelineMessage::IndexedPointCloud(original, _, _),
                PipelineMessage::IndexedPointCloud(reconstructed, _, _),
            ) => {
                let metrics = calculate_metrics(original, reconstructed, &self.metrics);
                channel.send(PipelineMessage::Metrics(metrics));
            }
            (PipelineMessage::End, _) | (_, PipelineMessage::End) => {
                channel.send(PipelineMessage::End);
            }
            (_, _) => {}
        }
    }
}
