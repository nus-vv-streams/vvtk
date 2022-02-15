use std::convert::TryFrom;
use std::str::FromStr;

/// This struct represents a single .pcd file
pub struct PointCloudData {
    header: PCDHeader,
    data: Vec<u8>,
}

impl PointCloudData {
    pub fn new(header: PCDHeader, data: Vec<u8>) -> Result<Self, String> {
        if header.buffer_size() != data.len() as u64 {
            Err(format!(
                "Expected {} bytes from header data, got {} instead",
                header.buffer_size(),
                data.len()
            ))
        } else {
            Ok(Self {
                header,
                data
            })
        }
    }

    pub fn header(&self) -> &PCDHeader {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PCDHeader {
    version: PCDVersion,
    fields: Vec<PCDField>,
    width: u64,
    height: u64,
    viewpoint: [f32; 7],
    points: u64,
}

impl PCDHeader {
    pub fn new(version: PCDVersion, fields: Vec<PCDField>, width: u64, height: u64, viewpoint: [f32; 7], points: u64) -> Result<Self, String> {
        if width.saturating_mul(height) != points {
            return Err(format!("Width * Height must be equal to number of points. Width: {width} Height: {height} Points: {points}"));
        }

        Ok(Self {
            version,
            fields,
            width,
            height,
            viewpoint,
            points
        })
    }

    pub fn version(&self) -> PCDVersion {
        self.version
    }

    pub fn fields(&self) -> &Vec<PCDField> {
        &self.fields
    }

    pub fn width(&self) -> u64 {
        self.width
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn viewpoint(&self) -> &[f32; 7] {
        &self.viewpoint
    }

    pub fn points(&self) -> u64 {
        self.points
    }

    /// Calculates the number of bytes that should be present in the
    /// data portion of the point cloud.
    pub fn buffer_size(&self) -> u64 {
        let mut size_per_point = 0;
        for field in &self.fields {
            let field_size = u8::from(field.size) as u64;
            size_per_point += field_size * field.count;
        }

        size_per_point * self.points
    }

    /// Calculates the number of data points that should be present per line
    /// in "ascii" format.
    ///
    /// Example: Given the following field section:
    ///     FIELDS x y z rgb
    ///     ...
    ///     COUNT 1 2 3 4
    ///
    /// We should expect to see 1 + 2 + 3 + 4 = 10 data points per line

    pub fn data_per_line(&self) -> u64 {
        self.fields.iter().fold(0, |acc, field| acc + field.count)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PCDVersion {
    V0_6,
    V0_7,
}

impl ToString for PCDVersion {
    fn to_string(&self) -> String {
        match self {
            Self::V0_6 => ".6",
            Self::V0_7 => ".7",
        }
        .to_string()
    }
}

impl FromStr for PCDVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0.6" | ".6" => Ok(Self::V0_6),
            "0.7" | ".7" => Ok(Self::V0_7),
            _ => Err(format!("Version type {s} not supported!")),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PCDField {
    name: String,
    size: PCDFieldSize,
    field_type: PCDFieldType,
    count: u64,
}

impl PCDField {
    pub fn new(
        name: String,
        size: PCDFieldSize,
        field_type: PCDFieldType,
        count: u64,
    ) -> Result<Self, String> {
        use PCDFieldSize::*;
        use PCDFieldType::*;

        match (size, field_type) {
            (One, Signed) | (One, Unsigned) => Ok(()),
            (Two, Signed) | (Two, Unsigned) => Ok(()),
            (Four, Signed) | (Four, Unsigned) | (Four, Float) => Ok(()),
            (Eight, Float) => Ok(()),
            _ => Err(format!(
                "Field combination of size: {size:?} and type: {field_type:?} not supported."
            )),
        }
        .map(|_| Self {
            name,
            size,
            field_type,
            count
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> PCDFieldSize {
        self.size
    }

    pub fn field_type(&self) -> PCDFieldType {
        self.field_type
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PCDFieldSize {
    One,
    Two,
    Four,
    Eight,
}

impl ToString for PCDFieldSize {
    fn to_string(&self) -> String {
        match self {
            Self::One => "1",
            Self::Two => "2",
            Self::Four => "4",
            Self::Eight => "8",
        }
        .to_string()
    }
}

impl FromStr for PCDFieldSize {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::One),
            "2" => Ok(Self::Two),
            "4" => Ok(Self::Four),
            "8" => Ok(Self::Eight),
            _ => Err(format!(
                "Only field sizes of 1, 2, 4, 8 are supported. Got: {s}"
            )),
        }
    }
}

impl TryFrom<u8> for PCDFieldSize {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            4 => Ok(Self::Four),
            8 => Ok(Self::Eight),
            _ => Err(format!(
                "Only field sizes of 1, 2, 4, 8 are supported. Got: {value}"
            )),
        }
    }
}

impl From<PCDFieldSize> for u8 {
    fn from(field_size: PCDFieldSize) -> Self {
        match field_size {
            PCDFieldSize::One => 1,
            PCDFieldSize::Two => 2,
            PCDFieldSize::Four => 4,
            PCDFieldSize::Eight => 8,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PCDFieldType {
    Signed,
    Unsigned,
    Float,
}

impl ToString for PCDFieldType {
    fn to_string(&self) -> String {
        match self {
            Self::Signed => "I",
            Self::Unsigned => "U",
            Self::Float => "F",
        }
        .to_string()
    }
}

impl FromStr for PCDFieldType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "I" => Ok(Self::Signed),
            "U" => Ok(Self::Unsigned),
            "F" => Ok(Self::Float),
            _ => Err(format!("Unknown field type {s}")),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PCDDataType {
    Ascii,
    Binary,
    CompressedBinary,
}

impl ToString for PCDDataType {
    fn to_string(&self) -> String {
        match self {
            Self::Ascii => "ascii",
            Self::Binary => "binary",
            Self::CompressedBinary => "compressed_binary",
        }
        .to_string()
    }
}

impl FromStr for PCDDataType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ascii" => Ok(Self::Ascii),
            "binary" => Ok(Self::Binary),
            "compressed_binary" => Ok(Self::CompressedBinary),
            _ => Err(format!("Unknown data type: {s}")),
        }
    }
}
