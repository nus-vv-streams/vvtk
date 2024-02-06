use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use clap::Parser;
use regex::Regex;
use std::path::PathBuf;

use crate::pipeline::subcommands::read::FileType;
use crate::utils::{find_all_files, read_file_to_point_cloud};
use std::str::FromStr;

use crate::abr::quetra::Quetra;
use crate::abr::RateAdapter;

#[derive(Debug, Copy, Clone, Eq, PartialEq, clap::ValueEnum)]
enum DashAlgo {
    Naive,
    Quetra,
}

impl ToString for DashAlgo {
    fn to_string(&self) -> String {
        match self {
            DashAlgo::Naive => "naive".to_string(),
            DashAlgo::Quetra => "quetra".to_string(),
        }
    }
}

impl FromStr for DashAlgo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "naive" => Ok(DashAlgo::Naive),
            "quetra" => Ok(DashAlgo::Quetra),
            _ => Err("unknown algorithm".to_string()),
        }
    }
}

#[derive(Parser)]
#[clap(
    about = "Dash will simulate a varying network conditions, it reads in one of our supported file formats. \nFiles can be of the type .pcd .ply. \nThe path can be a file path or a directory path contains these files.",
    override_usage = format!("\x1B[1m{}\x1B[0m [OPTIONS] <FILES>... +output=plys", "dash")
)]
pub struct Args {
    /// input directory with different quality of point clouds
    input_path: PathBuf,
    /// path to network settings
    network_path: PathBuf,
    #[clap(short, long, default_value = "naive")]
    algorithm: DashAlgo,
    #[clap(short, long)]
    /// read previous n files after sorting lexicalgraphically
    num: Option<usize>,
    #[clap(short = 't', long, value_enum, default_value_t = FileType::All)]
    filetype: FileType,
}

pub struct Dash {
    args: Args,
}

impl Dash {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Dash {
            args: Args::parse_from(args),
        })
    }

    fn prepare_bandwidth(&self) -> Vec<f32> {
        // reading network conditions
        let network_content = std::fs::read_to_string(self.args.network_path.clone())
            .expect("could not read network file");
        // using f32 for bandwidth in KB/s
        let mut bandwidth: Vec<f32> = Vec::new();
        for line in network_content.lines() {
            bandwidth.push(line.parse().unwrap());
        }
        bandwidth
    }

    fn main_process(&self) -> Vec<PathBuf> {
        // adapt from vvdash.rs
        let bandwidth = self.prepare_bandwidth();

        let mut starting_frame_int: usize = 0;
        let mut _frame_increment_int: usize = 0;
        let mut count: usize = 0;
        let mut total_frames: usize = 0;
        let extension = "pcd";

        let mut input_folder_r01 = self.args.input_path.clone();
        input_folder_r01.push(format!("{}", "R01"));
        let mut input_folder_r02 = self.args.input_path.clone();
        input_folder_r02.push(format!("{}", "R02"));
        let mut input_folder_r03 = self.args.input_path.clone();
        input_folder_r03.push(format!("{}", "R03"));
        let mut input_folder_r04 = self.args.input_path.clone();
        input_folder_r04.push(format!("{}", "R04"));
        let mut input_folder_r05 = self.args.input_path.clone();
        input_folder_r05.push(format!("{}", "R05"));
        // let mut input_folder: ReadDir;
        let mut input_folder_pathbuf: &PathBuf;

        // longdress format: r1_longdress_dec_0000.ply
        let mut entries = find_all_files(vec![input_folder_r05.clone().into_os_string()].as_ref());
        entries.sort();
        let re = Regex::new(r"(.{2})_(.{9})_(.{3})_(\d{4}).pcd").unwrap();
        let first_entry_filename = entries[0].as_path().to_str().unwrap();
        let first_entry_filename_short = &first_entry_filename
            [(input_folder_r05.as_path().to_str().unwrap().chars().count() + 1)..]; // + 1 for the slash /
        assert!(re.is_match(first_entry_filename_short)); // panics if file name not a match, able to input regex as CLI params?

        // S25C2AIR05_F30_rec_0536.pcd -> [R05] [F30] [0536] information needed for decoding are retrieved from file name
        for cap in re.captures_iter(first_entry_filename_short) {
            let starting_frame = &cap[4].to_owned();
            // frame_count is 'F30', substring
            _frame_increment_int = 1;
            starting_frame_int = starting_frame.parse().unwrap();
            total_frames = entries.len() * _frame_increment_int;
        }

        let available_bitrates = vec![vec![4641, 7975, 14050, 25974, 46778]];
        let start_no = starting_frame_int;

        let mut in_frame_name_buf = Vec::new();
        match self.args.algorithm {
            DashAlgo::Naive => {
                while count < total_frames {
                    let rate_prefix: &str;
                    // buffer-based approach used for rate adaptation, appropriate lower and higher reservoir
                    // needed in order to avoid overflow and underflow
                    let mut bandwidth_buf: f32 = 0.0;
                    // for i in count..count + frame_increment_int {}
                    bandwidth_buf += bandwidth[count / 30];

                    // for simulation purposes, use the .bin file sizes as benchmark for values (naive algo)
                    // values used for longdress, R01 to R05
                    if bandwidth_buf < available_bitrates[0][0] as f32 {
                        input_folder_pathbuf = &input_folder_r01;
                        rate_prefix = "r1";
                    } else if bandwidth_buf < available_bitrates[0][1] as f32 {
                        input_folder_pathbuf = &input_folder_r02;
                        rate_prefix = "r2";
                    } else if bandwidth_buf < available_bitrates[0][2] as f32 {
                        input_folder_pathbuf = &input_folder_r03;
                        rate_prefix = "r3";
                    } else if bandwidth_buf < available_bitrates[0][3] as f32 {
                        input_folder_pathbuf = &input_folder_r04;
                        rate_prefix = "r4";
                    } else {
                        input_folder_pathbuf = &input_folder_r05;
                        rate_prefix = "r5";
                    }

                    // longdress format: r1_longdress_dec_0000.ply
                    for i in count..count + 30 {
                        let in_frame_name = format!(
                            "{}_longdress_dec_{}.{}",
                            rate_prefix,
                            format!("{:0>4}", i + start_no),
                            extension
                        );

                        // let out_frame_name = format!("out_{}_{}.{}", format!("{:0>4}", i), quality, extension);
                        let mut input_frame = input_folder_pathbuf.clone();
                        input_frame.push(in_frame_name);
                        in_frame_name_buf.push(input_frame);
                    }
                    count += 30;
                }
                in_frame_name_buf
            }
            DashAlgo::Quetra => {
                let mut buffer_status: Vec<u64> = Vec::new();
                let mut quality_selected: Vec<u64> = Vec::new();
                // buffer capacity set to 10 seconds, fps 30
                let quetra = Quetra::new(10, 30.0);

                let mut buffer_occupancy = 0;
                let mut network_throughput;

                let cosines = vec![];

                while count < total_frames {
                    let rate_prefix: &str;
                    network_throughput = (bandwidth[count]) as f64;
                    let quality = quetra.select_quality(
                        buffer_occupancy,
                        network_throughput,
                        &available_bitrates,
                        &cosines,
                    );
                    // dbg!(network_throughput, quality[0]);

                    // fill buffer based on the downloaded segment duration
                    let download_bitrate = available_bitrates[0][quality[0]] as f64;
                    let no_of_frames: usize = (network_throughput / download_bitrate) as usize;
                    buffer_occupancy = (no_of_frames) as u64;
                    buffer_status.push(buffer_occupancy);

                    if quality[0] == 0 {
                        input_folder_pathbuf = &input_folder_r01;
                        rate_prefix = "r1";
                        quality_selected.push(1);
                    } else if quality[0] == 1 {
                        input_folder_pathbuf = &input_folder_r02;
                        rate_prefix = "r2";
                        quality_selected.push(2);
                    } else if quality[0] == 2 {
                        input_folder_pathbuf = &input_folder_r03;
                        rate_prefix = "r3";
                        quality_selected.push(3);
                    } else if quality[0] == 3 {
                        input_folder_pathbuf = &input_folder_r04;
                        rate_prefix = "r4";
                        quality_selected.push(4);
                    } else {
                        input_folder_pathbuf = &input_folder_r05;
                        rate_prefix = "r5";
                        quality_selected.push(5);
                    }

                    // longdress format: r1_longdress_dec_0000.ply
                    let in_frame_name = format!(
                        "{}_longdress_dec_{}.{}",
                        rate_prefix,
                        format!("{:0>4}", count + start_no),
                        extension
                    );

                    let mut input_frame = input_folder_pathbuf.clone();
                    input_frame.push(&in_frame_name);
                    count += 1;
                    in_frame_name_buf.push(input_frame);
                }

                in_frame_name_buf
            }
        }
    }
}

impl Subcommand for Dash {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        if messages.is_empty() {
            let mut in_frame_name_buf = self.main_process();
            if let Some(num) = self.args.num {
                if num < in_frame_name_buf.len() {
                    in_frame_name_buf = in_frame_name_buf.into_iter().take(num).collect();
                }
            }

            for (i, file) in in_frame_name_buf.iter().enumerate() {
                match &self.args.filetype {
                    FileType::All => {}
                    FileType::Pcd => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("pcd") {
                            continue;
                        }
                    }
                    FileType::Ply => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("ply") {
                            continue;
                        }
                    }
                    FileType::Bin => {
                        if file.extension().and_then(|ext| ext.to_str()) != Some("bin") {
                            continue;
                        }
                    }
                }

                let point_cloud = read_file_to_point_cloud(file);
                if let Some(pc) = point_cloud {
                    channel.send(PipelineMessage::IndexedPointCloud(pc, i as u32));
                }
            }
            channel.send(PipelineMessage::End);
        } else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}
