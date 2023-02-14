use crate::codec::Decoder;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::utils::read_file_to_point_cloud;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Error, Result};

pub struct NoopDecoder {
    to_decode: PathBuf,
    pcd: Option<PointCloud<PointXyzRgba>>,
}

impl NoopDecoder {
    pub fn new(filename: &OsStr) -> Self {
        return NoopDecoder {
            to_decode: PathBuf::from(filename),
            pcd: None,
        };
    }
}

impl Decoder for NoopDecoder {
    fn start(&mut self) -> Result<()> {
        self.pcd = read_file_to_point_cloud(&self.to_decode);
        Ok(())
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

pub struct MultiplaneDecoder {
    top: tmc2rs::Decoder,
    bottom: tmc2rs::Decoder,
    left: tmc2rs::Decoder,
    right: tmc2rs::Decoder,
    front: tmc2rs::Decoder,
    back: tmc2rs::Decoder,
}

/// (5Feb) For now, it is not important to know whether the pathbuf really corresponds to a top, bottom, left, right, front or back image.
pub struct MultiplaneDecodeReq {
    pub top: PathBuf,
    pub bottom: PathBuf,
    pub left: PathBuf,
    pub right: PathBuf,
    pub front: PathBuf,
    pub back: PathBuf,
}

impl MultiplaneDecoder {
    pub fn new(req: MultiplaneDecodeReq) -> Self {
        return MultiplaneDecoder {
            top: tmc2rs::Decoder::new(tmc2rs::Params::new(req.top, None)),
            bottom: tmc2rs::Decoder::new(tmc2rs::Params::new(req.bottom, None)),
            left: tmc2rs::Decoder::new(tmc2rs::Params::new(req.left, None)),
            right: tmc2rs::Decoder::new(tmc2rs::Params::new(req.right, None)),
            front: tmc2rs::Decoder::new(tmc2rs::Params::new(req.front, None)),
            back: tmc2rs::Decoder::new(tmc2rs::Params::new(req.back, None)),
        };
    }
}

impl Decoder for MultiplaneDecoder {
    fn start(&mut self) -> Result<()> {
        // start all decoders. This will run in parallel
        self.front.start();
        self.back.start();
        self.left.start();
        self.right.start();
        self.top.start();
        self.bottom.start();
        Ok(())
    }

    fn poll(&mut self) -> Option<PointCloud<PointXyzRgba>> {
        // assume all decoders have the same number of frames
        let front = self.front.recv_frame();
        if front.is_none() {
            return None;
        }
        let front = front.unwrap();
        let back = self.back.recv_frame().unwrap();
        let left = self.left.recv_frame().unwrap();
        let right = self.right.recv_frame().unwrap();
        let top = self.top.recv_frame().unwrap();
        let bottom = self.bottom.recv_frame().unwrap();

        // combining all viewpoints into one
        let front = PointCloud::from(front);
        let back = PointCloud::from(back);
        let left = PointCloud::from(left);
        let right = PointCloud::from(right);
        let top = PointCloud::from(top);
        let bottom = PointCloud::from(bottom);

        let mut combined = PointCloud {
            number_of_points: 0,
            points: Vec::new(),
        };
        combined.combine(&front);
        combined.combine(&back);
        combined.combine(&left);
        combined.combine(&right);
        combined.combine(&top);
        combined.combine(&bottom);

        Some(combined)
    }
}
