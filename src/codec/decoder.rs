use crate::codec::Decoder;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::utils::read_file_to_point_cloud;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Error, Result};
use log::debug;

pub struct NoopDecoder {
    to_decode: PathBuf,
    pcd: Option<PointCloud<PointXyzRgba>>,
}

impl NoopDecoder {
    pub fn new(filename: &OsStr) -> Self {
        NoopDecoder {
            to_decode: PathBuf::from(filename),
            pcd: None,
        }
    }
}

impl Decoder for NoopDecoder {
    fn start(&mut self) -> Result<()> {
        let now = std::time::Instant::now();
        self.pcd = read_file_to_point_cloud(&self.to_decode);
        let elapsed = now.elapsed();
        println!("Read file to point cloud took {:?}", elapsed);
        self.pcd
            .as_ref()
            .map(|_| ())
            .ok_or(Error::msg("Fail to read point cloud"))
    }

    fn poll(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.pcd.take()
    }

    // fn decode_folder(&self, directory: &Path) -> Result<()> {
    //     let path = Path::new(directory);
    //     for file_entry in path.read_dir().unwrap() {
    //         match file_entry {
    //             Ok(_entry) => {}
    //             Err(e) => return Err(Error::from(e)),
    //         }
    //     }
    //     Ok(())
    // }
}

pub struct DracoDecoder {
    path: PathBuf,
    filename: PathBuf,
    pcd: Option<PointCloud<PointXyzRgba>>,
}

impl DracoDecoder {
    pub fn new<P, Q>(path: P, filename: Q) -> Self
    where
        P: Into<PathBuf>,
        Q: Into<PathBuf>,
    {
        DracoDecoder {
            path: path.into(),
            filename: filename.into(),
            pcd: None,
        }
    }
}

impl Decoder for DracoDecoder {
    fn start(&mut self) -> Result<()> {
        // filename must be full path
        let mut output_filename = self.filename.clone();
        output_filename.set_extension("ply");
        let status = Command::new(&self.path)
            .arg("-i")
            .arg(&self.filename)
            .arg("-o")
            .arg(output_filename.to_str().unwrap())
            .status();
        if status.is_ok() {
            self.pcd = read_file_to_point_cloud(&output_filename);
            Ok(())
        } else {
            Err(Error::from(status.err().unwrap()))
        }
    }

    fn poll(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        self.pcd.take()
    }

    // fn decode_folder(&self, directory: &Path) -> Result<()> {
    //     let path = Path::new(directory);
    //     for file_entry in path.read_dir().unwrap() {
    //         match file_entry {
    //             Ok(entry) => {
    //                 let path = entry.path();
    //                 if path.is_file() {
    //                     self.decode(path.into_os_string().as_os_str());
    //                 }
    //             }
    //             Err(e) => return Err(Error::from(e)),
    //         }
    //     }
    //     Ok(())
    // }
}

pub struct Tmc2rsDecoder {
    decoders: Vec<tmc2rs::Decoder>,
}

impl Tmc2rsDecoder {
    pub fn new(paths: &[PathBuf]) -> Self {
        let decoders = paths
            .iter()
            .map(|path| tmc2rs::Decoder::new(tmc2rs::Params::new(path.to_owned())))
            .collect::<Vec<_>>();
        Tmc2rsDecoder { decoders }
    }
}

impl Decoder for Tmc2rsDecoder {
    fn start(&mut self) -> Result<()> {
        // start all decoders. This will run in parallel
        for decoder in self.decoders.iter_mut() {
            decoder.start();
        }
        Ok(())
    }

    fn poll(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        // assume all decoders have the same number of frames
        let now = std::time::Instant::now();
        let frame = self
            .decoders
            .iter()
            .map(|decoder| decoder.recv_frame())
            .map(|frame| frame.map(PointCloud::from))
            .reduce(|mut acc, frame| {
                acc.as_ref()?;
                acc.as_mut().unwrap().combine(&frame.unwrap());
                acc
            })
            .unwrap();
        let elapsed = now.elapsed();
        debug!("Decoder for 6 frames took {} ms", elapsed.as_millis());
        frame
    }
}
