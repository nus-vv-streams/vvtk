use cgmath::*;
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;

const CAMERA_SPEED: f32 = 2.0;
const CAMERA_SENSITIVITY: f32 = 0.5;
const PROJECTION_FOXY: f32 = 45.0;
const PROJECTION_ZNEAR: f32 = 0.1;
const PROJECTION_ZFAR: f32 = 100.0;

pub struct CameraState {
    pub(super) camera: Camera,
    camera_controller: CameraController,
    pub(super) camera_uniform: CameraUniform,
    projection: Projection,
    mouse_pressed: bool,
}

impl CameraState {
    pub fn new(camera: Camera, width: u32, height: u32) -> Self {
        let projection = Projection::new(
            width,
            height,
            cgmath::Deg(PROJECTION_FOXY),
            PROJECTION_ZNEAR,
            PROJECTION_ZFAR,
        );
        let camera_controller =
            CameraController::new(CAMERA_SPEED, CAMERA_SENSITIVITY, camera.position.clone());
        let mut camera_uniform = CameraUniform::default();
        camera_uniform.update_view_proj(&camera, &projection);

        Self {
            camera,
            camera_controller,
            camera_uniform,
            projection,
            mouse_pressed: false,
        }
    }

    pub fn create_buffer(
        &self,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[self.camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
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
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        (camera_buffer, camera_bind_group_layout, camera_bind_group)
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
        }
    }

    pub fn process_input(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => self.camera_controller.process_keyboard(*key, *state),
            DeviceEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            DeviceEvent::Button {
                button: 0, // in mac: touchpad pressed
                state,
            }
            | DeviceEvent::Button {
                button: 1, // Left Mouse Button
                state,
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            DeviceEvent::MouseMotion { delta } => {
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(delta.0, delta.1);
                }
                true
            }
            _ => false,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// Create a uniform buffer: a blob of data that is available to every invocation of a set of shaders.
/// This buffer is used to store our view projection matrix
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
}

impl CameraUniform {
    fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.matrix() * camera.calc_matrix()).into()
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Camera {
    current: CameraPosition,
    /// original position of the camera
    orig: CameraPosition,
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// The position of the camera
pub struct CameraPosition {
    pub position: Point3<f32>,
    /// Yaw is the rotation around the y axis
    /// - -90deg is looking down the negative z axis
    /// - 0deg is looking down the positive x axis
    pub yaw: Rad<f32>,
    /// Pitch is the rotation around the x axis
    /// - 0deg is looking down the z axis
    /// - 90deg is looking down the positive y axis
    pub pitch: Rad<f32>,
}

impl Default for CameraPosition {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            yaw: Rad(0.0),
            pitch: Rad(0.0),
        }
    }
}

impl Deref for Camera {
    type Target = CameraPosition;
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for Camera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

impl Camera {
    pub fn new<
        V: Into<Point3<f32>> + Clone,
        Y: Into<Rad<f32>> + Clone,
        P: Into<Rad<f32>> + Clone,
    >(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        let position = CameraPosition {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        };

        Self {
            current: position,
            orig: position,
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();

        Matrix4::look_to_rh(
            self.position,
            Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vector3::unit_y(),
        )
    }

    /// Resets camera to its first state
    fn reset(&mut self) {
        self.current = self.orig;
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    /// Get projection matrix
    pub fn matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
/// Used to control the camera movement by processing user inputs
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
    reset_view_requested: bool,
    if_reset: bool,
    initial_position: Point3<f32>,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, initial_position: Point3<f32>) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            reset_view_requested: false,
            if_reset: false,
            initial_position,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Q => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::E => {
                self.amount_down = amount;
                true
            }
            VirtualKeyCode::R => {
                self.reset_view_requested = true;
                true
            }
            VirtualKeyCode::Key0 => {
                self.if_reset = true;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        if self.reset_view_requested {
            camera.reset();
            self.reset_view_requested = false;
            return;
        }
        if self.if_reset {
            camera.position = self.initial_position.clone();
            self.if_reset = false;
        }

        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let scrollward =
            Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt * 0.5;

        // Rotate
        camera.yaw = delta_with_clamp(
            camera.yaw,
            Rad(self.rotate_horizontal) * self.sensitivity * dt,
        );
        camera.pitch = delta_with_clamp(
            camera.pitch,
            Rad(-self.rotate_vertical) * self.sensitivity * dt,
        );

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
    }
}

fn delta_with_clamp(orig: Rad<f32>, delta: Rad<f32>) -> Rad<f32> {
    let result = orig + delta;
    if result < -Rad(PI) {
        Rad(2.0 * PI) + result
    } else if result > Rad(PI) {
        -Rad(2.0 * PI) + result
    } else {
        result
    }
}
