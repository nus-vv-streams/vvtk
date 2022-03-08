use std::fmt::format;
use std::iter;
use std::sync::Arc;
use std::time::{Duration, Instant};
use egui::{Button, CentralPanel, CtxRef, FontDefinitions, Label, Slider, WidgetInfo, WidgetType};
use egui::output::OutputEvent;
use egui_demo_lib::WrapApp;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::{App, Frame};
use wgpu::SurfaceError;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event};
use winit::event::Event::{RedrawRequested, UserEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use crate::render::wgpu::builder::{Attachable, EventType, RenderEvent, RenderInformation, Windowed};
use crate::render::wgpu::gpu::Gpu;

struct EventProxy(std::sync::Mutex<winit::event_loop::EventLoopProxy<RenderEvent>>, WindowId);

impl epi::backend::RepaintSignal for EventProxy {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RenderEvent { window_id: self.1, event_type: EventType::Repaint }).ok();
    }
}

pub struct Controller {
    pub slider_end: usize,
}

impl Attachable for Controller {
    type Output = ControlWindow;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> Self::Output {
        let window = winit::window::WindowBuilder::new()
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_title("Controls")
            .with_inner_size(winit::dpi::PhysicalSize {
                width: 400i32,
                height: 200i32,
            })
            .build(&event_loop)
            .unwrap();

        let gpu = pollster::block_on(Gpu::new(&window));

        let surface_format = gpu.surface.get_preferred_format(&gpu.adapter).unwrap();

        let event_proxy = Arc::new(EventProxy(std::sync::Mutex::new(
            event_loop.create_proxy()
        ),window.id()));

        // We use the egui_winit_platform crate as the platform.
        let mut platform = Platform::new(PlatformDescriptor {
            physical_width: gpu.size.width as u32,
            physical_height: gpu.size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let mut egui_rpass = RenderPass::new(&gpu.device, surface_format, 1);

        ControlWindow {
            window,
            gpu,
            event_proxy,
            platform,
            egui_rpass,
            start_time: None,
            previous_frame_time: None,
            prev_slider_position: 0,
            slider_position: 0,
            slider_end: self.slider_end,
            info: None,
            listeners: Vec::new()
        }
    }
}

pub struct ControlWindow {
    window: Window,
    gpu: Gpu,
    event_proxy: Arc<EventProxy>,
    platform: Platform,
    egui_rpass: RenderPass,
    start_time: Option<Instant>,
    previous_frame_time: Option<f32>,

    prev_slider_position: usize,
    slider_position: usize,
    slider_end: usize,
    info: Option<RenderInformation>,
    listeners: Vec<WindowId>
}

impl App for ControlWindow {
    fn update(&mut self, ctx: &CtxRef, frame: &Frame) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.add(Button::new("Play / Pause")).clicked() {
                self.toggle();
            };

            ui.add(Slider::new(&mut self.slider_position, 0..=(self.slider_end))
                .text(&format!("/ {}", self.slider_end))
                .integer());
            if let Some(info) = self.info {
                ui.add(Label::new(&format!("Camera Position: {:?}", info.camera.position)));
                ui.add(Label::new(&format!("Camera Yaw: {:?}", cgmath::Deg::from(info.camera.yaw))));
                ui.add(Label::new(&format!("Camera Pitch: {:?}", cgmath::Deg::from(info.camera.pitch))));
            }
        });
        if self.slider_position != self.prev_slider_position {
            self.move_to(self.slider_position);
            self.prev_slider_position = self.slider_position;
        }
    }

    fn name(&self) -> &str {
        "Control Window"
    }
}

impl Windowed for ControlWindow {
    fn handle_event(&mut self, event: &Event<RenderEvent>, control: &mut ControlFlow) {
        self.platform.handle_event(event);
        match event {
            Event::MainEventsCleared => {
                self.window.request_redraw();
            },
            Event::RedrawRequested(..) => {
                self.render()
            }

            Event::UserEvent(RenderEvent { window_id, event_type })
                if *window_id == self.id() => {
                match event_type {
                    EventType::Repaint => { self.window.request_redraw(); }
                    EventType::Info(info) => {
                        self.info = Some(*info);
                        self.prev_slider_position = self.slider_position;
                        self.slider_position = info.current_position;
                    }
                    _ => {}
                }
            }
            Event::WindowEvent { ref event, window_id, .. }
                if *window_id == self.id() => match event {
                winit::event::WindowEvent::Resized(size) => {
                    self.gpu.resize(*size);
                }
                _ => {}
            },
            _ => (),
        }
    }
}

impl ControlWindow {
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn add_listener(&mut self, listener: WindowId) {
        self.listeners.push(listener);
    }

    fn toggle(&self) {
        let sender = self.event_proxy.0.lock().unwrap();

        for &listener in &self.listeners {
            sender.send_event(RenderEvent { window_id: listener, event_type: EventType::Toggle });
        }
    }

    fn move_to(&self, position: usize) {
        let sender = self.event_proxy.0.lock().unwrap();

        for &listener in &self.listeners {
            sender.send_event(RenderEvent { window_id: listener, event_type: EventType::MoveTo(position) });
        }
    }

    fn render(&mut self) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }

        let start_time = self.start_time.unwrap();
        self.platform.update_time(start_time.elapsed().as_secs_f64());

        let (output_frame, output_view) = match self.gpu.create_view() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                return;
            }
            Err(e) => {
                eprintln!("Dropped frame with error: {}", e);
                return;
            }
        };

        // Begin to draw the UI frame.
        let egui_start = Instant::now();
        self.platform.begin_frame();
        let app_output = epi::backend::AppOutput::default();

        let mut frame =  epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: "egui_example",
                web_info: None,
                cpu_usage: self.previous_frame_time,
                native_pixels_per_point: Some(self.window.scale_factor() as _),
                prefer_dark_mode: None,
            },
            output: app_output,
            repaint_signal: self.event_proxy.clone(),
        });

        // Draw the demo application.
        self.update(&self.platform.context(), &mut frame);

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let (_output, paint_commands) = self.platform.end_frame(Some(&self.window));
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
        self.previous_frame_time = Some(frame_time);

        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("encoder"),
        });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: self.gpu.config.width,
            physical_height: self.gpu.config.height,
            scale_factor: self.window.scale_factor() as f32,
        };
        self.egui_rpass.update_texture(&self.gpu.device, &self.gpu.queue, &self.platform.context().font_image());
        self.egui_rpass.update_user_textures(&self.gpu.device, &self.gpu.queue);
        self.egui_rpass.update_buffers(&self.gpu.device, &self.gpu.queue, &paint_jobs, &screen_descriptor);
        self.egui_rpass
            .execute(
                &mut encoder,
                &output_view,
                &paint_jobs,
                &screen_descriptor,
                Some(wgpu::Color::BLACK),
            )
            .unwrap();
        self.gpu.queue.submit(iter::once(encoder.finish()));
        output_frame.present();
    }
}