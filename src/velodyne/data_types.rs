use std::fmt::{Debug, Formatter};

#[derive(Debug, Clone, Copy)]
pub struct VelodynPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub intensity: f32,
}

impl VelodynPoint {
    pub fn new(x: f32, y: f32, z: f32, intensity: f32) -> Self {
        Self { x, y, z, intensity }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() == 16);
        let x = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let y = f32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let z = f32::from_ne_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let intensity = f32::from_ne_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert!(intensity >= 0f32 && intensity <= 255f32);
        let intensity = if intensity > 1.0 {
            intensity / 255.0
        } else {
            intensity
        };
        Self { x, y, z, intensity }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.x.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.y.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.z.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.intensity.to_ne_bytes());
        bytes
    }
}

/// This struct represents a single velodyne's bin file.
pub struct VelodyneBinData {
    pub(crate) data: Vec<VelodynPoint>,
}

impl VelodyneBinData {
    pub fn new(data: Vec<VelodynPoint>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &Vec<VelodynPoint> {
        &self.data
    }
}

impl Debug for VelodyneBinData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "VelodyneBinData length: {}", self.data.len())?;
        // write 3 points
        for i in 0..3 {
            write!(f, "\n{:?}", self.data[i])?;
        }
        write!(f, "\n")?;
        Ok(())
    }
}
