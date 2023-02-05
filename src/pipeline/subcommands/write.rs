use clap::Parser;
use log::warn;

use crate::pcd::{write_pcd_file, PCDDataType};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use std::fs::File;
use std::path::Path;

use super::Subcommand;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    output_dir: String,

    #[clap(long)]
    pcd: Option<PCDDataType>,
    // TODO: Add option to write as ply
}
pub struct Write {
    args: Args,
    count: u64,
}

impl Write {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args = Args::parse_from(args);
        std::fs::create_dir_all(Path::new(&args.output_dir))
            .expect("Failed to create output directory");
        Box::from(Write { args, count: 0 })
    }
}

impl Subcommand for Write {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        let output_path = Path::new(&self.args.output_dir);
        for message in messages {
            match &message {
                PipelineMessage::PointCloud(pc) => {
                    let pcd_data_type = self.args.pcd.expect("PCD data type should be provided");
                    let pcd = pc.into();
                    let file_name = format!("{}.pcd", self.count);
                    self.count += 1;
                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                        warn!("Failed to write {:?}\n{e}", output_file);
                    }
                }
                PipelineMessage::Metrics(metrics) => {
                    let file_name = format!("{}.metrics", self.count);
                    self.count += 1;
                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    File::create(output_file)
                        .and_then(|mut f| metrics.write_to(&mut f))
                        .expect("Should be able to create file to write metrics to");
                }
                PipelineMessage::End => {}
            }
            channel.send(message);
        }
    }
}
