use clap::Parser;

use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::{
    write_pcd_file, PCDDataType, PCDField, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion,
    PointCloudData,
};
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
                    let pcd = create_pcd(pc);
                    let file_name = format!("{}.pcd", self.count);
                    self.count += 1;
                    let file_name = Path::new(&file_name);
                    let output_file = output_path.join(file_name);
                    if let Err(e) = write_pcd_file(&pcd, pcd_data_type, &output_file) {
                        println!("Failed to write {:?}\n{e}", output_file);
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

fn create_pcd(point_cloud: &PointCloud<PointXyzRgba>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgb".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                1,
            )
            .unwrap(),
        ],
        point_cloud.number_of_points as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        point_cloud.number_of_points as u64,
    )
    .unwrap();
    let bytes = unsafe {
        let mut points = std::mem::ManuallyDrop::new(point_cloud.points.clone());
        Vec::from_raw_parts(
            points.as_mut_ptr() as *mut u8,
            point_cloud.number_of_points * std::mem::size_of::<PointXyzRgba>(),
            points.capacity() * std::mem::size_of::<PointXyzRgba>(),
        )
    };
    PointCloudData::new(header, bytes).unwrap()
}
