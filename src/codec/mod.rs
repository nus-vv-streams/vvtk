use anyhow::Result;

use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;

pub mod decoder;

pub trait Decoder {
    fn start(&mut self) -> Result<()>;
    fn poll(&mut self) -> Option<PointCloud<PointXyzRgba>>;
    // fn decode_folder(&self, foldername: &Path) -> Result<()>;
}
