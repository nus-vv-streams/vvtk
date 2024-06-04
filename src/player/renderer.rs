use super::builder::{
    Attachable, EventType, RenderEvent, RenderInformation, Windowed,
};
use super::metrics_reader::MetricsReader;
use super::render_manager::RenderManager;
// use std::f16::consts::E;
// use winit::dpi::{PhysicalPosition, PhysicalSize};
use log::debug;
use std::iter;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use crate::render::wgpu::camera::{Camera, CameraState};
use crate::render::wgpu::color::parse_wgpu_color;
use crate::render::wgpu::gpu::WindowGpu;
use crate::render::wgpu::point_cloud_renderer::PointCloudRenderer;
use crate::render::wgpu::renderable::Renderable;
use wgpu::util::StagingBelt;
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::dpi::PhysicalSize;
use winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::{Window, WindowBuilder, WindowId};



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Paused,
    Play,
}

pub struct Renderer<T, U>
where
    T: RenderManager<U>,
    U: Renderable,
{
    fps: f32,
    camera_state: CameraState,
    size: PhysicalSize<u32>,
    reader: T,
    metrics_reader: Option<MetricsReader>,
    _data: PhantomData<U>,
    bg_color: wgpu::Color,
}

impl<T, U> Renderer<T, U>
where
    T: RenderManager<U>,
    U: Renderable,
{
    pub fn new(
        reader: T,
        fps: f32,
        camera: Camera,
        (width, height): (u32, u32),
        metrics_reader: Option<MetricsReader>,
        bg_color_str: &str,
    ) -> Self {
        Self {
            reader,
            fps,
            camera_state: CameraState::new(camera, width, height),
            size: PhysicalSize { width, height },
            metrics_reader,
            _data: PhantomData::default(),
            bg_color: parse_wgpu_color(bg_color_str).unwrap(),
        }
    }
}

impl<T, U> Attachable for Renderer<T, U>
where
    T: RenderManager<U>,
    U: Renderable,
{
    type Output = State<T, U>;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> (Self::Output, Window) {
        let window = WindowBuilder::new()
            .with_title("vvplay")
            // .with_position(PhysicalPosition { x: 0, y: 0 })
            .with_resizable(true)
            // .with_min_inner_size(self.size)
            .with_max_inner_size(PhysicalSize::new(2048, 2048))
            .with_inner_size(self.size)
            .with_active(true)
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
            self.bg_color,
        );
        (state, window)
    }
}

/// Renderer's state
pub struct State<T, U>
where
    T: RenderManager<U>,
    U: Renderable,
{
    // Windowing
    event_proxy: EventLoopProxy<RenderEvent>,
    last_render_time: Option<Instant>,
    listeners: Vec<WindowId>,
    mouse_in_window: bool,
    mouse_pressed: bool,
    resizing: bool,

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
    T: RenderManager<U>,
    U: Renderable,
{
    fn add_output(&mut self, window_id: WindowId) {
        self.listeners.push(window_id);
    }

    fn handle_event(&mut self, event: &Event<RenderEvent>, window: &Window) {
        match event {
            Event::DeviceEvent { ref event, .. } => match event {
                DeviceEvent::MouseWheel { .. } => {
                    self.camera_state.handle_mouse_input(event);
                }
                DeviceEvent::Key { .. } => {
                    self.camera_state.handle_keyboard_input(event);
                    self.handle_keyboard_input(event);
                }
                _ => {
                    if self.mouse_in_window && !self.resizing && self.mouse_pressed {
                        self.camera_state.handle_mouse_input(event);
                    }
                }
            },
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                window_id,
            } if *window_id == window.id() => {
                self.camera_state
                    .handle_keyboard_input(&DeviceEvent::Key(*input));
                self.handle_keyboard_input(&DeviceEvent::Key(*input));
            }
            Event::WindowEvent { event, window_id } if *window_id == window.id() => {
                match event {
                    WindowEvent::CursorEntered { .. } => {
                        self.mouse_in_window = true;
                    }
                    WindowEvent::CursorLeft { .. } => {
                        self.mouse_in_window = false;
                    }
                    // we need to keep track of mouse click/drag that is related to
                    // resizing the window.  We can tell if the user is resizing when
                    // we received the resizing message and the mouse button is pressed.
                    WindowEvent::Resized { .. } => {
                        if self.mouse_pressed {
                            // Resize event is received when the window is first created, so need to
                            // cross check with mouse button pressed to determine if the user is resizing.
                            self.resizing = true;
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if *state == ElementState::Released && *button == MouseButton::Left {
                            self.mouse_pressed = false;
                        } else if *state == ElementState::Pressed && *button == MouseButton::Left {
                            self.mouse_pressed = true;
                        }
                        // mouse must be in window since we received the mouse event
                        self.mouse_in_window = true;
                        // Assuming that users only resize with mouse drag, when user release the button,
                        // resizing is done.
                        if self.resizing && !self.mouse_pressed {
                            self.resizing = false;
                            // not passing this mouse button release to camera control since this is the
                            // last of a resize operation
                        } else {
                            // pass the rest to camera control
                            self.camera_state.handle_mouse_input(&DeviceEvent::Button {
                                button: 1,
                                state: *state,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if *window_id == window.id() => {
                if self.last_render_time.is_none() {
                    self.last_render_time = Some(Instant::now());
                }
                let last_render_time = self.last_render_time.unwrap();
                let now = Instant::now();
                let dt = now - last_render_time;
                self.last_render_time = Some(now);
                match self.redraw(dt) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.gpu.size),
                    // TODO: The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => {}
                    Err(e) => eprintln!("Dropped frame due to {e:?}"),
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
    T: RenderManager<U>,
    U: Renderable,
{
    fn new(
        event_proxy: EventLoopProxy<RenderEvent>,
        gpu: WindowGpu,
        mut reader: T,
        fps: f32,
        camera_state: CameraState,
        metrics_reader: Option<MetricsReader>,
        bg_color: wgpu::Color,
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
            bg_color,
        );

        let metrics_renderer = MetricsRenderer::new(gpu.size, &gpu.device);

        let mut state = Self {
            event_proxy,
            listeners: Vec::new(),
            last_render_time: None,
            mouse_in_window: false,
            mouse_pressed: false,
            resizing: false,

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
            Err(wgpu::SurfaceError::OutOfMemory) => eprintln!("Out of memory"),
            Err(e) => eprintln!("Dropped frame due to {e:?}"),
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

    fn redisplay(&mut self) {
        self.move_to(self.current_position)
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
        debug!("move_to {} takes: {} µs", position, time_taken.as_micros());
        // println!(
        //     "time taken: {}",
        //     time_taken.max(self.time_to_advance).as_secs_f32()
        // );
        self.fps =
            0.9 * self.fps + 0.1 * (1.0 / time_taken.max(self.time_to_advance).as_secs_f32());
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

    fn handle_keyboard_input(&mut self, event: &DeviceEvent) {
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

    fn redraw(&mut self, dt: Duration) -> Result<(), wgpu::SurfaceError> {
        self.camera_state.update(dt);
        self.reader
            .set_camera_state(Some(self.camera_state.clone())); // TODO might be expensive
        self.pcd_renderer
            .update_camera(&self.gpu.queue, self.camera_state.camera_uniform);

        if self.state == PlaybackState::Play {
            self.time_since_last_update += dt;
            if self.time_since_last_update >= self.time_to_advance {
                self.advance();
                self.time_since_last_update = Duration::from_secs(0);
            }
        } else if self.reader.should_redraw(&self.camera_state) {
            self.redisplay();
        }

        let info = RenderInformation {
            camera: self.camera_state.camera,
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

    // temporary fix: remove this function because CameraPosition is not yet compatible with the rest of the code
    // use the original update_vertices function for now

    /// Update the vertices and optionally updates camera position
    /*
    fn update_vertices(&mut self) -> bool {
        if let (camera_pos, Some(data)) = self
            .reader
            .get_at(self.current_position, Some(*self.camera_state.camera))
        {
            self.pcd_renderer
                .update_vertices(&self.gpu.device, &self.gpu.queue, &data);
            if let Some(pos) = camera_pos {
                *self.camera_state.camera = pos;
            }
            return true;
        }
        false
    }
    */

    fn update_vertices(&mut self) -> bool {
        if let Some(data) = self.reader.get_at(self.current_position) {
            self.pcd_renderer
                .update_vertices(&self.gpu.device, &self.gpu.queue, &data);
            return true;
        }
        false
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
        // Calls encoder.finish() to obtain a CommandBuffer and submits it to the queue.
        self.gpu.queue.submit(iter::once(encoder.finish()));
        output.present();
        self.staging_belt.recall();
        Ok(())
    }
}

struct MetricsRenderer {
    size: PhysicalSize<u32>,
    glyph_brush: GlyphBrush<()>,
}

impl MetricsRenderer {
    pub fn new(size: PhysicalSize<u32>, device: &wgpu::Device) -> Self {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf"))
            .expect("Could not initialize font");
        let glyph_brush =
            GlyphBrushBuilder::using_font(font).build(device, wgpu::TextureFormat::Bgra8UnormSrgb);

        Self { size, glyph_brush }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
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

