use glam::{Mat4, Vec2, Vec3};
use std::path::Path;
use tobj::Material;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub transform: Mat4,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Model [IO] load error: {0}")]
    ModelLoadError(#[from] tobj::LoadError),
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load(path: &Path) -> Result<Self, ModelError> {
        let (models, materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let materials = materials?;

        let meshes = models
            .into_iter()
            .map(|m| {
                let normals = match m.mesh.normals.is_empty() {
                    true => vec![0f32; m.mesh.positions.len()],
                    false => m.mesh.normals,
                };

                let positions = m
                    .mesh
                    .positions
                    .chunks(3)
                    .zip(m.mesh.texcoords.chunks(2))
                    .zip(normals.chunks(3))
                    .map(|((pos, uv), normal)| Vertex {
                        position: Vec3::new(pos[0], pos[1], pos[2]),
                        uv: Vec2::new(uv[0], 1.0 - uv[1]),
                        normal: Vec3::new(normal[0], normal[1], normal[2]),
                    })
                    .collect::<Vec<_>>();

                Mesh {
                    name: path.to_str().unwrap().to_owned(),
                    positions,
                    indices: m.mesh.indices,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Self { meshes, materials })
    }
}

#[derive(Debug)]
pub struct Mesh {
    #[allow(unused)]
    pub name: String,
    pub positions: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: usize,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
}
