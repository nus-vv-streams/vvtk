use std::iter;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use wgpu::{LoadOp, Operations, RenderPassDepthStencilAttachment, SurfaceError};
use wgpu::util::DeviceExt;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::{Window, WindowBuilder, WindowId};
use crate::render::wgpu::builder::{Attachable, EventType, RenderEvent, RenderInformation, Windowed};
use crate::render::wgpu::camera::{Camera, CameraState};
use crate::render::wgpu::gpu::Gpu;
use crate::render::wgpu::reader::RenderReader;
use crate::render::wgpu::renderable::Renderable;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Paused,
    Play
}

pub struct Renderer<T, U> where T: RenderReader<U>, U: Renderable {
    fps: f32,
    camera_state: CameraState,
    size: PhysicalSize<u32>,
    reader: T,
    _data: PhantomData<U>,
}

impl<T, U> Renderer<T, U>  where T: RenderReader<U>, U: Renderable {
    pub fn new(reader: T, fps: f32, camera: Camera, (width, height): (u32, u32)) -> Self {
        Self {
            reader,
            fps,
            camera_state: CameraState::new(camera, width, height),
            size: PhysicalSize { width, height },
            _data: PhantomData::default(),
        }
    }
}

impl<T, U> Attachable for Renderer<T, U> where T: RenderReader<U>, U: Renderable {
    type Output = State<T, U>;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> (Self::Output, Window) {
        let window = WindowBuilder::new()
            .with_title("Point Cloud Renderer")
            .with_position(PhysicalPosition { x: 0, y: 0 })
            .with_inner_size(self.size)
            .build(event_loop)
            .unwrap();

        let gpu = pollster::block_on(Gpu::new(&window));
        let state = State::new(event_loop.create_proxy(), gpu, self.reader, self.fps, self.camera_state);
        (state, window)
    }
}

pub struct State<T, U> where T: RenderReader<U>, U: Renderable {
    // Windowing
    event_proxy: EventLoopProxy<RenderEvent>,
    last_render_time: Option<Instant>,
    listeners: Vec<WindowId>,

    // GPU variables
    gpu: Gpu,
    camera_state: CameraState,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    antialias_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    num_vertices: usize,

    // Playback
    current_position: usize,
    fps: f32,
    time_to_advance: std::time::Duration,
    state: PlaybackState,
    time_since_last_update: std::time::Duration,
    reader: T,
    _data: PhantomData<U>,
}

impl<T, U> Windowed for State<T, U>  where T: RenderReader<U>, U: Renderable {
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
                window_id
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
                    Err(wgpu::SurfaceError::OutOfMemory) => {},
                    Err(e) => eprintln!("Dropped frame due to {:?}", e),
                }
            }
            Event::UserEvent(RenderEvent { window_id, event_type }) if *window_id == window.id() => {
                match event_type {
                    EventType::Toggle => self.toggle(),
                    EventType::MoveTo(position) => self.move_to(*position),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.camera_state.resize(new_size);
        if new_size.width > 0 && new_size.height > 0 {
            let (depth_texture, depth_view) = U::create_depth_texture(&self.gpu);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
        }
    }
}

impl<T, U> State<T, U>  where T: RenderReader<U>, U: Renderable {
    fn new(event_proxy: EventLoopProxy<RenderEvent>,
           gpu: Gpu,
           reader: T,
           fps: f32,
           camera_state: CameraState) -> Self {
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) = camera_state.create_buffer(&gpu.device);
        let (antialias_bind_group_layout, antialias_bind_group) = Self::create_antialias_buffer(&gpu.device, &reader);

        let render_pipeline_layout =
            gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &antialias_bind_group_layout],
                push_constant_ranges: &[],
            });



        let render_pipeline = U::create_render_pipeline(&gpu, Some(&render_pipeline_layout));
        let (depth_texture, depth_view) = U::create_depth_texture(&gpu);

        let mut vertex_buffer = None;
        let mut num_vertices = 0;

        if let Some(data) = reader.get_at(0) {
            vertex_buffer = Some(data.create_buffer(&gpu.device));
            num_vertices = data.vertices();
        }

        let mut state = Self {
            event_proxy,
            listeners: Vec::new(),
            last_render_time: None,

            gpu,
            camera_state,
            camera_buffer,
            camera_bind_group,
            antialias_bind_group,
            depth_texture,
            depth_view,
            render_pipeline,
            vertex_buffer,
            num_vertices,

            current_position: 0,
            fps,
            time_to_advance: std::time::Duration::from_secs(1).div_f32(fps),
            state: PlaybackState::Paused,
            time_since_last_update: std::time::Duration::from_secs(0),
            reader,
            _data: PhantomData::default(),
        };

        match state.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::OutOfMemory) => {},
            Err(e) => eprintln!("Dropped frame due to {:?}", e),
        }
        state
    }

    fn create_antialias_buffer(device: &wgpu::Device, reader: &T) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let antialias_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Antialias Buffer"),
            contents: bytemuck::cast_slice(&[reader.antialias()]),
            usage: wgpu::BufferUsages::UNIFORM
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

    fn toggle(&mut self) {
        match self.state {
            PlaybackState::Play => self.pause(),
            PlaybackState::Paused => self.play()
        }
    }

    fn play(&mut self) {
        self.state = PlaybackState::Play;
    }

    fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    fn move_to(&mut self, position: usize) {
        if position < self.reader.len() {
            self.current_position = position;
            self.update_vertices();
        }
    }

    fn back(&mut self) {
        if self.current_position > 0 {
            self.move_to(self.current_position - 1);
        }
    }

    fn advance(&mut self) {
        if self.current_position == self.reader.len() - 1 {
            self.move_to(0);
        } else {
            self.move_to(self.current_position + 1);
        }
    }

    fn current_position(&self) -> usize {
        self.current_position
    }

    fn current(&self) -> Option<U> {
        self.reader.get_at(self.current_position())
    }

    fn handle_device_event(&mut self, event: &DeviceEvent) {
        self.camera_state.process_input(event);
        if let DeviceEvent::Key(KeyboardInput { virtual_keycode: Some(key), state, ..}) = event {
            match (key, state) {
                (VirtualKeyCode::Space, ElementState::Pressed) => {
                    self.toggle();
                },
                (VirtualKeyCode::Left, ElementState::Pressed) => {
                    self.pause();
                    self.back();
                },
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
        self.gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_state.camera_uniform()]),
        );

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
            fps: self.fps
        };

        for listener in &self.listeners {
            self.event_proxy.send_event(RenderEvent {
                window_id: *listener,
                event_type: EventType::Info(info)}).unwrap();
        }
        self.render()
    }

    fn update_vertices(&mut self) {
        if let Some(data) = self.current() {

            let vertices = data.vertices();
            match &self.vertex_buffer {
                None => self.vertex_buffer = Some(data.create_buffer(&self.gpu.device)),
                Some(buffer) => {
                    if vertices > self.num_vertices {
                        buffer.destroy();
                        self.vertex_buffer = Some(data.create_buffer(&self.gpu.device));
                    } else {
                        self.gpu.queue.write_buffer(
                            buffer,
                            0,
                            data.bytes()
                        );
                    }
                }
            }

            self.num_vertices = vertices;
        }
    }


    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (output, view) = self.gpu.create_view()?;
        let mut encoder = self.gpu.create_encoder();

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
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
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: true }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.antialias_bind_group, &[]);
            if let Some(buffer) = &self.vertex_buffer {
                render_pass.set_vertex_buffer(0, buffer.slice(..));
                render_pass.draw(0..(self.num_vertices as u32), 0..1);
            }
        }
        self.gpu.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
