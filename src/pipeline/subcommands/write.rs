use cgmath::num_traits::pow;
use clap::Parser;

use crate::formats::metadata::MetaData;
use crate::pcd::{
    create_pcd, create_pcd_from_pc_normal, create_pcd_from_pc_segment, write_pcd_file, PCDDataType,
};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::utils::{pcd_to_ply_from_data, pcd_to_ply_from_data_normal, ConvertOutputFormat};
use std::fs::File;
use std::path::Path;

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "Writes from input stream into a file, input stream can be pointcloud data or metrics",
    override_usage = format!("\x1B[1m{}\x1B[0m [OPTIONS] <output_dir> +input=plys", "write")
)]
pub struct Args {
    /// output directory to store point cloud files or metrics
    output_dir: String,

    #[clap(long, default_value = "pcd")]
    output_format: ConvertOutputFormat,

    #[clap(short, long, default_value = "binary")]
    storage_type: Option<PCDDataType>,

    #[clap(long, default_value_t = 5)]
    name_length: usize,
}
pub struct Write {
    args: Args,
    count: u64,
    metadata: MetaData,
}

impl Write {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args = Args::parse_from(args);
        std::fs::create_dir_all(Path::new(&args.output_dir))
            .expect("Failed to create output directory");
        Box::from(Write {
            args,
            count: 0,
            metadata: MetaData::default(),
        })
    }
}

impl Subcommand for Write {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        let output_path = Path::new(&self.args.output_dir);
        let max_count = pow(10, self.args.name_length);

        for message in messages {
            match &message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    // println!("Writing point cloud with point num {}", pc.points.len());
                    let pcd_data_type = self
                        .args
                        .storage_type
                        .expect("PCD data type should be provided");
                    let output_format = self.args.output_format.to_string();

                    // !! use index(i) instead of count to make sure the order of files
                    let padded_count = format!("{:0width$}", i, width = self.args.name_length);
                    let file_name = format!("{}.{}", padded_count, output_format);
                    self.count += 1;
                    if self.count >= max_count {
                        channel.send(PipelineMessage::End);
                        panic!("Too many files, please increase the name length by setting --name-length")
                    }

                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    if !output_path.exists() {
                        std::fs::create_dir_all(output_path)
                            .expect("Failed to create output directory");
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
                PipelineMessage::IndexedPointCloudNormal(pc, i) => {
                    // println!("Writing point cloud with point num {}", pc.points.len());
                    let pcd_data_type = self
                        .args
                        .storage_type
                        .expect("PCD data type should be provided");
                    let output_format = self.args.output_format.to_string();

                    // !! use index(i) instead of count to make sure the order of files
                    let padded_count = format!("{:0width$}", i, width = self.args.name_length);
                    let file_name = format!("{}.{}", padded_count, output_format);
                    self.count += 1;
                    if self.count >= max_count {
                        channel.send(PipelineMessage::End);
                        panic!("Too many files, please increase the name length by setting --name-length")
                    }

                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    if !output_path.exists() {
                        std::fs::create_dir_all(output_path)
                            .expect("Failed to create output directory");
                    }

                    // use pcd format as a trasition format now
                    let pcd = create_pcd_from_pc_normal(pc);

                    match output_format.as_str() {
                        "pcd" => {
                            if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                                println!("Failed to write {:?}\n{e}", output_file);
                            }
                        }
                        "ply" => {
                            if let Err(e) =
                                pcd_to_ply_from_data_normal(&output_file, pcd_data_type, pcd)
                            {
                                println!("Failed to write {:?}\n{e}", output_file);
                            }
                        }
                        _ => {
                            println!("Unsupported output format {}", output_format);
                            continue;
                        }
                    }
                }
                PipelineMessage::IndexedPointCloudWithResolution(pc, i, resolution) => {
                    let pcd_data_type = self
                        .args
                        .storage_type
                        .expect("PCD data type should be provided");
                    let output_format = self.args.output_format.to_string();

                    self.count += 1;
                    if self.count >= max_count {
                        channel.send(PipelineMessage::End);
                        panic!("Too many files, please increase the name length by setting --name-length")
                    }

                    for (s_index, segment) in pc.segments.iter().enumerate() {
                        // if segment.points.is_empty() {
                        //     continue;
                        // }

                        let pcd = create_pcd_from_pc_segment(segment);

                        let padded_count = format!("{:0width$}", i, width = self.args.name_length);

                        let (subfolder, file_name) = if pc.is_partitioned() {
                            (
                                output_path
                                    .join(format!("{}", resolution))
                                    .join(format!("{}", padded_count)),
                                format!("{}.{}", s_index, output_format),
                            )
                        } else {
                            (
                                output_path.join(format!("{}", resolution)),
                                format!("{}.{}", padded_count, output_format),
                            )
                        };
                        let file = Path::new(&file_name);

                        if !subfolder.exists() {
                            std::fs::create_dir_all(&subfolder)
                                .expect("Failed to create output directory");
                        }

                        let output_file = subfolder.join(file);

                        match output_format.as_str() {
                            "pcd" => {
                                if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                                    println!("Failed to write {:?}\n{e}", output_file);
                                }
                            }
                            "ply" => {
                                if let Err(e) =
                                    pcd_to_ply_from_data(&output_file, pcd_data_type, pcd)
                                {
                                    println!("Failed to write {:?}\n{e}", output_file);
                                }
                            }
                            _ => {
                                println!("Unsupported output format {}", output_format);
                                continue;
                            }
                        }
                    }
                }
                PipelineMessage::ManifestInformation(
                    bound,
                    point_num,
                    centroid,
                    num_of_additional_file,
                    partitions,
                ) => {
                    self.metadata
                        .next(bound.clone(), point_num.clone(), centroid.clone());
                    self.metadata.num_of_additional_file = *num_of_additional_file;
                    self.metadata.partitions = *partitions;
                }
                PipelineMessage::DummyForIncrement => {}
                PipelineMessage::End => {
                    if self.metadata.num_of_additional_file > 0 {
                        if !output_path.exists() {
                            std::fs::create_dir_all(output_path)
                                .expect("Failed to create output directory");
                        }

                        let metadata_file = output_path.join("metadata.json");
                        let json = serde_json::to_string_pretty(&self.metadata).unwrap();
                        std::fs::write(metadata_file, json).expect("Unable to write file");
                    }
                }
            }
            channel.send(message);
        }
    }
}
