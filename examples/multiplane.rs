use std::env;
use std::path::PathBuf;
use vivotk::codec::decoder::Tmc2rsDecoder;
use vivotk::codec::Decoder;
use vivotk::pcd::{write_pcd_file, PCDDataType, PointCloudData};

fn main() {
    let output_folder = env::args()
        .nth(1)
        .expect("output folder (e.g. . or ./data)");
    let left = env::args().nth(2).expect("input file");
    let bottom = env::args().nth(3).expect("input file");
    let back = env::args().nth(4).expect("input file");
    let right = env::args().nth(5).expect("input file");
    let top = env::args().nth(6).expect("input file");
    let front = env::args().nth(7).expect("input file");

    let mut decoder = Tmc2rsDecoder::new(&[
        PathBuf::from(left),
        PathBuf::from(bottom),
        PathBuf::from(back),
        PathBuf::from(right),
        PathBuf::from(top),
        PathBuf::from(front),
    ]);
    let now = std::time::Instant::now();
    decoder.start().unwrap();
    let mut file_counter = 1;
    while let Some(pc) = decoder.poll() {
        let pcd = PointCloudData::from(&pc);
        dbg!(pcd.header().points());
        let filename = format!("{}/{}.pcd", output_folder, file_counter);
        file_counter += 1;
        write_pcd_file(&pcd, PCDDataType::Ascii, &filename).unwrap();
    }
    let elapsed = now.elapsed();
    dbg!("Decoding took {:?} seconds", elapsed);
}
