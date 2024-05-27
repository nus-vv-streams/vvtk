use cgmath::num_traits::pow;
use clap::Parser;
// use log::warn;

use crate::formats::metadata::MetaData;
use crate::pcd::{
    create_pcd, create_pcd_from_pc_normal, write_pcd_data, write_pcd_file, PCDDataType,
};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::utils::{
    pcd_to_ply_from_data, pcd_to_ply_from_data_normal, pcd_to_ply_from_data_with_faces,
    ConvertOutputFormat,
};
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
    metadata: Option<MetaData>,
}

impl Write {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args = Args::parse_from(args);
        std::fs::create_dir_all(Path::new(&args.output_dir))
            .expect("Failed to create output directory");
        Box::from(Write {
            args,
            count: 0,
            metadata: None,
        })
    }
}

impl Subcommand for Write {
    // Possible change: merge the copy and paste part of the code
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        // println!("Start writing...");
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
                PipelineMessage::IndexedPointCloudWithTriangleFaces(pc, i, triangle_faces) => {
                    println!("Writing point cloud with {} points", pc.points.len());
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
                            if let Err(e) = pcd_to_ply_from_data_with_faces(
                                &output_file,
                                pcd_data_type,
                                pcd,
                                triangle_faces,
                            ) {
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
                PipelineMessage::SubcommandMessage(subcommand_object, i) => {
                    // Only vv extend will send SubcommandMessage, other subcommand will send IndexedPointCloud to make sure the other command will
                    // continue to be compatible by receiving IndexedPointCloud
                    let pc = subcommand_object.get_content();
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
                            if let Err(_e) = pcd_to_ply_from_data(&output_file, pcd_data_type, pcd) {
                            //if let Err(e) =
                            //    pcd_to_ply_from_data_normal(&output_file, pcd_data_type, pcd)
                            //{
                            //    println!("Failed to write {:?}\n{e}", output_file);
                            //}
                            }
                        }
                        _ => {
                            println!("Unsupported output format {}", output_format);
                            continue;
                        }
                    }
                }
                PipelineMessage::IndexedPointCloudWithName(pc, i, name, with_header) => {
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
                    let subfolder = output_path.join(name);
                    let output_file = subfolder.join(file_name);
                    if !subfolder.exists() {
                        std::fs::create_dir_all(&subfolder)
                            .expect("Failed to create output directory");
                    }

                    let pcd = create_pcd(pc);

                    match output_format.as_str() {
                        "pcd" => {
                            if *with_header {
                                if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                                    println!("Failed to write {:?}\n{e}", output_file);
                                }
                            } else {
                                if let Err(e) = write_pcd_data(&pcd, pcd_data_type, &output_file) {
                                    println!("Failed to write {:?}\n{e}", output_file);
                                }
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
                PipelineMessage::MetaData(
                    bound,
                    base_point_num,
                    additional_point_num,
                    partitions,
                ) => {
                    if self.metadata.is_none() {
                        self.metadata = Some(MetaData::default());
                    }
                    self.metadata.as_mut().unwrap().next(
                        bound.clone(),
                        base_point_num.clone(),
                        additional_point_num.clone(),
                    );
                    self.metadata.as_mut().unwrap().partitions = *partitions;
                }
                PipelineMessage::End => {
                    if let Some(metadata) = &self.metadata {
                        if !output_path.exists() {
                            std::fs::create_dir_all(output_path)
                                .expect("Failed to create output directory");
                        }

                        let metadata_file = output_path.join("metadata.json");
                        let json = serde_json::to_string_pretty(metadata).unwrap();
                        std::fs::write(metadata_file, json).expect("Unable to write file");
                    }
                }
                PipelineMessage::DummyForIncrement => {}
            }
            channel.send(message);
        }
    }
}
