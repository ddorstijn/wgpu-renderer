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
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);

        proj * view
    }
}

pub struct CameraController {
    pub speed: f32,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
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
            is_shift_pressed: false,
        }
    }

    pub fn process_key_events(&mut self, keycode: KeyCode, pressed: bool) {
        match keycode {
            KeyCode::KeyW => self.is_forward_pressed = pressed,
            KeyCode::KeyA => self.is_left_pressed = pressed,
            KeyCode::KeyS => self.is_backward_pressed = pressed,
            KeyCode::KeyD => self.is_right_pressed = pressed,
            KeyCode::ShiftLeft => self.is_shift_pressed = pressed,
            _ => {}
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let forward = (camera.target - camera.eye).normalize();
        let right = camera.up.cross(forward).normalize();

        if self.is_forward_pressed {
            match self.is_shift_pressed {
                true => {
                    camera.target -= Vec3::Y * self.speed;
                    camera.eye -= Vec3::Y * self.speed;
                }
                false => camera.eye += forward * self.speed,
            }
        }
        if self.is_backward_pressed {
            match self.is_shift_pressed {
                true => {
                    camera.target += Vec3::Y * self.speed;
                    camera.eye += Vec3::Y * self.speed;
                }
                false => camera.eye -= forward * self.speed * 10.0,
            }
        }
        if self.is_left_pressed {
            match self.is_shift_pressed {
                true => {
                    camera.target += Vec3::X * self.speed;
                    camera.eye += Vec3::X * self.speed;
                }
                false => camera.eye -= right * self.speed * 0.1,
            }
        }
        if self.is_right_pressed {
            match self.is_shift_pressed {
                true => {
                    camera.target -= Vec3::X * self.speed;
                    camera.eye -= Vec3::X * self.speed;
                }
                false => camera.eye += right * self.speed * 0.1,
            }
        }
    }
}
