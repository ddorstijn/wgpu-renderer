use crate::texture;
use glam::Mat4;
use std::{ops::Range, path::Path};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub transform: Mat4,
}

impl Vertex for Instance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTRIBS,
        }
    }
}
#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load(
        path: &Path,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let (models, obj_materials) = tobj::load_obj(
            path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let material_path_buf = path.parent().unwrap().join(&m.diffuse_texture.unwrap());
            let material_path = material_path_buf.as_path();
            let diffuse_texture =
                crate::texture::Texture::load(Some(m.name.as_str()), device, queue, material_path)?;

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: m.name,
                diffuse_texture,
                bind_group,
            })
        }

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| {
                        if m.mesh.normals.is_empty() {
                            ModelVertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: [
                                    m.mesh.texcoords[i * 2],
                                    1.0 - m.mesh.texcoords[i * 2 + 1],
                                ],
                                normal: [0.0, 0.0, 0.0],
                            }
                        } else {
                            ModelVertex {
                                position: [
                                    m.mesh.positions[i * 3],
                                    m.mesh.positions[i * 3 + 1],
                                    m.mesh.positions[i * 3 + 2],
                                ],
                                tex_coords: [
                                    m.mesh.texcoords[i * 2],
                                    1.0 - m.mesh.texcoords[i * 2 + 1],
                                ],
                                normal: [
                                    m.mesh.normals[i * 3],
                                    m.mesh.normals[i * 3 + 1],
                                    m.mesh.normals[i * 3 + 2],
                                ],
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", path)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", path)),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: path.to_str().unwrap().to_owned(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Self { meshes, materials })
    }
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub trait Vertex {
    const ATTRIBS: &'static [wgpu::VertexAttribute];

    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3, // position
        1 => Float32x2, // tex_coords
        2 => Float32x3  // normal
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

pub trait DrawModel<'a> {
    #[allow(unused)]
    fn draw_mesh(&mut self, mesh: &'a Mesh, material: &'a Material);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
    );

    #[allow(unused)]
    fn draw_model(&mut self, model: &'a Model);
    fn draw_model_instanced(&mut self, model: &'a Model, instances: Range<u32>);
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh, material: &'b Material) {
        self.draw_mesh_instanced(mesh, material, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(2, &material.bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model) {
        self.draw_model_instanced(model, 0..1);
    }

    fn draw_model_instanced(&mut self, model: &'b Model, instances: Range<u32>) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone());
        }
    }
}
