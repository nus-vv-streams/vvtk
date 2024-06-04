use crate::render::wgpu::camera::{CameraState, CameraUniform};
use std::marker::PhantomData;
use wgpu::{
    BindGroup, Buffer, CommandEncoder, Device, LoadOp, Operations, Queue,
    RenderPassDepthStencilAttachment, RenderPipeline, Texture, TextureFormat,
    TextureView,
};
use winit::dpi::PhysicalSize;
use super::renderable::Renderable;

pub struct PointCloudRenderer<T: Renderable> {
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    antialias_bind_group: BindGroup,
    depth_texture: Texture,
    depth_view: TextureView,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    num_vertices: usize,
    bg_color: wgpu::Color,
    _data: PhantomData<T>,
}

impl<T> PointCloudRenderer<T>
where
    T: Renderable,
{
    pub fn new(
        device: &Device,
        format: TextureFormat,
        initial_render: &T,
        initial_size: PhysicalSize<u32>,
        camera_state: &CameraState,
        bg_color: wgpu::Color,
    ) -> Self {
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) =
            camera_state.create_buffer(device);
        let (antialias_bind_group_layout, antialias_bind_group) =
            initial_render.antialias().create_buffer(device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &antialias_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline =
            T::create_render_pipeline(device, format, Some(&render_pipeline_layout));
        let (depth_texture, depth_view) = T::create_depth_texture(device, initial_size);

        let vertex_buffer = initial_render.create_buffer(device);
        let num_vertices = initial_render.num_vertices();

        Self {
            camera_buffer,
            camera_bind_group,
            antialias_bind_group,
            depth_texture,
            depth_view,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            bg_color,
            _data: PhantomData::default(),
        }
    }

    pub fn with_background_color(mut self, color: wgpu::Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>, device: &Device) {
        if new_size.width > 0 && new_size.height > 0 {
            let (depth_texture, depth_view) = T::create_depth_texture(device, new_size);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
        }
    }

    pub fn update_camera(&self, queue: &Queue, camera_uniform: CameraUniform) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    pub fn update_vertices(&mut self, device: &Device, queue: &Queue, data: &T) {
        let vertices = data.num_vertices();
        if vertices > self.num_vertices {
            self.vertex_buffer.destroy();
            self.vertex_buffer = data.create_buffer(device);
        } else {
            // print!("writing to buffer length: {}", data.bytes().len());
            queue.write_buffer(&self.vertex_buffer, 0, data.bytes());
        }
        self.num_vertices = vertices;
    }

    /// Stores render commands into encoder, specifying which texture to save the colors to.
    pub fn render(&mut self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                // which texture to save the colors to
                view,
                // the texture that will receive the resolved output. Same as `view` unless multisampling is enabled.
                // As we don't need to specify this, we leave it as None.
                resolve_target: None,
                ops: wgpu::Operations {
                    // `load` field tells wgpu how to handle colors stored from the previous frame.
                    // This will clear the screen with our background color.
                    load: wgpu::LoadOp::Clear(self.bg_color),
                    // This will clear the screen with a bluish color.
                    // load: wgpu::LoadOp::Clear(wgpu::Color {
                    //     r: self.bg_color.r / 255.0,
                    //    g: self.bg_color.g / 255.0,
                    //   b: self.bg_color.b / 255.0,
                    //  a: 1.0,
                    // }),
                    // true if we want to store the rendered results to the Texture behind our TextureView (in this case it's the SurfaceTexture).
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.antialias_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..(self.num_vertices as u32), 0..1);
    }
}
