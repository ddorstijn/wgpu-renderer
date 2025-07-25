use glam::Vec3;
use wre_camera::Camera;
use wre_terrain::Terrain;

pub struct World {
    camera: Camera,
    terrain: Terrain,
}

impl World {
    fn new() -> Self {
        Self {
            camera: Camera {
                eye: Vec3::new(0.0, 0.0001, 10.0),
                target: Vec3::new(0.0, 0.0, 00.0),
                up: Vec3::Z,
                aspect: config.width as f32 / config.height as f32,
                fovy: 45.0f32.to_radians(),
                znear: 0.1,
                zfar: 10000.0,
            },
            terrain: Terrain::new(),
        }
    }
}
