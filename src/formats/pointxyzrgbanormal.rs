#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PointXyzRgbaNormal {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    pub normal_x: f32,
    pub normal_y: f32,
    pub normal_z: f32,
}
