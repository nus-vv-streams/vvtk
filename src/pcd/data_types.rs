use std::convert::TryFrom;
use std::str::FromStr;

pub struct PointCloudData {
    pub header: PCDHeader,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PCDHeader {
    pub version: PCDVersion,
    pub fields: Vec<PCDField>,
    pub width: u64,
    pub height: u64,
    pub viewpoint: [f32; 7],
    pub points: u64,
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

#[allow(clippy::manual_non_exhaustive)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PCDField {
    pub name: String,
    pub size: PCDFieldSize,
    pub field_type: PCDFieldType,
    pub count: u64,
    // This fields ensures users cannot create PCDField directly
    _private: (),
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
            count,
            _private: (),
        })
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
