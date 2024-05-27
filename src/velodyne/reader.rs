use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

use crate::velodyne::data_types::{VelodynPoint, VelodyneBinData};

type Result<T> = std::result::Result<T, VelodynBinReadError>;

#[derive(Error, Debug)]
pub enum VelodynBinReadError {
    /// For ease of conversion from IO errors to PCDReadError.
    /// Note that error can still be due to an invalid encoding of PCD
    /// E.g. attempting to access the next line when there are no more lines.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// Represents an error with the data of the file.
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Reads [Velodyne Bin File] directly from a file given the path
pub fn read_velodyn_bin_file<P: AsRef<Path>>(p: P) -> Result<VelodyneBinData> {
    let file = File::open(p).map_err(VelodynBinReadError::IOError)?;
    let reader = BufReader::new(file);
    Parser::new(reader).parse()
}

struct Parser<R: BufRead> {
    reader: R,
}

impl<R: BufRead> Parser<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn parse(self) -> Result<VelodyneBinData> {
        let data = self
            .reader
            .bytes()
            .collect::<Vec<_>>()
            .chunks_exact(16)
            .map(|chunk| {
                let bytes = chunk
                    .iter()
                    .map(|b| *b.as_ref().unwrap())
                    .collect::<Vec<_>>();
                VelodynPoint::from_bytes(&bytes)
            })
            .collect();
        Ok(VelodyneBinData { data })
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
pub fn read_bin_to_point_cloud_xyzrgba<P: AsRef<Path>>(
    path_buf: P,
) -> Option<PointCloud<PointXyzRgba>> {
    let p = PointXyzRgba {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    Some(PointCloud {
        number_of_points: (1),
        points: (vec![p]),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read() {
        let path = Path::new("test_files/velodyne/000001.bin");
        let data = read_velodyn_bin_file(path).unwrap();
        print!("{:?}", data);
        println!("1 point: {:?}", data.data()[0]);
        println!("2 point: {:?}", data.data()[1]);
        println!("3 point: {:?}", data.data()[2]);
    }
}
