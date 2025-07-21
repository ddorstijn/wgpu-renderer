use std::time::Duration;

use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

const SPEED_MODIFIER: f32 = 50.0;

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection(&self) -> Mat4 {
        let view = Mat4::look_at_lh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_lh(self.fovy, self.aspect, self.znear, self.zfar);

        proj * view
    }
}

pub struct CameraController {
    pub speed: f32,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_space_pressed: bool,
    pub is_control_pressed: bool,
    pub is_shift_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_space_pressed: false,
            is_control_pressed: false,
            is_shift_pressed: false,
        }
    }

    pub fn process_key_events(&mut self, keycode: KeyCode, pressed: bool) {
        match keycode {
            KeyCode::KeyW => self.is_forward_pressed = pressed,
            KeyCode::KeyA => self.is_left_pressed = pressed,
            KeyCode::KeyS => self.is_backward_pressed = pressed,
            KeyCode::KeyD => self.is_right_pressed = pressed,
            KeyCode::Space => self.is_space_pressed = pressed,
            KeyCode::ControlLeft => self.is_control_pressed = pressed,
            KeyCode::ShiftLeft => self.is_shift_pressed = pressed,
            _ => {}
        }
    }

    pub fn update_camera(&self, camera: &mut Camera, delta_time: Duration) {
        let forward = Vec3::Y; //(camera.target - camera.eye).normalize();
        let right = Vec3::X; //camera.up.cross(forward).normalize();
        let up = Vec3::Z; //camera.up;

        let modifier = if self.is_shift_pressed {
            SPEED_MODIFIER
        } else {
            1.0
        };

        if self.is_forward_pressed {
            camera.target -= forward * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye -= forward * self.speed * modifier * delta_time.as_secs_f32();
        }
        if self.is_backward_pressed {
            camera.target += forward * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye += forward * self.speed * modifier * delta_time.as_secs_f32();
        }
        if self.is_left_pressed {
            camera.target -= right * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye -= right * self.speed * modifier * delta_time.as_secs_f32();
        }
        if self.is_right_pressed {
            camera.target += right * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye += right * self.speed * modifier * delta_time.as_secs_f32();
        }

        if self.is_space_pressed {
            camera.target += up * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye += up * self.speed * modifier * delta_time.as_secs_f32();
        }
        if self.is_control_pressed {
            camera.target -= up * self.speed * modifier * delta_time.as_secs_f32();
            camera.eye -= up * self.speed * modifier * delta_time.as_secs_f32();
        }
    }
}
