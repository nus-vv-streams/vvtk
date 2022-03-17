use wgpu::{Extent3d, RenderPipeline, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use wgpu::util::DeviceExt;
use crate::render::wgpu::antialias::AntiAlias;
use crate::render::wgpu::gpu::Gpu;

pub mod pcd;

pub trait Renderable: Clone {
    fn buffer_layout_desc<'a>() -> wgpu::VertexBufferLayout<'a>;
    fn create_render_pipeline(device: &Gpu, layout: Option<&wgpu::PipelineLayout>) -> RenderPipeline;
    fn create_depth_texture(gpu: &Gpu) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = gpu.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: gpu.size.width,
                height: gpu.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (depth_texture, depth_view)
    }
    fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: self.bytes(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }
    fn antialias(&self) -> AntiAlias {
        AntiAlias::default()
    }
    fn bytes(&self) -> &[u8];
    fn vertices(&self) -> usize;
}