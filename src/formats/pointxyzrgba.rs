use std::cmp::Ordering;

use serde::ser::{Serialize, SerializeStruct, Serializer};
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointXyzRgba {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PartialOrd for PointXyzRgba {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PointXyzRgba {
    fn cmp(&self, other: &Self) -> Ordering {
        self.x
            .total_cmp(&other.x)
            .then_with(|| self.y.total_cmp(&other.y))
            .then_with(|| self.z.total_cmp(&other.z))
    }
}

impl Eq for PointXyzRgba {}

impl Serialize for PointXyzRgba {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PointXyzRgba", 16)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.serialize_field("z", &self.z)?;
        #[cfg(target_endian = "little")]
        {
            state.serialize_field("b", &self.b)?;
            state.serialize_field("g", &self.g)?;
            state.serialize_field("r", &self.r)?;
            state.serialize_field("a", &self.a)?;
        }
        #[cfg(target_endian = "big")]
        {
            state.serialize_field("r", &self.r)?;
            state.serialize_field("g", &self.g)?;
            state.serialize_field("b", &self.b)?;
            state.serialize_field("a", &self.a)?;
        }
        state.end()
    }
}
