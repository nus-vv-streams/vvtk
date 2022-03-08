use std::iter;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use wgpu::{DepthStencilState, Extent3d, include_wgsl, LoadOp, Operations, PipelineLayout, RenderPassDepthStencilAttachment, RenderPipeline, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, VertexBufferLayout};
use wgpu::CompareFunction::Less;
use wgpu::util::DeviceExt;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use fltk::{app, prelude::*};
use fltk::app::{focus, Sender};
use fltk::button::Button;
use crate::pcd::PointCloudData;
use crate::render::wgpu::camera::{Camera, CameraState};
use crate::render::wgpu::gpu::Gpu;
use crate::render::wgpu::reader::RenderReader;

pub trait Renderable {
    fn buffer_layout_desc<'a>() -> wgpu::VertexBufferLayout<'a>;
    fn create_render_pipeline(device: &Gpu, layout: Option<&wgpu::PipelineLayout>) -> RenderPipeline;
    fn create_depth_texture(gpu: &Gpu) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = gpu.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: gpu.config.width,
                height: gpu.config.height,
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
    fn bytes(&self) -> &[u8];
    fn vertices(&self) -> usize;
}

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
        let shader = gpu.device.create_shader_module(&include_wgsl!("shaders/pcd.wgsl"));

        gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: layout,
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

pub struct RenderBuilder<T: 'static + Renderable, U: 'static + RenderReader<T>> {
    reader: U,
    fps: f32,
    camera_state: CameraState,
    width: u32,
    height: u32,

    _data: PhantomData<T>
}

impl<T, U> RenderBuilder<T, U> where T: 'static + Renderable, U: 'static + RenderReader<T> {
    pub fn new(reader: U, fps: f32, camera: Camera, (width, height): (u32, u32)) -> Self {
        Self {
            reader,
            fps,
            camera_state: CameraState::new(camera, width, height),
            width,
            height,
            _data: PhantomData::default()
        }
    }

    pub async fn play(self, show_controls: bool) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_position(Position::Physical(PhysicalPosition::new(0, 0)))
            .with_inner_size(Size::Physical(PhysicalSize::from((self.width, self.height))))
            .build(&event_loop)
            .unwrap();
        let gpu = Gpu::new(&window).await;
        let slider_length = self.reader.len() - 1;
        let mut state = Renderer::new(gpu, self.reader, self.fps, self.camera_state);

        let (state_tx, state_rx) = channel();
        enum Control {
            Toggle,
            MoveTo(usize),
        }
        let sender: Arc<Mutex<Option<Sender<RenderInformation>>>> = Arc::new(Mutex::new(None));
        if show_controls {
            let app_sender = sender.clone();

            std::thread::spawn(move || {
                let window_width = 400;
                let window_height = 200;
                let app = app::App::default();
                let (controls_tx, controls_rx) = app::channel();
                {
                    *app_sender.lock().unwrap() = Some(controls_tx);
                    drop(app_sender);
                }
                let mut wind = fltk::window::Window::new(self.width as i32, self.height as i32 / 2, window_width, window_height, "Playback Controls");
                let col = fltk::group::Column::new(0, 0, window_width, window_height, "Details");
                let mut but = Button::new((window_width / 2) - 40, 10, 80, 40, "Play/Pause");
                let mut slider = fltk::valuator::HorNiceSlider::default().with_size(window_width - 20, 20).center_of_parent();
                slider.set_minimum(0.);
                slider.set_maximum(slider_length as f64);
                slider.set_step(1., 1);
                slider.set_value(0.);
                let slider_tx = state_tx.clone();
                slider.set_callback(move |s| {
                    slider_tx.send(Control::MoveTo(s.value() as usize)).unwrap();
                });
                let mut progress = fltk::frame::Frame::default()
                    .with_label(&format!("0/{}", slider_length));
                let mut camera_position = fltk::frame::Frame::default()
                    .with_size(window_width, 20)
                    .with_label("Camera Position: ")
                    .below_of(&slider, 10);
                let mut camera_yaw = fltk::frame::Frame::default()
                    .with_size(window_width, 20)
                    .with_label("Camera Yaw: ")
                    .below_of(&camera_position, 10);
                let mut camera_pitch = fltk::frame::Frame::default()
                    .with_size(window_width, 20)
                    .with_label("Camera Pitch: ")
                    .below_of(&camera_yaw, 10);
                col.end();
                wind.end();
                wind.show();
                but.set_callback(move |_| {
                    state_tx.send(Control::Toggle).unwrap();
                });

                while app.wait() {
                    if let Some(info) = controls_rx.recv() {
                        progress.set_label(&format!("{}/{}", info.current_position, slider_length));
                        camera_position.set_label(&format!("Camera Position: {:?}", info.camera.position));
                        camera_yaw.set_label(&format!("Camera Yaw: {:?}", cgmath::Deg::from(info.camera.yaw)));
                        camera_pitch.set_label(&format!("Camera Pitch: {:?}", cgmath::Deg::from(info.camera.pitch)));
                        slider.set_value(info.current_position as f64)
                    }
                }
            });

            while sender.lock().unwrap().is_none() {}
        }

        let controls_tx = sender.clone();
        state.on_update(Box::new(move |state| {
            let guard = controls_tx.lock().unwrap();
            match *guard {
                Some(s) => {
                    s.send(state)
                },
                None => {}
            }
        }));
        let mut last_render_time = std::time::Instant::now();
        let mut focused = true;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match state_rx.try_recv() {
                Ok(Control::Toggle) => {
                    state.toggle();
                },
                Ok(Control::MoveTo(position)) => { state.move_to(position) },
                _ =>{}
            };


            match event {
                Event::MainEventsCleared => window.request_redraw(),
                Event::DeviceEvent {
                    ref event,
                    ..
                } => {
                    if let winit::event::DeviceEvent::Key(_) = event {
                        return;
                    }
                    if focused {
                        state.input(event);
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        WindowEvent::Focused(focus) => {
                            focused = *focus;
                        },
                        WindowEvent::KeyboardInput {
                            input,
                            ..
                        } => {
                            if focused {
                                state.input(&DeviceEvent::Key(*input));
                            }
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(window_id) if window_id == window.id() => {
                    let now = std::time::Instant::now();
                    let dt = now - last_render_time;
                    state.update(dt);
                    last_render_time = now;
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            }
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderInformation {
    pub camera: Camera,
    pub current_position: usize,
    pub fps: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Paused,
    Play
}

pub struct Renderer<T: 'static + Renderable, U: 'static + RenderReader<T>> {
    gpu: Gpu,
    camera_state: CameraState,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: Option<wgpu::Buffer>,
    num_vertices: usize,

    current_position: usize,
    fps: f32,
    time_to_advance: std::time::Duration,
    state: PlaybackState,
    time_since_last_update: std::time::Duration,
    reader: U,
    _data: PhantomData<T>,
    on_update: Option<Box<dyn Fn(RenderInformation) -> ()>>
}

unsafe impl<T, U> Sync for Renderer<T, U> where T: Renderable, U: RenderReader<T> {}

unsafe impl<T, U> Send for Renderer<T, U> where T: Renderable, U: RenderReader<T> {}


impl<T, U> Renderer<T, U> where T: 'static + Renderable, U: 'static + RenderReader<T> {
    pub fn new(gpu: Gpu, reader: U, fps: f32, camera_state: CameraState) -> Self {
        let (camera_buffer, camera_bind_group_layout, camera_bind_group) = camera_state.create_buffer(&gpu.device);
        let render_pipeline_layout =
            gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = T::create_render_pipeline(&gpu, Some(&render_pipeline_layout));
        let (depth_texture, depth_view) = T::create_depth_texture(&gpu);

        let mut vertex_buffer = None;
        let mut num_vertices = 0;

        if let Some(data) = reader.get_at(0) {
            vertex_buffer = Some(data.create_buffer(&gpu.device));
            num_vertices = data.vertices();
        }


        Self {
            gpu,
            camera_state,
            camera_buffer,
            camera_bind_group,
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
            on_update: None
        }
    }

    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Paused => self.play(),
            PlaybackState::Play => self.pause(),
        }
    }

    pub fn play(&mut self) {
        self.state = PlaybackState::Play;
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    pub fn move_to(&mut self, position: usize) {
        if position < self.reader.len() {
            self.current_position = position;
            self.update_vertices();
        }
    }

    pub fn back(&mut self) {
        if self.current_position > 0 {
            self.move_to(self.current_position - 1);
        }
    }

    pub fn advance(&mut self) {
        self.move_to(self.current_position + 1);
        if self.current_position == self.reader.len() - 1 {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn current_position(&self) -> usize {
        self.current_position
    }

    pub fn current(&self) -> Option<T> {
        self.reader.get_at(self.current_position)
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
        self.camera_state.resize(new_size);
        if new_size.width > 0 && new_size.height > 0 {
            let (depth_texture, depth_view) = T::create_depth_texture(&self.gpu);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
        }
    }

    pub fn input(&mut self, event: &DeviceEvent) -> bool {
        self.camera_state.process_input(event);
        if let DeviceEvent::Key(KeyboardInput { virtual_keycode: Some(key), state, ..}) = event {
            match (key, state) {
                (VirtualKeyCode::Space, ElementState::Pressed) => {
                    self.toggle();
                    true
                },
                (VirtualKeyCode::Left, ElementState::Pressed) => {
                    self.state = PlaybackState::Paused;
                    self.back();
                    true
                },
                (VirtualKeyCode::Right, ElementState::Pressed) => {
                    self.state = PlaybackState::Paused;
                    self.advance();
                    true
                }
                _ => false
            }
        } else {
            false
        }
    }

    pub fn on_update(&mut self, callback: Box<dyn Fn(RenderInformation) -> ()>) {
        self.on_update = Some(callback);
    }

    pub fn update(&mut self, dt: std::time::Duration) -> bool {
        self.camera_state.update(dt);
        self.gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_state.camera_uniform()]),
        );

        let result = if self.state == PlaybackState::Paused {
            false
        } else {
            self.time_since_last_update += dt;
            if self.time_since_last_update >= self.time_to_advance {
                self.advance();
                self.time_since_last_update -= self.time_to_advance;
                true
            } else {
                false
            }
        };

        if let Some(cb) = &self.on_update {
            cb(RenderInformation {
                camera: self.camera_state.camera(),
                current_position: self.current_position,
                fps: self.fps
            })
        }

        result
    }

    pub fn update_vertices(&mut self) {
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


    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
