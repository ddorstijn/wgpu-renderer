use anyhow::Result;
use mesh::Mesh;

pub mod material;
pub mod mesh;
pub mod texture;

pub struct Model {
    pub name: String,
    pub mesh: Mesh,
    pub material: Material,
}

impl Model {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        name: String,
        mesh: Mesh,
        material: Material,
    ) -> Self {
        Self {
            name,
            mesh,
            material,
        }
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let model = ModelLoader::load_model(bytes)?;
        Ok(model)
    }
}
