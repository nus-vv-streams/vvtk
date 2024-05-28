use cgmath::*;
use std::f32::consts::{FRAC_PI_2, PI};
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;

const CAMERA_SPEED: f32 = 1.0;
const CAMERA_SENSITIVITY: f32 = 0.2;
const PROJECTION_FOXY: f32 = 45.0;
const PROJECTION_ZNEAR: f32 = 0.001;
const PROJECTION_ZFAR: f32 = 100.0;

#[derive(Clone)]
pub struct CameraState {
    pub(super) camera: Camera,
    camera_controller: CameraController,
    pub(super) camera_uniform: CameraUniform,
    projection: Projection,
    mouse_pressed: bool,
    window_size: winit::dpi::PhysicalSize<u32>,
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
            CameraController::new(CAMERA_SPEED, CAMERA_SENSITIVITY, camera.clone());
        let mut camera_uniform = CameraUniform::default();
        camera_uniform.update_view_proj(&camera, &projection);

        Self {
            camera,
            camera_controller,
            camera_uniform,
            projection,
            mouse_pressed: false,
            window_size: winit::dpi::PhysicalSize::new(width, height),
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

    /// Update camera position
    pub fn update_camera_pos(&mut self, camera_pos: CameraPosition) {
        self.camera.current = camera_pos;
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.window_size = new_size;
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

    pub fn coincident_plane(&self, point: [f32; 3]) -> [f32; 3] {
        let point = Point3::from(point);
        let view_proj = self.projection.matrix() * self.camera.calc_matrix();
        let point_t = view_proj.transform_point(point);

        // if not in the NDC space, return the original point
        if point_t.x.abs() > 1.0 || point_t.y.abs() > 1.0 || point_t.z > 1.0 {
            return point.into();
        }

        let midpoint = Point3::new(0.0, 0.0, point_t.z);
        let inv_view_proj = view_proj.inverse_transform().unwrap();

        let res = inv_view_proj.transform_point(midpoint);
        res.into()
    }

    pub fn distance(&self, point: [f32; 3]) -> f32 {
        let point = Point3::from(point);
        (point - self.camera.position).magnitude()
    }

    pub fn get_plane_at_z(&self, z: f32) -> (f32, f32) {
        let fovy = self.projection.fovy;
        let aspect = self.projection.aspect;
        let height = 2.0 * z * (fovy / 2.0).tan();
        let width = height * aspect;

        (width, height)
    }

    pub fn get_window_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window_size
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// Create a uniform buffer: a blob of data that is available to every invocation of a set of shaders.
/// This buffer is used to store our view projection matrix
pub struct CameraUniform {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
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
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[allow(dead_code)]
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Copy, Clone)]
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
    pub up: Vector3<f32>, // either unit_y or -unit_y
}

impl Default for CameraPosition {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            yaw: Rad(0.0),
            pitch: Rad(0.0),
            up: Vector3::unit_y(),
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
            up: Vector3::unit_y(),
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
            self.up,
        )
    }

    /// Resets camera to its first state
    fn reset(&mut self) {
        self.current = self.orig;
    }
}

#[derive(Clone)]
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

#[derive(Debug, Clone)]
enum RotateDirection {
    HorizontalClockwise,
    HorizontalCounterClockwise,
    VerticalClockwise,
    VerticalCounterClockwise,
    NoRotation,
}

#[derive(Debug, Clone)]
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
    initial_camera: Camera,
    rotate_direction: RotateDirection,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, initial_camera: Camera) -> Self {
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
            initial_camera,
            rotate_direction: RotateDirection::NoRotation,
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
            VirtualKeyCode::L => {
                self.rotate_direction = RotateDirection::HorizontalClockwise;
                true
            }
            VirtualKeyCode::J => {
                self.rotate_direction = RotateDirection::HorizontalCounterClockwise;
                true
            }
            VirtualKeyCode::I => {
                self.rotate_direction = RotateDirection::VerticalClockwise;
                true
            }
            VirtualKeyCode::K => {
                self.rotate_direction = RotateDirection::VerticalCounterClockwise;
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

    fn update_camera_rotation(&mut self, camera: &mut Camera, dt: Duration) {
        let rotate_angle_speed = std::f32::consts::PI; // 90 degree per second
        let rotate_angle = Rad(rotate_angle_speed * dt.as_secs_f32());
        let clock_wise = match self.rotate_direction {
            RotateDirection::HorizontalClockwise | RotateDirection::VerticalClockwise => 1.0f32,
            RotateDirection::HorizontalCounterClockwise
            | RotateDirection::VerticalCounterClockwise => -1f32,
            _ => 0f32,
        };
        match self.rotate_direction {
            RotateDirection::HorizontalClockwise | RotateDirection::HorizontalCounterClockwise => {
                // the camera rotates around the y axis, radius is the abs(y) which stays the unchanged
                // radius = sqrt(x^2 + z^2)
                let radius = camera.position.x.hypot(camera.position.z);
                // get the angle of the camera position with z axis
                let z_angle = Rad(camera.position.x.atan2(camera.position.z));
                let updated_z_angle = z_angle + rotate_angle * clock_wise;
                let updated_z_angle = updated_z_angle % Rad(2.0 * std::f32::consts::PI);

                // y remains the same, x and z change based on rotate_angle
                camera.position.x = updated_z_angle.0.sin() * radius;
                camera.position.z = updated_z_angle.0.cos() * radius;
                // yaw needs to be updated as well
                camera.yaw = camera.yaw - rotate_angle * clock_wise;
                camera.yaw = camera.yaw % Rad(2.0 * std::f32::consts::PI);
            }

            RotateDirection::VerticalClockwise | RotateDirection::VerticalCounterClockwise => {
                // the camera rotates around the x axis, radius is the abs(x) which stays the unchanged
                // radius = sqrt(y^2 + z^2)
                let radius = camera.position.y.hypot(camera.position.z);
                // get the angle of the camera position with z axis
                let z_angle = Rad(camera.position.y.atan2(camera.position.z));
                let updated_z_angle = z_angle + rotate_angle * clock_wise;
                let updated_z_angle = updated_z_angle % Rad(2.0 * std::f32::consts::PI);

                // x remains the same, y and z change based on rotate_angle
                camera.position.y = updated_z_angle.0.sin() * radius;
                camera.position.z = updated_z_angle.0.cos() * radius;
                // pitch needs to be updated as well
                camera.pitch = camera.pitch - rotate_angle * clock_wise;
                camera.pitch = camera.pitch % Rad(2.0 * std::f32::consts::PI);
                camera.up = camera.pitch.cos().signum() * Vector3::new(0.0, 1.0, 0.0);
            }
            _ => {}
        }
        self.rotate_direction = RotateDirection::NoRotation;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        if self.reset_view_requested {
            camera.reset();
            self.reset_view_requested = false;
            return;
        }
        if self.if_reset {
            camera.position = self.initial_camera.position.clone();
            camera.yaw = self.initial_camera.yaw.clone();
            camera.pitch = self.initial_camera.pitch.clone();
            camera.up = self.initial_camera.up.clone();
            self.if_reset = false;
        }

        self.update_camera_rotation(camera, dt);

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

        // Keep the camera's angle from going too high/low.
        // if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
        //     camera.pitch = -Rad(SAFE_FRAC_PI_2);
        // } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
        //     camera.pitch = Rad(SAFE_FRAC_PI_2);
        // }
        // since we want to rotate the camera vertically, we don't need to limit the pitch
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
