use std::env;
use std::path::PathBuf;
use vivotk::codec::decoder::{MultiplaneDecodeReq, MultiplaneDecoder};
use vivotk::codec::Decoder;
use vivotk::pcd::PointCloudData;

fn main() {
    let _output_file = env::args().nth(1).expect("output file");
    let left = env::args().nth(2).expect("input file");
    let bottom = env::args().nth(3).expect("input file");
    let back = env::args().nth(4).expect("input file");
    let right = env::args().nth(5).expect("input file");
    let top = env::args().nth(6).expect("input file");
    let front = env::args().nth(7).expect("input file");

    let mut decoder = MultiplaneDecoder::new(MultiplaneDecodeReq {
        left: PathBuf::from(left),
        bottom: PathBuf::from(bottom),
        back: PathBuf::from(back),
        right: PathBuf::from(right),
        top: PathBuf::from(top),
        front: PathBuf::from(front),
    });
    let now = std::time::Instant::now();
    decoder.start().unwrap();
    while let Some(pc) = decoder.poll() {
        let pcd = PointCloudData::from(&pc);
        dbg!(pcd.header().points());
        // write_pcd_file(&pcd, PCDDataType::Ascii, &output_file).unwrap();
    }
    let elapsed = now.elapsed();
    dbg!("Decoding took {:?} seconds", elapsed);
}
