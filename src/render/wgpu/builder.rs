use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use std::time::{Duration, Instant};
use crate::render::wgpu::camera::Camera;

#[derive(Debug)]
pub struct RenderEvent {
    pub(crate) window_id: WindowId,
    pub(crate) event_type: EventType
}

#[derive(Debug)]
pub enum EventType {
    MoveTo(usize),
    Toggle,
    Info(RenderInformation),
    Repaint
}

#[derive(Debug, Clone, Copy)]
pub struct RenderInformation {
    pub camera: Camera,
    pub current_position: usize,
    pub fps: f32,
}

pub trait Attachable {
    type Output: Windowed;
    fn attach(self, event_loop: &EventLoop<RenderEvent>) -> Self::Output;
}

pub trait Windowed {
    fn handle_event(&mut self, event: &Event<RenderEvent>, control: &mut ControlFlow);
}

pub struct RenderBuilder {
    event_loop: EventLoop<RenderEvent>,
    windows: Vec<Rc<RefCell<dyn Windowed>>>,
}

impl RenderBuilder {
    pub fn new() -> Self {
        Self {
            event_loop: EventLoop::with_user_event(),
            windows: Vec::new()
        }
    }

    pub fn add_window<T>(&mut self, attachable: T) -> Rc<RefCell<T::Output>>
        where T: Attachable, <T as Attachable>::Output: 'static + Windowed {
        let windowed = attachable.attach(&self.event_loop);
        let windowed = Rc::new(RefCell::new(windowed));
        self.windows.push(windowed.clone());
        windowed
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            if let Event::WindowEvent { ref event, .. } = event {
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
                    } => {
                        *control_flow = ControlFlow::Exit;
                        return;
                    },
                    _ => {}
                }
            }

            for window in self.windows.iter_mut() {
                (**window).borrow_mut().handle_event(&event, control_flow);
            }
        });
    }
}