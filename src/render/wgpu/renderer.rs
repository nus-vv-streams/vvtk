use crate::render::wgpu::builder::{
    Attachable, EventType, RenderEvent, RenderInformation, Windowed,
};
use crate::render::wgpu::camera::{Camera, CameraState, CameraUniform};
use crate::render::wgpu::gpu::WindowGpu;
use crate::render::wgpu::reader::RenderReader;
use log::info;
use std::iter;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, Buffer, CommandEncoder, Device, LoadOp, Operations, Queue,
    RenderPassDepthStencilAttachment, RenderPipeline, SurfaceError, Texture, TextureFormat,
    TextureView,
};
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::{Window, WindowBuilder, WindowId};

use super::metrics_reader::MetricsReader;
use super::renderable::Renderable;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Paused,
    Play,
}

pub struct Renderer<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    fps: f32,
    camera_state: CameraState,
    size: PhysicalSize<u32>,
    reader: T,
    metrics_reader: Option<MetricsReader>,
    _data: PhantomData<U>,
}

impl<T, U> Renderer<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    pub fn new(
        reader: T,
        fps: f32,
        camera: Camera,
        (width, height): (u32, u32),
        metrics_reader: Option<MetricsReader>,
    ) -> Self {
        Self {
            reader,
            fps,
            camera_state: CameraState::new(camera, width, height),
            size: PhysicalSize { width, height },
            metrics_reader,
            _data: PhantomData::default(),
        }
    }
}

impl<T, U> Attachable for Renderer<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    type Output = State<T, U>;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> (Self::Output, Window) {
        let window = WindowBuilder::new()
            .with_title("Point Cloud Renderer")
            .with_position(PhysicalPosition { x: 0, y: 0 })
            .with_inner_size(self.size)
            .build(event_loop)
            .unwrap();

        let gpu = pollster::block_on(WindowGpu::new(&window));
        let state = State::new(
            event_loop.create_proxy(),
            gpu,
            self.reader,
            self.fps,
            self.camera_state,
            self.metrics_reader,
        );
        (state, window)
    }
}

pub struct State<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    // Windowing
    event_proxy: EventLoopProxy<RenderEvent>,
    last_render_time: Option<Instant>,
    listeners: Vec<WindowId>,

    // GPU variables
    gpu: WindowGpu,
    pcd_renderer: PointCloudRenderer<U>,
    camera_state: CameraState,

    // Playback
    current_position: usize,
    fps: f32, // the average playout fps
    time_to_advance: std::time::Duration,
    state: PlaybackState,
    time_since_last_update: std::time::Duration,
    reader: T,

    // Rendering Stats
    metrics_reader: Option<MetricsReader>,
    metrics_renderer: MetricsRenderer,
    metrics: Vec<(String, String)>,
    staging_belt: StagingBelt,
}

impl<T, U> Windowed for State<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    fn add_output(&mut self, window_id: WindowId) {
        self.listeners.push(window_id);
    }

    fn handle_event(&mut self, event: &Event<RenderEvent>, window: &Window) {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                if let winit::event::DeviceEvent::Key(_) = event {
                    return;
                }
                self.handle_device_event(event);
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                window_id,
            } if *window_id == window.id() => {
                self.handle_device_event(&DeviceEvent::Key(*input));
            }
            Event::RedrawRequested(window_id) if *window_id == window.id() => {
                if self.last_render_time.is_none() {
                    self.last_render_time = Some(Instant::now());
                }
                let last_render_time = self.last_render_time.unwrap();
                let now = Instant::now();
                let dt = now - last_render_time;
                self.last_render_time = Some(now);
                match self.update(dt) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::OutOfMemory) => {}
                    Err(e) => eprintln!("Dropped frame due to {:?}", e),
                }
            }
            Event::UserEvent(RenderEvent {
                window_id,
                event_type,
            }) if *window_id == window.id() => match event_type {
                EventType::Toggle => self.toggle(),
                EventType::MoveTo(position) => self.move_to(*position),
                _ => {}
            },
            _ => {}
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.camera_state.resize(new_size);
        self.pcd_renderer.resize(new_size, &self.gpu.device);
    }
}

impl<T, U> State<T, U>
where
    T: RenderReader<U>,
    U: Renderable,
{
    fn new(
        event_proxy: EventLoopProxy<RenderEvent>,
        gpu: WindowGpu,
        mut reader: T,
        fps: f32,
        camera_state: CameraState,
        metrics_reader: Option<MetricsReader>,
    ) -> Self {
        let initial_render = reader
            .start()
            .expect("There should be at least one point cloud to render!");
        let pcd_renderer = PointCloudRenderer::new(
            &gpu.device,
            gpu.config.format,
            &initial_render,
            gpu.size,
            &camera_state,
        );

        let metrics_renderer = MetricsRenderer::new(gpu.size, &gpu.device);

        let mut state = Self {
            event_proxy,
            listeners: Vec::new(),
            last_render_time: None,

            gpu,
            pcd_renderer,
            camera_state,

            current_position: 0,
            fps,
            time_to_advance: std::time::Duration::from_secs(1).div_f32(fps),
            state: PlaybackState::Paused,
            time_since_last_update: std::time::Duration::from_secs(0),
            reader,

            metrics_reader,
            metrics_renderer,
            metrics: vec![],
            staging_belt: StagingBelt::new(1024),
        };

        state.update_stats();
        match state.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::OutOfMemory) => {}
            Err(e) => eprintln!("Dropped frame due to {:?}", e),
        }
        state
    }

    fn toggle(&mut self) {
        match self.state {
            PlaybackState::Play => self.pause(),
            PlaybackState::Paused => self.play(),
        }
    }

    fn play(&mut self) {
        self.state = PlaybackState::Play;
    }

    fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    fn move_to(&mut self, position: usize) {
        if position >= self.reader.len() {
            return;
        }
        let now = Instant::now();
        let tmp = self.current_position;
        self.current_position = position;
        if !self.update_vertices() {
            self.current_position = tmp;
        }
        self.update_stats();
        // FIXME: avg_fps might not be accurately when a frame fails to render. but it's not a big deal
        let time_taken = now.elapsed();
        info!(
            "time taken: {}",
            time_taken.max(self.time_to_advance).as_secs_f32()
        );
        self.fps =
            0.9 * self.fps + 0.1 * (1.0 / time_taken.max(self.time_to_advance).as_secs_f32());
    }

    fn back(&mut self) {
        if self.current_position > 0 {
            self.move_to(self.current_position - 1);
        }
    }

    fn advance(&mut self) {
        // println!(
        //     "[renderer.rs] advanced called. current_position: {}",
        //     self.current_position
        // );
        if self.current_position == self.reader.len() - 1 {
            self.move_to(0);
        } else {
            self.move_to(self.current_position + 1);
        }
    }

    fn current(&mut self) -> Option<U> {
        self.reader.get_at(self.current_position)
    }

    fn handle_device_event(&mut self, event: &DeviceEvent) {
        self.camera_state.process_input(event);
        if let DeviceEvent::Key(KeyboardInput {
            virtual_keycode: Some(key),
            state,
            ..
        }) = event
        {
            match (key, state) {
                (VirtualKeyCode::Space, ElementState::Pressed) => {
                    self.toggle();
                }
                (VirtualKeyCode::Left, ElementState::Pressed) => {
                    self.pause();
                    self.back();
                }
                (VirtualKeyCode::Right, ElementState::Pressed) => {
                    self.pause();
                    self.advance();
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, dt: Duration) -> Result<(), SurfaceError> {
        self.camera_state.update(dt);
        self.pcd_renderer
            .update_camera(&self.gpu.queue, self.camera_state.camera_uniform());

        if self.state == PlaybackState::Play {
            self.time_since_last_update += dt;
            if self.time_since_last_update >= self.time_to_advance {
                self.advance();
                self.time_since_last_update -= self.time_to_advance;
            }
        };

        let info = RenderInformation {
            camera: self.camera_state.camera(),
            current_position: self.current_position,
            fps: self.fps,
        };

        for listener in &self.listeners {
            self.event_proxy
                .send_event(RenderEvent {
                    window_id: *listener,
                    event_type: EventType::Info(info),
                })
                .unwrap();
        }
        self.render()
    }

    fn update_vertices(&mut self) -> bool {
        if let Some(data) = self.current() {
            self.pcd_renderer
                .update_vertices(&self.gpu.device, &self.gpu.queue, &data);
            return true;
        }
        return false;
    }

    fn update_stats(&mut self) {
        if let Some(metrics_reader) = &self.metrics_reader {
            if let Some(metrics) = metrics_reader.get_at(self.current_position) {
                self.metrics = metrics.metrics();
            }
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (output, view) = self.gpu.create_view()?;
        let mut encoder = self.gpu.create_encoder();

        self.pcd_renderer.render(&mut encoder, &view);
        self.metrics_renderer.draw(
            &self.gpu.device,
            &mut self.staging_belt,
            &mut encoder,
            &view,
            &self.metrics,
        );

        self.staging_belt.finish();
        self.gpu.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();
        Ok(())
    }
}

pub struct PointCloudRenderer<T: Renderable> {
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    antialias_bind_group: BindGroup,
    depth_texture: Texture,
    depth_view: TextureView,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    num_vertices: usize,
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
        let num_vertices = initial_render.vertices();

        Self {
            camera_buffer,
            camera_bind_group,
            antialias_bind_group,
            depth_texture,
            depth_view,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            _data: PhantomData::default(),
        }
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
        let vertices = data.vertices();
        if vertices > self.num_vertices {
            self.vertex_buffer.destroy();
            self.vertex_buffer = data.create_buffer(device);
        } else {
            queue.write_buffer(&self.vertex_buffer, 0, data.bytes());
        }
        self.num_vertices = vertices;
    }

    pub fn render(&mut self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
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

struct MetricsRenderer {
    size: PhysicalSize<u32>,
    glyph_brush: GlyphBrush<()>,
}

impl MetricsRenderer {
    pub fn new(size: PhysicalSize<u32>, device: &Device) -> Self {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf"))
            .expect("Could not initialize font");
        let glyph_brush =
            GlyphBrushBuilder::using_font(font).build(device, wgpu::TextureFormat::Bgra8UnormSrgb);

        Self { size, glyph_brush }
    }

    pub fn draw(
        &mut self,
        device: &Device,
        staging_belt: &mut StagingBelt,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        stats: &Vec<(String, String)>,
    ) {
        let x_offset = 30.0;
        let mut y_offset = self.size.height as f32 - 30.0;
        for (key, val) in stats {
            self.glyph_brush.queue(Section {
                screen_position: (x_offset, y_offset),
                bounds: (self.size.width as f32, self.size.height as f32),
                text: vec![Text::new(&format!("{key}: {val}"))
                    .with_color([0.0, 0.0, 0.0, 1.0])
                    .with_scale(20.0)],
                ..Section::default()
            });
            y_offset -= 30.0;
        }

        self.glyph_brush
            .draw_queued(
                device,
                staging_belt,
                encoder,
                view,
                self.size.width,
                self.size.height,
            )
            .expect("Draw queued");
    }
}
