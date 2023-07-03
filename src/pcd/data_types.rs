use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

use serde::Serialize;

use crate::formats::PointCloud;

/// This struct represents a single .pcd file
pub struct PointCloudData {
    pub(crate) header: PCDHeader,
    pub(crate) data: Vec<u8>,
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
            Ok(Self { header, data })
        }
    }

    pub fn header(&self) -> &PCDHeader {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl<T> From<&PointCloud<T>> for PointCloudData
where
    T: Clone + Serialize,
{
    fn from(point_cloud: &PointCloud<T>) -> Self {
        let header = PCDHeader::new(
            PCDVersion::V0_7,
            vec![
                PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
                PCDField::new(
                    "rgb".to_string(),
                    PCDFieldSize::Four,
                    PCDFieldType::Unsigned,
                    1,
                )
                .unwrap(),
            ],
            point_cloud.number_of_points as u64,
            1,
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            point_cloud.number_of_points as u64,
            PCDDataType::Binary,
        )
        .unwrap();
        // bincode makes use of serde to serialize the data into bytes
        let bytes = bincode::serialize(&point_cloud.points).unwrap();
        PointCloudData::new(header, bytes[8..].into()).unwrap()
    }
}

impl Clone for PointCloudData {
    fn clone(&self) -> Self {
        Self {
            header: self.header.clone(),
            data: self.data.clone(),
        }
    }
}

impl Debug for PointCloudData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PointCloudData: {:?}", self.header)
    }
}

/// Header information for the PCD file
#[derive(Debug, Clone, PartialEq)]
pub struct PCDHeader {
    version: PCDVersion,
    fields: Vec<PCDField>,
    width: u64,
    height: u64,
    viewpoint: [f32; 7],
    points: u64,
    data_type: PCDDataType,
}

impl PCDHeader {
    pub fn new(
        version: PCDVersion,
        fields: Vec<PCDField>,
        width: u64,
        height: u64,
        viewpoint: [f32; 7],
        points: u64,
        data_type: PCDDataType,
    ) -> Result<Self, String> {
        if width.saturating_mul(height) != points {
            return Err(format!("Width * Height must be equal to number of points. Width: {width} Height: {height} Points: {points}"));
        }

        Ok(Self {
            version,
            fields,
            width,
            height,
            viewpoint,
            points,
            data_type,
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

    pub fn data_type(&self) -> PCDDataType {
        self.data_type
    }

    /// Calculates the number of bytes that should be present in the
    /// data portion of the point cloud.
    pub fn buffer_size(&self) -> u64 {
        let mut size_per_point = 0;
        for field in &self.fields {
            let field_size = field.size() as u64;
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

/// Version of the PCD file format
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

/// The information for each dimension of the point
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PCDField {
    name: String,
    data_type: PCDFieldDataType,
    count: u64,
}

impl PCDField {
    pub fn new(
        name: String,
        size: PCDFieldSize,
        field_type: PCDFieldType,
        count: u64,
    ) -> Result<Self, String> {
        let data_type = (size, field_type).try_into()?;
        Ok(Self {
            name,
            data_type,
            count,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data_type(&self) -> PCDFieldDataType {
        self.data_type
    }

    pub fn size(&self) -> u8 {
        match self.data_type {
            PCDFieldDataType::U8 => 1,
            PCDFieldDataType::I8 => 1,
            PCDFieldDataType::U16 => 2,
            PCDFieldDataType::I16 => 2,
            PCDFieldDataType::U32 => 4,
            PCDFieldDataType::I32 => 4,
            PCDFieldDataType::F32 => 4,
            PCDFieldDataType::F64 => 8,
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

/// A valid combination of the [PCDFieldType] and [PCDFieldSize]
///
/// Certain combinations have size and field types have no valid representation and
/// this type guarantees that the point cloud data must contain a valid combination.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PCDFieldDataType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    F64,
}

impl TryFrom<(PCDFieldSize, PCDFieldType)> for PCDFieldDataType {
    type Error = String;

    fn try_from((size, field_type): (PCDFieldSize, PCDFieldType)) -> Result<Self, Self::Error> {
        use PCDFieldSize::*;
        use PCDFieldType::*;

        match (size, field_type) {
            (One, Signed) => Ok(Self::I8),
            (One, Unsigned) => Ok(Self::U8),
            (Two, Signed) => Ok(Self::I16),
            (Two, Unsigned) => Ok(Self::U16),
            (Four, Signed) => Ok(Self::I32),
            (Four, Unsigned) => Ok(Self::U32),
            (Four, Float) => Ok(Self::F32),
            (Eight, Float) => Ok(Self::F64),
            _ => Err(format!(
                "Field combination of size: {size:?} and type: {field_type:?} not supported."
            )),
        }
    }
}

impl From<PCDFieldDataType> for PCDFieldSize {
    fn from(data: PCDFieldDataType) -> Self {
        match data {
            PCDFieldDataType::U8 => Self::One,
            PCDFieldDataType::I8 => Self::One,
            PCDFieldDataType::U16 => Self::Two,
            PCDFieldDataType::I16 => Self::Two,
            PCDFieldDataType::U32 => Self::Four,
            PCDFieldDataType::I32 => Self::Four,
            PCDFieldDataType::F32 => Self::Four,
            PCDFieldDataType::F64 => Self::Eight,
        }
    }
}

impl From<PCDFieldDataType> for PCDFieldType {
    fn from(data: PCDFieldDataType) -> Self {
        match data {
            PCDFieldDataType::U8 => Self::Unsigned,
            PCDFieldDataType::I8 => Self::Signed,
            PCDFieldDataType::U16 => Self::Unsigned,
            PCDFieldDataType::I16 => Self::Signed,
            PCDFieldDataType::U32 => Self::Unsigned,
            PCDFieldDataType::I32 => Self::Signed,
            PCDFieldDataType::F32 => Self::Float,
            PCDFieldDataType::F64 => Self::Float,
        }
    }
}

/// The size in bytes of the dimension of the field
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

/// The type of the dimension of the field
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

/// The storage format of the point cloud data file
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
