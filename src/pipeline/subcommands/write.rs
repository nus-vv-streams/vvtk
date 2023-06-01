use clap::Parser;


use crate::pcd::{
    write_pcd_file, PCDDataType, create_pcd
};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use std::fs::File;
use std::path::Path;
use crate::utils::{ConvertOutputFormat, pcd_to_ply, pcd_to_ply_from_data};

use super::Subcommand;
#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    output_dir: String,

    #[clap(long, default_value = "pcd")]
    output_format: ConvertOutputFormat,

    #[clap(short, long, default_value = "binary")]
    storage_type: Option<PCDDataType>,
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
                    println!("Writing point cloud with point num {}", pc.points.len());
                    let pcd_data_type = self.args.storage_type.expect("PCD data type should be provided");
                    let output_format = self.args.output_format.to_string();

                    let file_name = format!("{}.{}", self.count, output_format);
                    self.count += 1;
                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    if !output_path.exists() {
                        std::fs::create_dir_all(output_path).expect("Failed to create output directory");
                    }

                    // use pcd format as a trasition format now
                    let pcd = create_pcd(pc);

                    match output_format.as_str() {
                        "pcd" => {
                            if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                                println!("Failed to write {:?}\n{e}", output_file);
                            }
                        }
                        "ply" => {
                            if let Err(e) = pcd_to_ply_from_data(&output_file, pcd_data_type, pcd) {
                                println!("Failed to write {:?}\n{e}", output_file);
                            }
                        }
                        _ => {
                            println!("Unsupported output format {}", output_format);
                            continue;
                        }
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
                PipelineMessage::End | PipelineMessage::DummyForIncrement => {}
            }
            channel.send(message);
        }
    }
}