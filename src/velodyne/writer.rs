use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use byteorder::{NativeEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

use crate::velodyne::{VelodynPoint, VelodyneBinData};

type IOResult = Result<(), std::io::Error>;

/// Writes the point cloud into the file
pub fn write_pcd_file<P: AsRef<Path>>(
    vbd: &VelodyneBinData,
    p: P,
) -> IOResult {
    let file = File::create(p)?;
    let writer = BufWriter::new(file);
    Writer::new(vbd, writer).write()?;
    Ok(())
}

struct Writer<'a, W: Write> {
    writer: W,
    vbd: &'a VelodyneBinData,
}

impl<'a, W: Write> Writer<'a, W> {
    pub fn new(vbd: &'a VelodyneBinData, writer: W) -> Self {
        Self {
            vbd,
            writer,
        }
    }

    fn write(mut self) -> IOResult {
        for point in self.vbd.data.iter() {
            self.writer.write_all(&point.to_bytes())?;
        }
        Ok(())
    }

}