use wgpu::{DepthStencilState, include_wgsl, PipelineLayout, RenderPipeline, TextureFormat, VertexBufferLayout};
use wgpu::CompareFunction::Less;
use crate::pcd::PointCloudData;
use crate::render::wgpu::gpu::Gpu;
use crate::render::wgpu::renderable::Renderable;

impl Renderable for PointCloudData {
    fn buffer_layout_desc<'a>() -> VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: 16,
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

    fn create_render_pipeline(gpu: &Gpu, layout: Option<&PipelineLayout>) -> RenderPipeline {
        let shader = gpu.device.create_shader_module(&include_wgsl!("../shaders/pcd.wgsl"));

        gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Self::buffer_layout_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
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
                bias: Default::default()
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        })
    }

    fn bytes(&self) -> &[u8] {
        self.data()
    }

    fn vertices(&self) -> usize {
        self.data().len() / 16
    }

}
