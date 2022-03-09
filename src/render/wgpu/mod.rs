pub mod gpu;
pub mod renderer;
pub mod camera;
pub mod reader;
pub mod builder;
pub mod renderable;
pub mod controls;


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AntiAlias {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    _padding: f32
}

impl AntiAlias {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, _padding: 1.0 }
    }
}