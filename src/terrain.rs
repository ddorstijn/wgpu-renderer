use std::sync::Arc;

use glam::Vec2;
use wgpu::util::DeviceExt;

use crate::util::create_render_pipeline;

// In your main render state or engine structure
pub struct TerrainSystem {
    levels: Vec<ClipmapLevel>,
    render_pipeline: wgpu::RenderPipeline,

    // Shared mesh data, stored once
    vertex_buffer: Arc<wgpu::Buffer>,
    index_buffer: Arc<wgpu::Buffer>,
    index_count: u32,

    // Shared heightmap resources
    heightmap_texture: wgpu::Texture,
    heightmap_view: wgpu::TextureView,
    heightmap_sampler: wgpu::Sampler,
    shared_bind_group: wgpu::BindGroup,
}

// Represents a single level of the clipmap. Note it no longer contains buffers.
struct ClipmapLevel {
    // Each level has its own uniform buffer for scale and offset
    uniform_buffer: wgpu::Buffer,
    // The bind group connects the uniform buffer to the shader
    bind_group: wgpu::BindGroup,

    // CPU-side tracking
    current_offset: (i32, i32),
    scale: f32,
}

impl TerrainSystem {
    // M is the resolution of your grid, e.g., 255
    const M: usize = 255;

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        render_format: wgpu::TextureFormat,
    ) -> Self {
        // --- Heightmap Generation (Placeholder) ---
        let heightmap_size = 1024;
        let heightmap_data: Vec<f32> = vec![0.5; heightmap_size * heightmap_size]; // Flat terrain
        let heightmap_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("Heightmap Texture"),
                size: wgpu::Extent3d {
                    width: heightmap_size as u32,
                    height: heightmap_size as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(&heightmap_data),
        );
        let heightmap_view = heightmap_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let heightmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Heightmap Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // --- Mesh Generation ---
        let (vertex_buffer, index_buffer, index_count) = Self::create_grid_mesh(device);

        // --- Bind Group Layouts ---
        let shared_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Terrain Shared BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let level_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Terrain Level BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // --- Bind Groups ---
        let shared_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Shared BG"),
            layout: &shared_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&heightmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&heightmap_sampler),
                },
            ],
        });

        // --- Render Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Pipeline Layout"),
                bind_group_layouts: &[
                    camera_bind_group_layout,
                    &shared_bind_group_layout,
                    &level_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = create_render_pipeline(
            device,
            &render_pipeline_layout,
            render_format,
            &[TerrainVertex::desc()],
            wgpu::include_wgsl!("terrain.wgsl"),
        );

        // --- Create Clipmap Levels ---
        let num_levels = 6;
        let mut levels = Vec::with_capacity(num_levels);
        for i in 0..num_levels {
            let scale = 2.0f32.powi(i as i32);

            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Level {} Uniform Buffer", i)),
                size: std::mem::size_of::<LevelUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Level {} BG", i)),
                layout: &level_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            levels.push(ClipmapLevel {
                uniform_buffer,
                bind_group,
                current_offset: (0, 0),
                scale,
            });
        }

        Self {
            levels,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            heightmap_texture,
            heightmap_view,
            heightmap_sampler,
            shared_bind_group,
        }
    }

    fn create_grid_mesh(device: &wgpu::Device) -> (Arc<wgpu::Buffer>, Arc<wgpu::Buffer>, u32) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for j in 0..=Self::M {
            for i in 0..=Self::M {
                vertices.push(TerrainVertex {
                    // Position vertices in a [-0.5, 0.5] range
                    position: Vec2::new(
                        i as f32 / Self::M as f32 - 0.5,
                        j as f32 / Self::M as f32 - 0.5,
                    ),
                });
            }
        }

        for j in 0..Self::M {
            for i in 0..Self::M {
                let row1 = (j * (Self::M + 1)) as u32;
                let row2 = ((j + 1) * (Self::M + 1)) as u32;
                indices.push(row1 + i as u32);
                indices.push(row2 + i as u32);
                indices.push(row1 + (i + 1) as u32);
                indices.push(row1 + (i + 1) as u32);
                indices.push(row2 + i as u32);
                indices.push(row2 + (i + 1) as u32);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (
            Arc::new(vertex_buffer),
            Arc::new(index_buffer),
            indices.len() as u32,
        )
    }

    pub fn update_terrain_system(&mut self, queue: &wgpu::Queue, camera_position: glam::Vec3) {
        for level in &mut self.levels {
            let grid_cell_size = level.scale;
            let camera_x = (camera_position.x / grid_cell_size).floor() as i32;
            let camera_z = (camera_position.z / grid_cell_size).floor() as i32;

            if camera_x != level.current_offset.0 || camera_z != level.current_offset.1 {
                level.current_offset = (camera_x, camera_z);
                let new_offset_x = camera_x as f32 * grid_cell_size;
                let new_offset_z = camera_z as f32 * grid_cell_size;
                let uniforms = LevelUniforms {
                    offset_scale: [new_offset_x, new_offset_z, level.scale, 0.0],
                };
                queue.write_buffer(&level.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
            }
        }
    }

    pub fn render<'rpass>(
        &'rpass self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        camera_bind_group: &'rpass wgpu::BindGroup,
    ) {
        rpass.set_pipeline(&self.render_pipeline);

        // Set shared bind groups once
        rpass.set_bind_group(0, camera_bind_group, &[]);
        rpass.set_bind_group(1, &self.shared_bind_group, &[]);

        // Set shared mesh buffers once
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // Iterate through the levels to draw them
        for level in &self.levels {
            // Set the per-level bind group
            rpass.set_bind_group(2, &level.bind_group, &[]);
            rpass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }
}

// Uniforms sent to the GPU for each level
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LevelUniforms {
    // We use a vec4 for alignment reasons. xy is the offset, z is scale.
    offset_scale: [f32; 4],
}

// The vertex layout for our grid mesh
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TerrainVertex {
    // We only need a 2D position for the grid, the Y value comes from the heightmap
    position: Vec2,
}

impl TerrainVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
