use crate::texture;

pub struct Material {
    pub name: String,
    pub diffuse_texture: Option<texture::Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
}
