use crate::render::wgpu::builder::{
    Attachable, EventType, RenderEvent, RenderInformation, Windowed,
};
use crate::render::wgpu::gpu::WindowGpu;
use egui::{Button, CentralPanel, Context, FontDefinitions, Label, Slider};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::Frame;
use std::fmt::format;
use std::iter;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowId};

struct EventProxy(
    std::sync::Mutex<winit::event_loop::EventLoopProxy<RenderEvent>>,
    WindowId,
);

impl epi::backend::RepaintSignal for EventProxy {
    fn request_repaint(&self) {
        self.0
            .lock()
            .unwrap()
            .send_event(RenderEvent {
                window_id: self.1,
                event_type: EventType::Repaint,
            })
            .ok();
    }
}

pub struct Controller {
    pub slider_end: usize,
}

impl Attachable for Controller {
    type Output = ControlWindow;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> (Self::Output, Window) {
        let window = winit::window::WindowBuilder::new()
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_title("Controls")
            .with_inner_size(winit::dpi::PhysicalSize {
                width: 500i32,
                height: 400i32,
            })
            .build(event_loop)
            .unwrap();

        let gpu = pollster::block_on(WindowGpu::new(&window));

        let surface_format = gpu.surface.get_supported_formats(&gpu.adapter)[0];

        let event_proxy = Arc::new(EventProxy(
            std::sync::Mutex::new(event_loop.create_proxy()),
            window.id(),
        ));

        // We use the egui_winit_platform crate as the platform.
        let platform = Platform::new(PlatformDescriptor {
            physical_width: gpu.size.width,
            physical_height: gpu.size.height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        let egui_rpass = RenderPass::new(&gpu.device, surface_format, 1);

        let object = ControlWindow {
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
            listeners: Vec::new(),
            display_help: false,
        };

        (object, window)
    }
}

pub struct ControlWindow {
    gpu: WindowGpu,
    event_proxy: Arc<EventProxy>,
    platform: Platform,
    egui_rpass: RenderPass,
    start_time: Option<Instant>,
    previous_frame_time: Option<f32>,

    prev_slider_position: usize,
    slider_position: usize,
    slider_end: usize,
    info: Option<RenderInformation>,
    listeners: Vec<WindowId>,
    display_help: bool,
}

impl ControlWindow {
    fn update(&mut self, ctx: &Context, _frame: &Frame) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.add(Button::new("Play / Pause")).clicked() {
                self.toggle();
            };

            ui.add(
                Slider::new(&mut self.slider_position, 0..=(self.slider_end))
                    .text(&format!("/ {}", self.slider_end))
                    .integer(),
            );

            if let Some(info) = self.info {
                ui.add(Label::new(format!(
                    "Camera Position: PointXyz [{:.2}, {:.2}, {:.2}]", info.camera.position.x, info.camera.position.y, info.camera.position.z
                )));
                ui.add(Label::new(format!(
                    "Camera Yaw: {:?}",
                    cgmath::Deg::from(info.camera.yaw)
                )));
                ui.add(Label::new(format!(
                    "Camera Pitch: {:?}",
                    cgmath::Deg::from(info.camera.pitch)
                )));
                ui.add(Label::new(format!("Avg fps: {:?}", info.fps)));

                let display_or_hide = if self.display_help {
                    "Hide"
                } else {
                    "Display"
                };
                if ui.add(Button::new(format!("{} Control Help", display_or_hide))).clicked() {
                    self.display_help = !self.display_help;
                };

                if self.display_help {
                    ui.scope(|ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Heading);
                        ui.visuals_mut().override_text_color = Some(egui::Color32::YELLOW);
                        ui.label("\nHow to control?");
                    });

                    ui.scope(|ui| {
                        ui.style_mut().override_text_style   = Some(egui::TextStyle::Body);
                        ui.visuals_mut().override_text_color = Some(egui::Color32::LIGHT_YELLOW);
                        ui.style_mut().override_text_style   = Some(egui::TextStyle::Monospace);
                        ui.label("W          Key - Moves your position to the front");
                        ui.label("A          Key - Moves your position to the left");
                        ui.label("S          Key - Moves your position to the back");
                        ui.label("D          Key - Moves your position to the right");
                        ui.label("Q          Key - Moves your position up");
                        ui.label("E          Key - Moves your position down");
                        ui.label("0          Key - Resets your position to the initial position");
                        ui.label("Space      Key - Toggles  Play / Pause");
                        ui.label("LeftArrow  Key - Rewinds  by 1 frame");
                        ui.label("RightArrow Key - Advances by 1 frame");
                        ui.label("Adjusts camera yaw/picth with mouse \n(Hold right click on Mac, left click on Windows)");
                    });
                }
            }
        });

        if self.slider_position != self.prev_slider_position {
            self.move_to(self.slider_position);
            self.prev_slider_position = self.slider_position;
        }
    }
}

impl Windowed for ControlWindow {
    fn add_output(&mut self, window_id: WindowId) {
        self.listeners.push(window_id);
    }

    fn handle_event(&mut self, event: &Event<RenderEvent>, window: &Window) {
        self.platform.handle_event(event);
        match event {
            Event::RedrawRequested(window_id) if *window_id == window.id() => self.render(window),

            Event::UserEvent(RenderEvent {
                window_id,
                event_type,
            }) if *window_id == window.id() => match event_type {
                EventType::Repaint => {
                    window.request_redraw();
                }
                EventType::Info(info) => {
                    self.info = Some(*info);
                    self.prev_slider_position = info.current_position;
                    self.slider_position = info.current_position;
                }
                _ => {}
            },
            _ => (),
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.gpu.resize(size);
    }
}

impl ControlWindow {
    fn toggle(&self) {
        let sender = self.event_proxy.0.lock().unwrap();

        for &listener in &self.listeners {
            sender
                .send_event(RenderEvent {
                    window_id: listener,
                    event_type: EventType::Toggle,
                })
                .unwrap();
        }
    }

    fn move_to(&self, position: usize) {
        let sender = self.event_proxy.0.lock().unwrap();

        for &listener in &self.listeners {
            sender
                .send_event(RenderEvent {
                    window_id: listener,
                    event_type: EventType::MoveTo(position),
                })
                .unwrap();
        }
    }

    fn render(&mut self, window: &Window) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }

        let start_time = self.start_time.unwrap();
        self.platform
            .update_time(start_time.elapsed().as_secs_f64());

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

        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: "egui_example",
                web_info: None,
                cpu_usage: self.previous_frame_time,
                native_pixels_per_point: Some(window.scale_factor() as _),
                prefer_dark_mode: None,
            },
            output: app_output,
            repaint_signal: self.event_proxy.clone(),
        });

        // Draw the demo application.
        self.update(&self.platform.context(), &frame);

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.platform.end_frame(Some(window));
        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
        self.previous_frame_time = Some(frame_time);

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: self.gpu.config.width,
            physical_height: self.gpu.config.height,
            scale_factor: window.scale_factor() as f32,
        };

        self.egui_rpass
            .add_textures(
                &self.gpu.device,
                &self.gpu.queue,
                &full_output.textures_delta,
            )
            .expect("Should be able to add textures to control window");

        self.egui_rpass.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &paint_jobs,
            &screen_descriptor,
        );
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
