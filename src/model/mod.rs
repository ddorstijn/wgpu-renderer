use anyhow::Result;
use material::Material;
use mesh::{Mesh, ModelVertex};
use wgpu::util::DeviceExt;

use crate::{State, texture::Texture};

pub mod material;
pub mod mesh;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub struct Model {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn from_obj(
        state: &State,
        layout: &wgpu::BindGroupLayout,
        file_path: &str,
    ) -> Result<Self> {
        // Load the OBJ file
        let (models, materials) = tobj::load_obj(file_path, &tobj::LoadOptions::default())?;

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| ModelVertex {
                        position: [
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ],
                        tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                        normal: [0.0, 0.0, 0.0],
                    })
                    .collect::<Vec<_>>();

                let vertex_buffer =
                    state
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{:?} Vertex Buffer", file_path)),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    state
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{:?} Index Buffer", file_path)),
                            contents: bytemuck::cast_slice(&m.mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                Mesh {
                    name: file_path.to_string(),
                    vertex_buffer,
                    index_buffer,
                    index_count: m.mesh.indices.len() as u32,
                    material_index: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        let materials: Result<Vec<Material>> = if let Ok(materials) = materials {
            materials
                .iter()
                .map(|m| {
                    if let Some(diffuse_texture) = m.diffuse_texture.as_ref() {
                        let diffuse_texture = Texture::from_bytes(
                            &state.device,
                            &state.queue,
                            load_binary(diffuse_texture.as_str())?.as_slice(),
                            file_path,
                        )?;

                        let bind_group =
                            state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(
                                            &diffuse_texture.view,
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(
                                            &diffuse_texture.sampler,
                                        ),
                                    },
                                ],
                                label: Some(&format!("{:?} Bind Group", file_path)),
                            });

                        Ok(Material {
                            name: m.name.clone(),
                            diffuse_texture: Some(diffuse_texture),
                            bind_group: Some(bind_group),
                        })
                    } else {
                        Ok(Material {
                            name: m.name.clone(),
                            diffuse_texture: None,
                            bind_group: None,
                        })
                    }
                })
                .collect()
        } else {
            Ok(vec![])
        };

        Ok(Self {
            name: file_path.to_string(),
            meshes,
            materials: materials?,
        })
    }
}

pub fn load_string(file_name: &str) -> Result<String> {
    let txt = std::fs::read_to_string(file_name)?;

    Ok(txt)
}

pub fn load_binary(file_name: &str) -> Result<Vec<u8>> {
    let data = std::fs::read(file_name)?;

    Ok(data)
}
