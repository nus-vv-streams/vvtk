use crate::render::wgpu::camera::Camera;
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Debug)]
pub struct RenderEvent {
    pub(crate) window_id: WindowId,
    pub(crate) event_type: EventType,
}

#[derive(Debug)]
pub enum EventType {
    MoveTo(usize),
    Toggle,
    Info(RenderInformation),
    Repaint,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderInformation {
    pub camera: Camera,
    pub current_position: usize,
    pub fps: f32,
}

pub trait Attachable {
    type Output: Windowed;

    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> (Self::Output, Window);
}

pub struct WindowedObject {
    pub window: Window,
    pub object: Box<dyn Windowed>,
    pub focused: bool,
}

pub trait Windowed {
    fn add_output(&mut self, window_id: WindowId);
    fn handle_event(&mut self, event: &Event<RenderEvent>, window: &Window);
    fn resize(&mut self, size: PhysicalSize<u32>);
}

pub struct RenderBuilder {
    event_loop: EventLoop<RenderEvent>,
    window_objects: HashMap<WindowId, WindowedObject>,
}

impl Default for RenderBuilder {
    fn default() -> Self {
        Self {
            event_loop: EventLoop::with_user_event(),
            window_objects: HashMap::new(),
        }
    }
}

impl RenderBuilder {
    pub fn add_window<T>(&mut self, attachable: T) -> WindowId
    where
        T: Attachable,
        <T as Attachable>::Output: 'static,
    {
        let (object, window) = attachable.attach(&self.event_loop);
        let id = window.id();
        let object = Box::new(object);
        self.window_objects.insert(
            id,
            WindowedObject {
                window,
                object,
                focused: true,
            },
        );
        id
    }

    pub fn get_windowed_mut(&mut self, id: WindowId) -> Option<&mut Box<dyn Windowed>> {
        self.window_objects.get_mut(&id).map(|obj| &mut obj.object)
    }

    pub fn run(mut self) {
        self.event_loop.run(move |new_event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match &new_event {
                Event::MainEventsCleared => {
                    for WindowedObject { window, .. } in self.window_objects.values() {
                        window.request_redraw()
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } => {
                    if let Some(windowed_object) = self.window_objects.get_mut(window_id) {
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::Destroyed
                            | WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        ..
                                    },
                                ..
                            } => {
                                *control_flow = ControlFlow::Exit;
                            }
                            WindowEvent::Resized(physical_size) => {
                                windowed_object.object.resize(*physical_size);
                            }
                            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                windowed_object.object.resize(**new_inner_size);
                            }
                            WindowEvent::Focused(focus) => windowed_object.focused = *focus,
                            _ => {
                                if windowed_object.focused {
                                    windowed_object
                                        .object
                                        .handle_event(&new_event, &windowed_object.window)
                                }
                            }
                        }
                    }
                }
                Event::RedrawRequested(window_id) => {
                    if let Some(windowed_object) = self.window_objects.get_mut(window_id) {
                        windowed_object
                            .object
                            .handle_event(&new_event, &windowed_object.window)
                    }
                }
                Event::UserEvent(RenderEvent { window_id, .. }) => {
                    if let Some(windowed_object) = self.window_objects.get_mut(window_id) {
                        windowed_object
                            .object
                            .handle_event(&new_event, &windowed_object.window)
                    }
                }
                _ => {
                    for (
                        _,
                        WindowedObject {
                            object,
                            window,
                            focused,
                        },
                    ) in self.window_objects.iter_mut()
                    {
                        if *focused {
                            object.handle_event(&new_event, window);
                        }
                    }
                }
            }
        });
    }
}
