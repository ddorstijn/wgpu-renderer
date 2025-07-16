use glam::{Mat4, Vec3};

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
    pub camera_sensitivity: f32,
    pub climb_speed: f32,
    pub normal_move_speed: f32,
    pub slow_move_factor: f32,
    pub fast_move_factor: f32,

    rotation_x: f32,
    rotation_y: f32,
}

/// Represents a simple RTS camera with a Z-up coordinate system.
pub struct RtsCamera {
    /// The camera's position in world space.
    pub position: Vec3,
    /// The point the camera is looking at.
    pub target: Vec3,
    /// The up direction of the world. For Z-up, this is typically Vec3::Z.
    pub world_up: Vec3,
    /// The distance from the target. Used for zoom.
    pub distance: f32,
    /// The horizontal angle (yaw) around the Z-axis, in radians.
    pub yaw: f32,
    /// The vertical angle (pitch) from the Z-axis, in radians.
    /// 0 radians means looking straight down the Z-axis.
    /// PI/2 radians means looking along the XY plane.
    pub pitch: f32,
    /// The field of view in the Y direction (vertical), in radians.
    pub fov_y: f32,
    /// The aspect ratio (width / height) of the viewport.
    pub aspect_ratio: f32,
    /// The near clipping plane distance.
    pub near_plane: f32,
    /// The far clipping plane distance.
    pub far_plane: f32,
}

impl Default for RtsCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 10.0), // Start above the origin
            target: Vec3::ZERO,
            world_up: Vec3::Z, // Z is up
            distance: 15.0,
            yaw: 0.0,                           // Looking along positive Y
            pitch: std::f32::consts::FRAC_PI_3, // Looking down at an angle
            fov_y: std::f32::consts::FRAC_PI_2, // 90 degrees
            aspect_ratio: 16.0 / 9.0,
            near_plane: 0.1,
            far_plane: 1000.0,
        }
    }
}

impl RtsCamera {
    /// Updates the camera's position and target based on its angles and distance.
    fn update_position_from_angles(&mut self) {
        // Calculate the direction vector from angles
        // In a Z-up system:
        // pitch = 0 looks along +Z
        // pitch = PI/2 looks in XY plane
        // yaw rotates in the XY plane

        let x = self.distance * self.pitch.sin() * self.yaw.sin();
        let y = self.distance * self.pitch.sin() * self.yaw.cos();
        let z = self.distance * self.pitch.cos();

        let offset = Vec3::new(x, y, z);
        self.position = self.target + offset;
    }

    /// Moves the camera's target (and thus the camera) on the XY plane.
    /// `delta_x` moves left/right relative to camera's current view.
    /// `delta_y` moves forward/backward relative to camera's current view.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Get the camera's forward vector (projected onto XY plane)
        let mut forward_xy = (self.target - self.position).normalize();
        forward_xy.z = 0.0; // Ensure it's in the XY plane
        forward_xy = forward_xy.normalize();

        // Get the camera's right vector (projected onto XY plane)
        let mut right_xy = forward_xy.cross(self.world_up).normalize();
        right_xy.z = 0.0; // Ensure it's in the XY plane
        right_xy = right_xy.normalize();

        self.target += right_xy * delta_x;
        self.target += forward_xy * delta_y;

        self.update_position_from_angles(); // Recalculate position based on new target
    }

    /// Zooms the camera in or out by adjusting the distance from the target.
    pub fn zoom(&mut self, delta_distance: f32) {
        self.distance = (self.distance + delta_distance).max(1.0); // Prevent going through the target
        self.update_position_from_angles();
    }

    /// Rotates the camera around the world's Z-axis (yaw).
    pub fn rotate_yaw(&mut self, delta_yaw: f32) {
        self.yaw += delta_yaw;
        self.update_position_from_angles();
    }

    /// Tilts the camera up or down (pitch).
    /// Clamped to prevent flipping and looking completely vertically up or down.
    pub fn rotate_pitch(&mut self, delta_pitch: f32) {
        self.pitch += delta_pitch;
        // Clamp pitch to avoid gimbal lock or looking straight up/down
        self.pitch = self.pitch.clamp(0.1, std::f32::consts::PI - 0.1); // Avoid 0 and PI
        self.update_position_from_angles();
    }

    /// Returns the camera's view matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.world_up)
    }

    /// Returns the camera's projection matrix.
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y,
            self.aspect_ratio,
            self.near_plane,
            self.far_plane,
        )
    }

    /// Returns the combined view-projection matrix.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}
