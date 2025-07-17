use glam::Vec2;
use wgpu::util::DeviceExt;

use crate::util::create_render_pipeline;

pub struct ClipmapLevel {
    pub elevation: wgpu::TextureView,
    pub normal_map: wgpu::TextureView,
}

impl ClipmapLevel {
    pub fn new(device: &wgpu::Device, grid_size: u32) -> Self {
        let elevation_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Elevation Texture"),
            size: wgpu::Extent3d {
                width: grid_size,
                height: grid_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // 2. Create normal texture: 4-channel 8-bit (RGBA8Unorm) for packing fine/coarse normals
        let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Normal Texture"),
            size: wgpu::Extent3d {
                width: grid_size * 2, // optional: use higher-res for finer shading
                height: grid_size * 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self {
            elevation: elevation_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            normal_map: normal_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        }
    }
}

pub struct GeometryClipmap {
    pub levels: Vec<ClipmapLevel>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
}

impl GeometryClipmap {
    pub fn new(device: &wgpu::Device, grid_size: u32, num_levels: usize) -> Self {
        let block_size = (grid_size as u16 + 1) / 4;

        let levels = (0..num_levels)
            .map(|_| ClipmapLevel::new(device, grid_size))
            .collect();
        let vertex_buffer = Self::create_grid_vertex_buffer(device, block_size);
        let index_buffer = Self::create_grid_index_buffer(device, block_size);
        // Define vertex format: just vec2<f32>
        let vertex_layouts = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }];

        // Create pipeline layout
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Pipeline Layout"),
            bind_group_layouts: &[/* your texture + uniform bind groups here */],
            push_constant_ranges: &[],
        });

        let pipeline = create_render_pipeline(
            device,
            &layout,
            color_format,
            depth_format,
            &vertex_layouts,
            wgpu::include_wgsl!("terrain.wgsl"),
        );

        Self {
            levels,
            vertex_buffer,
            index_buffer,
            pipeline,
        }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass, camera_pos: [f32; 2]) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        for level in &self.levels {
            bind_textures_and_uniforms(pass, level, camera_pos);
            pass.draw_indexed(0..num_indices, 0, 0..1);
        }
    }

    fn create_grid_vertex_buffer(device: &wgpu::Device, block_size: u16) -> wgpu::Buffer {
        let mut vertices = Vec::with_capacity(block_size as usize);

        for y in 0..block_size {
            for x in 0..block_size {
                vertices.push(Vec2::new(x as f32, y as f32));
            }
        }

        let raw_bytes = bytemuck::cast_slice(&vertices);

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: raw_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    fn create_grid_index_buffer(device: &wgpu::Device, block_size: u16) -> wgpu::Buffer {
        // Number of vertices along one dimension
        let verts_per_row = block_size;
        let mut indices: Vec<u16> = Vec::new();

        for y in 0..(block_size - 1) {
            for x in 0..verts_per_row {
                let i0 = y * verts_per_row + x;
                let i1 = (y + 1) * verts_per_row + x;
                indices.push(i0);
                indices.push(i1);
            }

            // Insert degenerate triangle (duplicate last vertex) unless it's the last strip
            if y < block_size - 2 {
                let last_i1 = (y + 1) * verts_per_row + (verts_per_row - 1);
                let next_i0 = (y + 1) * verts_per_row;
                indices.push(last_i1);
                indices.push(next_i0);
            }
        }

        let raw_bytes = bytemuck::cast_slice(&indices);

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Index Buffer"),
            contents: raw_bytes,
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}
