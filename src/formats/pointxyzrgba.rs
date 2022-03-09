#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PointXyzRgba {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}