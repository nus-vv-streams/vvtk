use clap::Parser;
use std::fs::copy;
use std::fs::read_dir;
use std::fs::File;
use std::fs::ReadDir;
use std::path::PathBuf;

// take files from input folder and a simulated network condition,
// then output files of varying qualities into output folder
#[derive(Parser)]
struct Args {
    #[clap(parse(from_os_str))]
    input_path: PathBuf,
    output_path: PathBuf,
    network_path: PathBuf,
    frames: usize,
}

fn main() {
    // e.g. vvdash ./input ./output ./simulated_network.txt 300
    // vvdash [input_path] [output_path] [network_conditions] [number_of_frames]
    let args: Args = Args::parse();
    let input_path = args.input_path;
    let output_path = args.output_path;
    let network_path = args.network_path;
    let total_frames = args.frames;

    // reading network conditions
    let network_content =
        std::fs::read_to_string(network_path).expect("could not read network file");
    let mut bandwidth: Vec<i32> = Vec::new();
    for line in network_content.lines() {
        bandwidth.push(line.parse().unwrap());
    }

    let mut count: usize = 0;

    // assume default input folder is always the "hi" folder
    let mut input_folder_hi = input_path.clone();
    input_folder_hi.push("hi");
    let mut input_folder_lo = input_path;
    input_folder_lo.push("lo");
    let mut input_folder: ReadDir;
    let mut input_folder_pathbuf: &PathBuf;

    while count < total_frames {
        // arbitrary separation between lo and hi quality: 150KB/s
        if bandwidth[count] < 150 {
            input_folder = read_dir(&input_folder_lo).unwrap();
            input_folder_pathbuf = &input_folder_lo;
        } else {
            input_folder = read_dir(&input_folder_hi).unwrap();
            input_folder_pathbuf = &input_folder_hi;
        }

        // get the correct frame from the input folder
        let input_frame_name: PathBuf = input_folder.nth(count).unwrap().ok().unwrap().path();

        let mut input_frame = input_folder_pathbuf.clone();
        input_frame.push(input_frame_name.file_name().unwrap());
        let mut output_frame = output_path.clone();
        output_frame.push(input_frame_name.file_name().unwrap());
        let _o = File::create(&output_frame);
        copy(&input_frame, &output_frame).unwrap_or_else(|_| {
            panic!(
                "failed to copy from {} to {}",
                &input_frame.display(),
                &output_frame.display()
            )
        });

        count += 1;
    }

    // major issue with implementation: readdir() does not guarentee the order (alphabetical),
    // may need to experiment with reading into a Vec and then sorting the files
    // order of arrival of frames matter when taken into a buffer during streaming
    // https://stackoverflow.com/questions/40021882/how-to-sort-readdir-iterator
}
