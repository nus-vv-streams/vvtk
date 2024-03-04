use crate::pcd::data_types::{
    PCDDataType, PCDField, PCDFieldDataType, PCDHeader, PCDVersion, PointCloudData,
};
use std::convert::TryInto;
use std::fmt::Debug;

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

type Result<T> = std::result::Result<T, PCDReadError>;

/// Reads [PointCloudData] directly from a file given the path
pub fn read_pcd_file<P: AsRef<Path>>(p: P) -> Result<PointCloudData> {
    let file = File::open(p).map_err(PCDReadError::IOError)?;
    let reader = BufReader::new(file);
    Parser::new(reader).parse()
}

/// Reads [PointCloudData] directly from a file given the path with a header
pub fn read_pcd_file_with_header<P: AsRef<Path>>(
    p: P,
    header: PCDHeader,
) -> Result<PointCloudData> {
    let file = File::open(p).map_err(PCDReadError::IOError)?;
    let reader = BufReader::new(file);
    Parser::new(reader).parse_data(header)
}

/// Reads [PCDHeader] directly from a file given the path
pub fn read_pcd_header<P: AsRef<Path>>(p: P) -> Result<PCDHeader> {
    let file = File::open(p).map_err(PCDReadError::IOError)?;
    let reader = BufReader::new(file);
    Parser::new(reader).parse_header()
}

/// Parses a [PointCloudData] from the reader
/// ```no_run
/// use vivotk::pcd::{PCDReadError, read_pcd};
///
/// fn main() -> Result<(), PCDReadError> {
///     let reader_pcd = read_pcd("VERSION .7 ...".as_bytes())?;
///     println!("{}", reader_pcd.data().len());
///     Ok(())
/// }
/// ```
pub fn read_pcd<R: Read>(r: R) -> Result<PointCloudData> {
    let reader = BufReader::new(r);
    Parser::new(reader).parse()
}

/// Represents possible error scenarios when attempting to parse a point cloud data format file.
#[derive(Error, Debug)]
pub enum PCDReadError {
    /// For ease of conversion from IO errors to PCDReadError.
    /// Note that error can still be due to an invalid encoding of PCD
    /// E.g. attempting to access the next line when there are no more lines.
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    /// Represents an error with the header type of the file.
    #[error("Invalid header while parsing {section:?}. {error_msg:?}\n\t{actual_line:?}")]
    InvalidHeader {
        /// The portion of the header where the error is encountered
        section: String,
        /// A custom error messaging describing the error
        error_msg: String,
        /// The line which caused the error
        actual_line: String,
    },
    /// Represents an error with the data of the file.
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

struct Parser<R: BufRead> {
    reader: R,
    line: String,
}

impl<R: BufRead> Parser<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            line: String::new(),
        }
    }

    fn parse(mut self) -> Result<PointCloudData> {
        let header = self.parse_header()?;
        self.parse_data(header)
    }

    fn parse_header(&mut self) -> Result<PCDHeader> {
        let version = self.parse_header_version()?;
        let fields = self.parse_fields()?;
        let (width, height) = self.parse_width_and_height()?;
        let viewpoint = self.parse_viewpoint()?;
        let points = self.parse_points()?;
        let data_type = self.parse_data_type()?;

        PCDHeader::new(version, fields, width, height, viewpoint, points, data_type)
            .map_err(|s| self.header_err("", s))
    }

    fn parse_data_type(&mut self) -> Result<PCDDataType> {
        self.next_line()?;
        self.strip_line_prefix("DATA")?
            .parse::<PCDDataType>()
            .map_err(|e| self.header_err("DATA", e))
    }

    fn parse_header_version(&mut self) -> Result<PCDVersion> {
        self.next_line()?;
        let version_str = self.strip_line_prefix("VERSION")?;
        match PCDVersion::from_str(version_str) {
            Ok(version) => match version {
                PCDVersion::V0_6 => {
                    Err(self.header_err("VERSION", "Version 0.6 is not supported".to_string()))
                }
                PCDVersion::V0_7 => Ok(PCDVersion::V0_7),
            },
            Err(s) => Err(self.header_err("VERSION", s)),
        }
    }

    fn parse_fields(&mut self) -> Result<Vec<PCDField>> {
        let names = self.parse_vec("FIELDS")?;

        let sizes = self.parse_vec("SIZE")?;
        if sizes.len() != names.len() {
            return Err(self.header_err(
                "SIZE",
                format!("Expected length {}, got {}", names.len(), sizes.len()),
            ));
        }

        let types = self.parse_vec("TYPE")?;
        if types.len() != names.len() {
            return Err(self.header_err(
                "TYPE",
                format!("Expected length {}, got {}", names.len(), types.len()),
            ));
        }

        let counts = self.parse_vec("COUNT")?;
        if counts.len() != names.len() {
            return Err(self.header_err(
                "COUNT",
                format!("Expected length {}, got {}", names.len(), counts.len()),
            ));
        }

        let mut fields = Vec::with_capacity(names.len());
        for (i, name) in names.into_iter().enumerate() {
            let field = PCDField::new(name, sizes[i], types[i], counts[i])
                .map_err(|s| self.header_err("", s))?;
            fields.push(field);
        }

        Ok(fields)
    }

    fn parse_vec<T>(&mut self, prefix: &str) -> Result<Vec<T>>
    where
        T: FromStr,
        <T as FromStr>::Err: Debug,
    {
        self.next_line()?;
        self.strip_line_prefix(prefix)?
            .split_whitespace()
            .map(|s| s.parse::<T>())
            .collect::<std::result::Result<Vec<T>, T::Err>>()
            .map_err(|e| self.header_err(prefix, format!("{e:?}")))
    }

    fn parse_width_and_height(&mut self) -> Result<(u64, u64)> {
        self.next_line()?;
        let width = self
            .strip_line_prefix("WIDTH")?
            .parse::<u64>()
            .map_err(|e| self.header_err("WIDTH", e.to_string()))?;

        self.next_line()?;
        let height = self
            .strip_line_prefix("HEIGHT")?
            .parse::<u64>()
            .map_err(|e| self.header_err("HEIGHT", e.to_string()))?;

        Ok((width, height))
    }

    fn parse_viewpoint(&mut self) -> Result<[f32; 7]> {
        self.parse_vec::<f32>("VIEWPOINT")?
            .try_into()
            .map_err(|v: Vec<f32>| {
                self.header_err("VIEWPOINT", format!("Expected length 7, got {}", v.len()))
            })
    }

    fn parse_points(&mut self) -> Result<u64> {
        self.next_line()?;
        self.strip_line_prefix("POINTS")?
            .parse::<u64>()
            .map_err(|e| self.header_err("POINTS", e.to_string()))
    }

    fn parse_data(self, header: PCDHeader) -> Result<PointCloudData> {
        let data_type = header.data_type();

        match data_type {
            PCDDataType::Ascii => self.parse_ascii_data(header),
            PCDDataType::Binary => self.parse_binary_data(header),
            PCDDataType::CompressedBinary => {
                Err(self.header_err("DATA", "Compressed binary type not supported".to_string()))
            }
        }
    }

    fn parse_ascii_data(self, header: PCDHeader) -> Result<PointCloudData> {
        use byteorder::{NativeEndian, WriteBytesExt};

        let size = header.buffer_size();
        let mut buffer = Vec::with_capacity(size as usize);

        let data_per_line = header.data_per_line();

        for line in self.reader.lines() {
            // Should only read the number of points specified in the header
            if buffer.len() >= buffer.capacity() {
                break;
            }
            let line = line.map_err(PCDReadError::IOError)?;
            let data = line.split_whitespace().collect::<Vec<&str>>();
            if data.len() as u64 != data_per_line {
                return Err(PCDReadError::InvalidData(format!(
                    "Expected {} data points, got {}.\nLine: {}",
                    data_per_line,
                    data.len(),
                    line
                )));
            }

            use PCDFieldDataType::*;
            use PCDReadError::InvalidData;
            let mut index = 0;
            for field in header.fields() {
                for _ in 0..field.count() {
                    match field.data_type() {
                        U8 => buffer.write_u8(
                            data[index]
                                .parse::<u8>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        I8 => buffer.write_i8(
                            data[index]
                                .parse::<i8>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        U16 => buffer.write_u16::<NativeEndian>(
                            data[index]
                                .parse::<u16>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        I16 => buffer.write_i16::<NativeEndian>(
                            data[index]
                                .parse::<i16>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        U32 => buffer.write_u32::<NativeEndian>(
                            data[index]
                                .parse::<u32>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        I32 => buffer.write_i32::<NativeEndian>(
                            data[index]
                                .parse::<i32>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        F32 => buffer.write_f32::<NativeEndian>(
                            data[index]
                                .parse::<f32>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                        F64 => buffer.write_f64::<NativeEndian>(
                            data[index]
                                .parse::<f64>()
                                .map_err(|e| InvalidData(e.to_string()))?,
                        ),
                    }
                    .unwrap();
                    index += 1;
                }
            }
        }

        PointCloudData::new(header, buffer).map_err(PCDReadError::InvalidData)
    }

    fn parse_binary_data(mut self, header: PCDHeader) -> Result<PointCloudData> {
        let mut buffer = vec![0; header.buffer_size() as usize];
        self.reader
            .read_exact(&mut buffer)
            .map_err(PCDReadError::IOError)?;
        PointCloudData::new(header, buffer).map_err(PCDReadError::InvalidData)
    }

    fn strip_line_prefix(&mut self, prefix: &str) -> Result<&str> {
        self.line
            .trim()
            .strip_prefix(prefix)
            .ok_or_else(|| self.header_err(prefix, format!("Expected line to start with {prefix}")))
            .map(|s| s.trim())
    }

    fn next_line(&mut self) -> Result<()> {
        self.line.clear();
        self.reader
            .read_line(&mut self.line)
            .map_err(PCDReadError::IOError)?;
        while self.line.starts_with('#') || self.line.is_empty() {
            self.line.clear();
            self.reader
                .read_line(&mut self.line)
                .map_err(PCDReadError::IOError)?;
        }
        Ok(())
    }

    fn header_err(&self, section: &str, error_msg: String) -> PCDReadError {
        PCDReadError::InvalidHeader {
            section: section.to_string(),
            error_msg,
            actual_line: self.line.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pcd::data_types::PCDVersion;
    use crate::pcd::reader::{PCDReadError, Parser};
    use crate::pcd::{read_pcd_file, PCDField, PCDFieldSize, PCDFieldType, PCDHeader};
    use byteorder::{NativeEndian, ReadBytesExt};
    use std::io::{BufReader, Cursor};

    fn expected_header() -> PCDHeader {
        PCDHeader::new(
            PCDVersion::V0_7,
            vec![
                PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new(
                    "rgb".to_string(),
                    PCDFieldSize::Four,
                    PCDFieldType::Float,
                    1,
                )
                .unwrap(),
            ],
            213,
            1,
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            213,
            crate::pcd::PCDDataType::Ascii,
        )
        .unwrap()
    }

    fn parse_str(s: &str) -> Parser<BufReader<&[u8]>> {
        Parser::new(BufReader::new(s.as_bytes()))
    }

    fn assert_header_fail<T>(result: Result<T, PCDReadError>, fail_section: &str) {
        match result {
            Ok(_) => panic!("Parsing should fail"),
            Err(e) => match e {
                PCDReadError::InvalidHeader { section, .. } => assert_eq!(&section, fail_section),
                _ => panic!("Error should be due to {fail_section}"),
            },
        }
    }

    #[test]
    fn parse_version_success() {
        let versions = ["VERSION .7", "VERSION 0.7"];

        for version in versions {
            let mut parser = parse_str(version);
            let header = parser.parse_header_version().unwrap();
            assert_eq!(header, PCDVersion::V0_7);
        }
    }

    #[test]
    fn parse_version_fail() {
        let versions = [
            "VERSION .6",
            "VERSION 0.6",
            "Version .6",
            "VERSION 6",
            "VERSION 7",
            "ImNotEvenAVersion 0.6",
        ];

        for version in versions {
            let mut parser = parse_str(version);
            let header = parser.parse_header_version();
            assert_header_fail(header, "VERSION");
        }
    }

    #[test]
    fn parse_fields_success() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 1 2 4 8\n\
             TYPE I U U F\n\
             COUNT 1 2 3 4";

        let expected = [
            PCDField::new("x".to_string(), PCDFieldSize::One, PCDFieldType::Signed, 1).unwrap(),
            PCDField::new(
                "y".to_string(),
                PCDFieldSize::Two,
                PCDFieldType::Unsigned,
                2,
            )
            .unwrap(),
            PCDField::new(
                "z".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Unsigned,
                3,
            )
            .unwrap(),
            PCDField::new(
                "rgba".to_string(),
                PCDFieldSize::Eight,
                PCDFieldType::Float,
                4,
            )
            .unwrap(),
        ];

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields().unwrap();
        assert_eq!(fields, expected);
    }

    #[test]
    fn parse_fields_failure() {
        let fields = "NOTFIELD x y z rgba\n\
             SIZE 1 2 4 8\n\
             TYPE I U U F\n\
             COUNT 1 2 3 4";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "FIELDS");
    }

    #[test]
    fn parse_fields_invalid_size() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 3 2 4 8\n\
             TYPE I U U F\n\
             COUNT 1 2 3 4";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "SIZE");
    }

    #[test]
    fn parse_fields_invalid_type() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 1 2 4 8\n\
             TYPE I A U F\n\
             COUNT 1 2 3 4";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "TYPE");
    }

    #[test]
    fn parse_fields_invalid_size_length() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 1 2 4 8 1\n\
             TYPE I U U F\n\
             COUNT 1 2 3 4";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "SIZE");
    }

    #[test]
    fn parse_fields_invalid_type_length() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 1 2 4 8\n\
             TYPE I U U F I\n\
             COUNT 1 2 3 4";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "TYPE");
    }

    #[test]
    fn parse_fields_invalid_count_length() {
        let fields = "FIELDS x y z rgba\n\
             SIZE 1 2 4 8\n\
             TYPE I U U F\n\
             COUNT 1 2 3 4 1";

        let mut parser = parse_str(fields);
        let fields = parser.parse_fields();
        assert_header_fail(fields, "COUNT");
    }

    #[test]
    fn parse_fields_invalid_size_and_type() {
        let fields_to_test = [
            "FIELDS x\nSIZE 1\nTYPE F\nCOUNT 1",
            "FIELDS x\nSIZE 2\nTYPE F\nCOUNT 1",
            "FIELDS x\nSIZE 8\nTYPE U\nCOUNT 1",
            "FIELDS x\nSIZE 8\nTYPE I\nCOUNT 1",
        ];

        for fields in fields_to_test {
            let mut parser = parse_str(fields);
            let fields = parser.parse_fields();
            assert_header_fail(fields, "");
        }
    }

    #[test]
    fn parse_height_and_width_success() {
        let width_height_str = "WIDTH 640\nHEIGHT 320";
        let mut parser = parse_str(width_height_str);
        let width_height = parser.parse_width_and_height().unwrap();
        assert_eq!(width_height, (640, 320));
    }

    #[test]
    fn parse_width_failure() {
        let width_height_str_to_test = [
            "NOTWIDTH 640\nHEIGHT 320",
            "WIDTH 6.40\nHEIGHT 320",
            "WIDTH abc\nHEIGHT 320",
        ];

        for width_height_str in width_height_str_to_test {
            let mut parser = parse_str(width_height_str);
            let width_height = parser.parse_width_and_height();
            assert_header_fail(width_height, "WIDTH");
        }
    }

    #[test]
    fn parse_height_failure() {
        let width_height_str_to_test = [
            "WIDTH 640\nNOTHEIGHT 320",
            "WIDTH 640\nHEIGHT 3.20",
            "WIDTH 640\nHEIGHT abc",
        ];

        for width_height_str in width_height_str_to_test {
            let mut parser = parse_str(width_height_str);
            let width_height = parser.parse_width_and_height();
            assert_header_fail(width_height, "HEIGHT");
        }
    }

    #[test]
    fn parse_viewpoint_success() {
        let viewpoint_str = "VIEWPOINT 0.1 0 0 1 0 0 0";
        let mut parser = parse_str(viewpoint_str);
        let viewpoint = parser.parse_viewpoint().unwrap();
        assert_eq!(viewpoint, [0.1, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn parse_viewpoint_failure() {
        let viewpoint_str_to_test = [
            "NOTVIEWPOINT 0.1 0 0 1 0 0 0",
            "VIEWPOINT a 0 0 1 0 0 0",
            "VIEWPOINT 0.1 0 0 1 0 0 0 1",
        ];

        for viewpoint_str in viewpoint_str_to_test {
            let mut parser = parse_str(viewpoint_str);
            let viewpoint = parser.parse_viewpoint();
            assert_header_fail(viewpoint, "VIEWPOINT");
        }
    }

    #[test]
    fn parse_points_success() {
        let points_str = "POINTS 307200";
        let mut parser = parse_str(points_str);
        let points = parser.parse_points().unwrap();
        assert_eq!(points, 307200);
    }

    #[test]
    fn parse_points_failure() {
        let points_str_to_test = ["NOTPOINTS 307200", "POINTS 30.2", "POINTS abc"];

        for points_str in points_str_to_test {
            let mut parser = parse_str(points_str);
            let points = parser.parse_points();
            assert_header_fail(points, "POINTS")
        }
    }

    #[test]
    fn parse_header_success() {
        let header_str = "VERSION .7 \n\
               FIELDS x y z rgb \n\
               SIZE 4 4 4 4 \n\
               TYPE F F F F \n\
               COUNT 1 1 1 1 \n\
               WIDTH 213 \n\
               HEIGHT 1 \n\
               VIEWPOINT 0 0 0 1 0 0 0 \n\
               POINTS 213 \n\
               DATA ascii \n\
        ";

        let mut parser = Parser::new(BufReader::new(header_str.as_bytes()));
        let header = parser.parse_header().unwrap();
        assert_eq!(header, expected_header());
    }

    #[test]
    fn parse_header_with_comments_success() {
        let header_str = "# This is point cloud file \n\
               VERSION .7 \n\
               # I am another comment\n\
               FIELDS x y z rgb \n\
               # I am another comment\n\
               # I am another comment\n\
               SIZE 4 4 4 4 \n\
               # I am another comment\n\
               TYPE F F F F \n\
               # I am another comment\n\
               COUNT 1 1 1 1 \n\
               # I am another comment\n\
               WIDTH 213 \n\
               # I am another comment\n\
               HEIGHT 1 \n\
               # I am another comment\n\
               VIEWPOINT 0 0 0 1 0 0 0 \n\
               # I am another comment\n\
               POINTS 213 \n\
               # I am another comment\n\
               DATA ascii \n\
               # I am another comment\n\
        ";

        let mut parser = Parser::new(BufReader::new(header_str.as_bytes()));
        let header = parser.parse_header().unwrap();
        assert_eq!(header, expected_header());
    }

    #[test]
    fn parse_ascii_success() {
        let pcd = read_pcd_file("test_files/pcd/ascii.pcd").unwrap();
        assert_eq!(pcd.header(), &expected_header());
        let mut rdr = Cursor::new(pcd.data());

        // Just read first 3 lines
        let expected = [
            0.93773, 0.33763, 0.0, 4.2108e+06, 0.90805, 0.35641, 0.0, 4.2108e+06, 0.81915, 0.32,
            0.0, 4.2108e+06,
        ];

        for val in expected {
            assert_eq!(rdr.read_f32::<NativeEndian>().unwrap(), val);
        }
    }
}
