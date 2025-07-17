use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

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
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_key_events(&mut self, keycode: KeyCode, pressed: bool) {
        match keycode {
            KeyCode::KeyW | KeyCode::ArrowUp => self.is_forward_pressed = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.is_left_pressed = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.is_backward_pressed = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.is_right_pressed = pressed,
            _ => {}
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let forward = (camera.target - camera.eye).normalize();
        let right = camera.up.cross(forward).normalize();

        if self.is_forward_pressed {
            camera.eye += forward * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward * self.speed;
        }
        if self.is_left_pressed {
            camera.eye -= right * self.speed;
        }
        if self.is_right_pressed {
            camera.eye += right * self.speed;
        }
    }
}
