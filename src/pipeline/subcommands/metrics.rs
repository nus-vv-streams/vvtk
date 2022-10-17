use std::{
    ffi::OsString,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use clap::Parser;

use crate::{
    metrics::calculate_metrics,
    pipeline::{channel::Channel, PipelineMessage, Progress},
    utils::{find_all_files, read_file_to_point_cloud},
};

use super::Subcommand;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    reference: Vec<OsString>,

    #[clap(short, long)]
    output_dir: OsString,
}

pub struct Metrics {
    files: Vec<PathBuf>,
    output_path: OsString,
    count: usize,
}

impl Metrics {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        std::fs::create_dir_all(Path::new(&args.output_dir))
            .expect("Failed to create output directory");
        let mut files = find_all_files(&args.reference);
        files.sort();
        Box::new(Metrics {
            files,
            count: 0,
            output_path: args.output_dir,
        })
    }
}

impl Subcommand for Metrics {
    fn handle(
        &mut self,
        messages: Vec<PipelineMessage>,
        channel: &Channel,
        progress: &Sender<Progress>,
    ) {
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
                let output_path = Path::new(&self.output_path);
                let file_name = format!("{}.metrics", self.count);
                self.count += 1;
                let file_name = Path::new(&file_name);
                let output_file = output_path.join(file_name);
                let file = File::create(&output_file).expect(&format!(
                    "Failed to open file {:?}",
                    output_file.as_os_str()
                ));
                let mut writer = BufWriter::new(file);
                metrics.write_to(&mut writer);
                progress.send(Progress::Incr);
            }
            (PipelineMessage::End, _) | (_, PipelineMessage::End) => {
                progress.send(Progress::Completed);
            }
        }
    }
}
