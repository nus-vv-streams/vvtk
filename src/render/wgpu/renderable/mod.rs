use wgpu::util::DeviceExt;
use wgpu::CompareFunction::Less;
use wgpu::{
    include_wgsl, DepthStencilState, Device, Extent3d, PipelineLayout, RenderPipeline,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, VertexBufferLayout,
};
use winit::dpi::PhysicalSize;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use super::antialias::AntiAlias;

pub trait Renderable: Clone {
    /// Defines how a buffer is represented in memory.
    /// This is used by render_pipeline to map the buffer in the shader
    fn buffer_layout_desc<'a>() -> wgpu::VertexBufferLayout<'a>;
    fn create_render_pipeline(
        device: &Device,
        format: TextureFormat,
        layout: Option<&wgpu::PipelineLayout>,
    ) -> RenderPipeline;
    fn create_depth_texture(
        device: &Device,
        size: PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.width,
                height: size.height,
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
    /// Create buffer that will be used to store the data
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
    fn num_vertices(&self) -> usize;
}

impl Renderable for PointCloud<PointXyzRgba> {
    fn buffer_layout_desc<'a>() -> VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            // how wide a vertex is
            array_stride: 16,
            // whether each element of this buffer represents per-vertex or per-instance data
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }

    fn create_render_pipeline(
        device: &Device,
        format: TextureFormat,
        layout: Option<&PipelineLayout>,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(include_wgsl!("./pointxyzrgba.wgsl"));

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                // type of vertices to pass to the vertex shader
                buffers: &[Self::buffer_layout_desc()],
            },
            // we need fragment shader to store color data to the surface
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    // write to all colors: red, blue, green, alpha
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            // how to interpret our vertices when converting into triangles
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                depth_write_enabled: true,
                depth_compare: Less,
                stencil: Default::default(),
                format: TextureFormat::Depth32Float,
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                // how many samples the pipeline will use.
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        })
    }

    fn antialias(&self) -> AntiAlias {
        let first_point = self.points.get(0).unwrap();
        let mut max_x = first_point.x;
        let mut max_y = first_point.y;
        let mut max_z = first_point.z;
        let mut min_x = first_point.x;
        let mut min_y = first_point.y;
        let mut min_z = first_point.z;

        for point in &self.points {
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
            max_z = max_z.max(point.z);
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            min_z = min_z.min(point.z);
        }
        let mut max = (max_x - min_x).max(max_y - min_y).max(max_z - min_z);
        if max == 0.0 {
            max = 1.0
        }
        AntiAlias {
            x: (max_x - min_x) / 2.0,
            y: (max_y - min_y) / 2.0,
            z: (max_z - min_z) / 2.0,
            scale: max,
        }
    }

    fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.points)
    }

    fn num_vertices(&self) -> usize {
        self.number_of_points
    }

    fn create_depth_texture(
        device: &Device,
        size: PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.width,
                height: size.height,
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
}
