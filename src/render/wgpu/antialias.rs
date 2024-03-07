use crate::formats::pointxyzrgba::PointXyzRgba;

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AntiAlias {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub scale: f32,
}

impl Default for AntiAlias {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            scale: 1.0,
        }
    }
}

/// Note: this is actually not anti-aliasing
impl AntiAlias {
    pub fn create_buffer(&self, device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let antialias_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Antialias Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let antialias_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("antialias_bind_group_layout"),
            });

        let antialias_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &antialias_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: antialias_buffer.as_entire_binding(),
            }],
            label: Some("antialias_bind_group"),
        });

        (antialias_bind_group_layout, antialias_bind_group)
    }

    pub fn apply(&self, points: &Vec<PointXyzRgba>) -> Vec<PointXyzRgba> {
        let mut result = Vec::with_capacity(points.len());

        for p in points.iter() {
            let x = (p.x - self.x) / self.scale;
            let y = (p.y - self.y) / self.scale;
            let z = (p.z - self.z) / self.scale;

            result.push(PointXyzRgba { x, y, z, ..*p });
        }

        result
    }

    pub fn apply_single(&self, point: &[f32; 3]) -> [f32; 3] {
        let x = (point[0] - self.x) / self.scale;
        let y = (point[1] - self.y) / self.scale;
        let z = (point[2] - self.z) / self.scale;

        [x, y, z]
    }
}
