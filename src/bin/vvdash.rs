use clap::Parser;
use regex::Regex;
use std::fs;
use std::fs::copy;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

// take binary files from input folder and a simulated network condition,
// then output binary files of varying qualities into output folder (should decoding be done here?)
#[derive(Parser)]
struct Args {
    #[clap(parse(from_os_str))]
    input_path: PathBuf,
    output_path: PathBuf,
    network_path: PathBuf,
    algorithm: String,
    throughput_estimation: String,
}

fn get_filename(filepath: &Path) -> io::Result<()> {
    // Get all files in target directory.
    // Replace "." with a more useful directory if needed.
    for entry in fs::read_dir(filepath)? {
        let path = entry?.path();
        // Get path string.
        let path_str = path.to_str().unwrap();
        println!("PATH: {}", path_str);
    }
    Ok(())
}

fn get_entries(filepath: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(filepath)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;

    // The order in which `read_dir` returns entries is not guaranteed. If reproducible
    // ordering is required the entries should be explicitly sorted.

    entries.sort();

    // The entries have now been sorted by their path.

    Ok(entries)
}

fn main() {
    let args: Args = Args::parse();
    let input_path = args.input_path;
    let output_path = args.output_path;
    let network_path = args.network_path;
    let algorithm = args.algorithm;
    let throughput_estimation = args.throughput_estimation;
    let start_no: usize;

    // reading network conditions
    let network_content =
        std::fs::read_to_string(network_path).expect("could not read network file");
    // using f32 for bandwidth in KB/s
    let mut bandwidth: Vec<f32> = Vec::new();
    for line in network_content.lines() {
        bandwidth.push(line.parse().unwrap());
    }

    let mut starting_frame_int: usize = 0;
    let mut frame_increment_int: usize = 0;
    let mut count: usize = 0;
    let mut total_frames: usize = 0;
    let extension = "pcd";

    let mut input_folder_R01 = input_path.clone();
    input_folder_R01.push(format!("{}", "R01"));
    let mut input_folder_R02 = input_path.clone();
    input_folder_R02.push(format!("{}", "R02"));
    let mut input_folder_R03 = input_path.clone();
    input_folder_R03.push(format!("{}", "R03"));
    let mut input_folder_R04 = input_path.clone();
    input_folder_R04.push(format!("{}", "R04"));
    let mut input_folder_R05 = input_path.clone();
    input_folder_R05.push(format!("{}", "R05"));
    // let mut input_folder: ReadDir;
    let mut input_folder_pathbuf: &PathBuf;

    // getting entries from default input folder ("R05" folder) to retrieve infomation (any alternatives?)
    let entries = get_entries(input_folder_R05.as_path()).expect("failed to get entries");

    let re = Regex::new(r"(.{7})(.{3})_(.{3})_(.{3})_(\d{4}).pcd").unwrap();

    let first_entry_filename = entries[0].as_path().to_str().unwrap();
    let first_entry_filename_short =
        &first_entry_filename[(input_folder_R05.as_path().to_str().unwrap().chars().count() + 1)..]; // + 1 for the slash /
    println!("First entry filename: {}", first_entry_filename_short);
    assert!(re.is_match(first_entry_filename_short)); // panics if file name not a match, able to input regex as CLI params?

    // S25C2AIR05_F30_rec_0536.pcd -> [R05] [F30] [0536] information needed for decoding are retrieved from file name
    for cap in re.captures_iter(first_entry_filename_short) {
        let prefix = &cap[1].to_owned();
        let rate = &cap[2].to_owned();
        let frame_count = &cap[3].to_owned();
        let starting_frame = &cap[5].to_owned();

        // frame_count is 'F30', substring
        frame_increment_int = 1;
        starting_frame_int = starting_frame.parse().unwrap();
        total_frames = entries.len() * frame_increment_int;
    }

    start_no = starting_frame_int;

    while count < total_frames {
        let quality: &str;
        // buffer-based approach used for rate adaptation, appropriate lower and higher reservoir
        // needed in order to avoid overflow and underflow
        let mut bandwidth_buf: f32 = 0.0;
        for i in count..count + frame_increment_int {
            bandwidth_buf += bandwidth[i];
        }

        // a simplistic average value is chosen (4411.4, size in KB of 1s of R05 binaries),
        // additional research needed to determine appropriate lower and higher reservoir values
        // for simulation purposes, use the .bin file sizes as benchmark for values (naive algo)
        if bandwidth_buf < 120.0 {
            // input_folder = read_dir(&input_folder_lo).unwrap();
            input_folder_pathbuf = &input_folder_R01;
            quality = "R01";
        } else {
            // input_folder = read_dir(&input_folder_hi).unwrap();
            input_folder_pathbuf = &input_folder_R05;
            quality = "R05";
        }

        // take and use the regex input from CLI, format of frame name to be considered:
        // can extract out regex of format 'S25C2AI' to be an input param in the future (-f flag?)
        let in_frame_name = format!(
            "S25C2AI{}_F30_rec_{}.{}",
            quality,
            format!("{:0>4}", count + start_no),
            extension
        );

        let out_frame_name = format!("out_{}_{}.{}", count, quality, extension);

        let mut input_frame = input_folder_pathbuf.clone();
        input_frame.push(&in_frame_name);
        let mut output_frame = output_path.clone();
        output_frame.push(&out_frame_name);
        let _o = File::create(&output_frame);
        copy(&input_frame, &output_frame).expect(&format!(
            "failed to copy from {} to {}",
            &input_frame.display(),
            &output_frame.display()
        ));

        count += frame_increment_int;
    }

    // decoding after binaries are in out folder (should be separate function?)
    //
}
