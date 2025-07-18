use std::path::Path;

use glam::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use wgpu::util::DeviceExt;

use crate::{model::VertexAttribute, texture, util::create_render_pipeline};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    transform: Mat4,
    color: Vec4,
}

impl VertexAttribute for InstanceData {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
       // start at location 1 because Vec2 position is at 0
       1 => Float32x4,
       2 => Float32x4,
       3 => Float32x4,
       4 => Float32x4,

       5 => Float32x4,
    ];

    /// VertexBufferLayout for a 4×4 matrix at locations 1..4, instance‐step.
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: Self::ATTRIBS,
        }
    }
}

// In your main render state or engine structure
pub struct TerrainSystem {
    levels: Vec<ClipmapLevel>,

    tile: Mesh2d,
    filler: Mesh2d,
    trim: Mesh2d,
    cross: Mesh2d,
    seam: Mesh2d,

    #[allow(unused)]
    heightmap: texture::Texture, // Later used for editing
    render_pipeline: wgpu::RenderPipeline,
    shared_bind_group: wgpu::BindGroup,

    // per‐mesh instance buffers & counts
    tile_instances: wgpu::Buffer,
    filler_instances: wgpu::Buffer,
    trim_instances: wgpu::Buffer,
    cross_instances: wgpu::Buffer,
    seam_instances: wgpu::Buffer,

    tile_count: u32,
    filler_count: u32,
    trim_count: u32,
    cross_count: u32,
    seam_count: u32,
}

// Represents a single level of the clipmap. Note it no longer contains buffers.
struct ClipmapLevel {
    scale: f32,
    size: Vec2,
}

impl TerrainSystem {
    const TILE_RESOLUTION: u32 = 48;
    const PATCH_VERT_RESOLUTION: u32 = Self::TILE_RESOLUTION + 1;
    const CLIPMAP_RESOLUTION: u32 = Self::TILE_RESOLUTION * 4 + 1;
    const CLIPMAP_VERT_RESOLUTION: u32 = Self::CLIPMAP_RESOLUTION + 1;
    const NUM_LEVELS: usize = 7;

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        render_format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let heightmap = texture::Texture::load(
            Some("Terrain Heightmap"),
            device,
            queue,
            Path::new("assets/heightmap.png"),
        )?;

        let tile = Self::generate_tile_mesh(device);
        let filler = Self::generate_filler_mesh(device);
        let trim = Self::generate_trim_mesh(device);
        let cross = Self::generate_cross_mesh(device);
        let seam = Self::generate_seam_mesh(device);

        // --- Bind Group Layouts ---
        let heightmap_bind_group_layout =
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

        // --- Bind Groups ---
        let shared_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Shared BG"),
            layout: &heightmap_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&heightmap.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&heightmap.sampler),
                },
            ],
        });

        // --- Render Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Pipeline Layout"),
                bind_group_layouts: &[camera_bind_group_layout, &heightmap_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = create_render_pipeline(
            device,
            &render_pipeline_layout,
            render_format,
            &[Vec2::desc(), InstanceData::desc()],
            wgpu::include_wgsl!("terrain.wgsl"),
        );

        // --- Pre‐allocate instance buffers for maximum possible instances ---
        // 4×4 tiles per level × NUM_LEVELS
        let max_tiles = (Self::NUM_LEVELS * 4 * 4) as u64;
        // 1 filler cross per level
        let max_cross = Self::NUM_LEVELS as u64;
        // 1 filler ring per level
        let max_filler = Self::NUM_LEVELS as u64;
        // and so on…
        let max_trim = Self::NUM_LEVELS as u64;
        let max_seam = Self::NUM_LEVELS as u64;

        let alloc = |count| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: std::mem::size_of::<InstanceData>() as u64 * count,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        Ok(Self {
            levels: (0..Self::NUM_LEVELS)
                .map(|i| {
                    let scale = (1u32 << i) as f32;
                    let size = Vec2::splat((Self::TILE_RESOLUTION << i) as f32);
                    ClipmapLevel { scale, size }
                })
                .collect(),

            tile,
            filler,
            trim,
            cross,
            seam,
            heightmap,
            render_pipeline,
            shared_bind_group,

            tile_instances: alloc(max_tiles),
            filler_instances: alloc(max_filler),
            trim_instances: alloc(max_trim),
            cross_instances: alloc(max_cross),
            seam_instances: alloc(max_seam),

            tile_count: 0,
            filler_count: 0,
            trim_count: 0,
            cross_count: 0,
            seam_count: 0,
        })
    }

    pub fn update_terrain_system(&mut self, queue: &wgpu::Queue, camera_position: Vec3) {
        // We'll accumulate instance data here
        let mut tile_data = Vec::new();
        let mut filler_data = Vec::new();
        let mut trim_data = Vec::new();
        let mut cross_data = Vec::new();
        let mut seam_data = Vec::new();

        // The main 4×4 tile ring & filler/trim/seam per level
        for (i, level) in self.levels.iter().enumerate() {
            let s = level.scale;
            let size = level.size;
            // snapped camera for this LOD
            let snap = (camera_position.xz() / s).floor() * s;
            // bottom‐left corner of 4×4 grid
            let base = snap - size * 2.0;

            // --- 4×4 Tiles (skip middle 2×2 if not finest) ---
            for x in 0..4 {
                for y in 0..4 {
                    if i != 0 && (1..=2).contains(&x) && (1..=2).contains(&y) {
                        continue;
                    }
                    let fill = Vec2::new(x.clamp(0, 1) as f32, y.clamp(0, 1) as f32) * s;
                    let bl = base + Vec2::new(x as f32, y as f32) * size + fill;
                    // build transform = T(bl.xy) * S(s)
                    let scale_matrix = Mat4::from_scale(Vec3::new(s, 1.0, s));
                    let translation_matrix = Mat4::from_translation(Vec3::new(bl.x, 0.0, bl.y));
                    let xf = translation_matrix * scale_matrix;
                    tile_data.push(InstanceData {
                        transform: xf,
                        color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                    });
                }
            }

            // --- Cross ---
            {
                let snap = camera_position.xz().floor(); // integer world‐)xy
                let xf = Mat4::from_translation(Vec3::new(snap.x, 0.0, snap.y));
                cross_data.push(InstanceData {
                    transform: xf,
                    color: Vec4::new(0.0, 1.0, 0.0, 1.0),
                });
            }

            // --- Filler ring ---
            {
                let snap = (camera_position / level.scale).floor() * level.scale;

                let xf = Mat4::from_translation(Vec3::new(snap.x, 0.0, snap.y));
                filler_data.push(InstanceData {
                    transform: xf,
                    color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                });
            }

            // --- Seam (not outermost) ---
            if i + 1 < self.levels.len() {
                let next_s = self.levels[i + 1].scale;
                let snap2 = (camera_position.xz() / next_s).floor() * next_s;
                let base2 = snap2 - Vec2::splat((Self::TILE_RESOLUTION << (i + 1)) as f32);
                // one seam mesh per level
                let scale_matrix = Mat4::from_scale(Vec3::new(s, 1.0, s));
                let transform_matrix = Mat4::from_translation(Vec3::new(base2.x, 0.0, base2.y));
                let xf = transform_matrix * scale_matrix;
                seam_data.push(InstanceData {
                    transform: xf,
                    color: Vec4::new(1.0, 1.0, 0.0, 1.0),
                });
            }

            // --- Trim (one per level except outermost) ---
            if i + 1 < self.levels.len() {
                let d = camera_position.xz() - next_snap;

                // 1) Decide which half of that outer cell the camera lies in:
                let in_right_half = d.x >= scale;
                let in_top_half = d.y >= scale;

                // 2) We actually want to place the trim in the *opposite* corner
                //    so we flip those booleans:
                let x_flip = (!in_right_half) as u32; // bottom-half => put trim on top
                let y_flip = (!in_top_half) as u32; // left-half   => put trim on right

                // 3) Pack into a 0..3 index (bit2 = x, bit1 = y)
                let rot_idx = (x_flip << 1) | y_flip;

                // 4) Hardcode your 4 rotations so there’s no floating-point wiggle
                //    (identity, 90°, 270°, 180° for example)
                let rotations = [
                    Mat4::IDENTITY,                   // 00: bottom-left → bottom-left
                    Mat4::from_rotation_z(-PI * 0.5), // 01: bottom-left → bottom-right
                    Mat4::from_rotation_z(PI * 0.5),  // 10: bottom-left → top-left
                    Mat4::from_rotation_z(PI),        // 11: bottom-left → top-right
                ];

                // 5) Build your final transform in the correct order:
                //    scale first (on XZ), then translate
                let S = Mat4::from_scale(Vec3::new(scale, 1.0, scale));
                let T = Mat4::from_translation(Vec3::new(tile_center.x, 0.0, tile_center.y));
                let xf = T * S * rotations[rot_idx as usize];

                // push it into your trim_data
                trim_data.push(InstanceData {
                    transform: xf.to_cols_array_2d(),
                });

                let center = snap + Vec2::splat(0.5 * s);
                // compute rotation index (0..3) here if you want; for brevity assume ID
                let scale_matrix = Mat4::from_scale(Vec3::new(s, 1.0, s));
                let tramslation_matrix = Mat4::from_translation(Vec3::new(center.x, 0.0, center.y));
                let xf = tramslation_matrix * scale_matrix;
                trim_data.push(InstanceData {
                    transform: xf,
                    color: Vec4::new(0.0, 1.0, 1.0, 1.0),
                });
            }
        }

        // 3) Upload each to its GPU buffer
        self.tile_count = tile_data.len() as u32;
        self.cross_count = cross_data.len() as u32;
        self.seam_count = seam_data.len() as u32;
        self.trim_count = trim_data.len() as u32;
        self.filler_count = filler_data.len() as u32;

        queue.write_buffer(&self.tile_instances, 0, bytemuck::cast_slice(&tile_data));
        queue.write_buffer(&self.cross_instances, 0, bytemuck::cast_slice(&cross_data));
        queue.write_buffer(&self.seam_instances, 0, bytemuck::cast_slice(&seam_data));
        queue.write_buffer(&self.trim_instances, 0, bytemuck::cast_slice(&trim_data));
        queue.write_buffer(
            &self.filler_instances,
            0,
            bytemuck::cast_slice(&filler_data),
        );
    }

    pub fn render<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        rpass.set_pipeline(&self.render_pipeline);

        // shared bind groups
        rpass.set_bind_group(0, camera_bind_group, &[]);
        rpass.set_bind_group(1, &self.shared_bind_group, &[]);

        // DRAW TILE INSTANCES
        rpass.set_vertex_buffer(0, self.tile.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.tile_instances.slice(..));
        rpass.set_index_buffer(self.tile.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..self.tile.index_count, 0, 0..self.tile_count);

        // DRAW CROSS INSTANCES
        rpass.set_vertex_buffer(0, self.cross.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.cross_instances.slice(..));
        rpass.set_index_buffer(self.cross.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..self.cross.index_count, 0, 0..self.cross_count);

        // DRAW SEAM INSTANCES
        rpass.set_vertex_buffer(0, self.seam.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.seam_instances.slice(..));
        rpass.set_index_buffer(self.seam.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..self.seam.index_count, 0, 0..self.seam_count);

        // DRAW TRIM INSTANCES
        rpass.set_vertex_buffer(0, self.trim.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.trim_instances.slice(..));
        rpass.set_index_buffer(self.trim.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..self.trim.index_count, 0, 0..self.trim_count);

        // DRAW FILLER INSTANCES (if any)
        rpass.set_vertex_buffer(0, self.filler.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.filler_instances.slice(..));
        rpass.set_index_buffer(
            self.filler.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        rpass.draw_indexed(0..self.filler.index_count, 0, 0..self.filler_count);
    }

    // generate tile mesh
    fn generate_tile_mesh(device: &wgpu::Device) -> Mesh2d {
        let mut vertices = Vec::with_capacity(Self::PATCH_VERT_RESOLUTION.pow(2) as usize);
        for y in 0..Self::PATCH_VERT_RESOLUTION {
            for x in 0..Self::PATCH_VERT_RESOLUTION {
                vertices.push(Vec2::new(x as f32, y as f32));
            }
        }

        let mut indices: Vec<u32> = Vec::with_capacity(6 * Self::TILE_RESOLUTION.pow(2) as usize);
        let patch2d = |x, y| y * Self::PATCH_VERT_RESOLUTION + x;
        for y in 0..Self::TILE_RESOLUTION {
            for x in 0..Self::TILE_RESOLUTION {
                indices.push(patch2d(x, y));
                indices.push(patch2d(x + 1, y + 1));
                indices.push(patch2d(x, y + 1));

                indices.push(patch2d(x, y));
                indices.push(patch2d(x + 1, y));
                indices.push(patch2d(x + 1, y + 1));
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tile vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tile index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh2d {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as _,
        }
    }

    pub fn generate_filler_mesh(device: &wgpu::Device) -> Mesh2d {
        // --- Capacity Calculation ---
        let vert_capacity = (Self::PATCH_VERT_RESOLUTION * 8) as usize;
        let index_capacity = (Self::TILE_RESOLUTION * 24) as usize;

        let mut vertices: Vec<Vec2> = Vec::with_capacity(vert_capacity);

        // --- 1. Generate Vertices for the 4 Arms ---
        let offset = Self::TILE_RESOLUTION as f32;

        // Arm 1: +X direction
        for i in 0..Self::PATCH_VERT_RESOLUTION {
            let x = offset + i as f32 + 1.0;
            vertices.push(Vec2::new(x, 0.0));
            vertices.push(Vec2::new(x, 1.0));
        }

        // Arm 2: +Y direction
        for i in 0..Self::PATCH_VERT_RESOLUTION {
            let y = offset + i as f32 + 1.0;
            vertices.push(Vec2::new(1.0, y));
            vertices.push(Vec2::new(0.0, y));
        }

        // Arm 3: -X direction
        for i in 0..Self::PATCH_VERT_RESOLUTION {
            let x = -(offset + i as f32);
            vertices.push(Vec2::new(x, 1.0));
            vertices.push(Vec2::new(x, 0.0));
        }

        // Arm 4: -Y direction
        for i in 0..Self::PATCH_VERT_RESOLUTION {
            let y = -(offset + i as f32);
            vertices.push(Vec2::new(0.0, y));
            vertices.push(Vec2::new(1.0, y));
        }
        debug_assert_eq!(vertices.len(), vert_capacity);

        // --- 2. Generate Indices ---
        let mut indices: Vec<u32> = Vec::with_capacity(index_capacity);

        for i in 0..(Self::TILE_RESOLUTION * 4) {
            let arm = i / Self::TILE_RESOLUTION;
            let local_i_in_arm = i % Self::TILE_RESOLUTION;

            // Corrected calculation for the base index of the current quad's vertices.
            let arm_vertex_start = arm * (Self::PATCH_VERT_RESOLUTION * 2);
            let bl = arm_vertex_start + local_i_in_arm * 2;
            let br = bl + 1;
            let tl = bl + 2;
            let tr = bl + 3;

            // Use different triangulation (winding order) for horizontal and vertical arms.
            if arm % 2 == 0 {
                // Horizontal arms (0 and 2)
                indices.extend_from_slice(&[br, bl, tr, bl, tl, tr]);
            } else {
                // Vertical arms (1 and 4)
                indices.extend_from_slice(&[br, bl, tl, br, tl, tr]);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Filler vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Filler index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh2d {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as _,
        }
    }

    pub fn generate_trim_mesh(device: &wgpu::Device) -> Mesh2d {
        // Pre-calculate capacity to avoid reallocations
        let vert_capacity = (Self::CLIPMAP_VERT_RESOLUTION * 4 + 2) as usize;
        let index_capacity = ((Self::CLIPMAP_VERT_RESOLUTION * 2) - 1) as usize * 6;

        let mut vertices: Vec<Vec2> = Vec::with_capacity(vert_capacity);

        // --- 1. Generate Vertices ---

        // Generate the vertical part of the "L" shape
        for i in 0..=Self::CLIPMAP_VERT_RESOLUTION {
            let y = (Self::CLIPMAP_VERT_RESOLUTION - i) as f32;
            vertices.push(Vec2::new(0.0, y));
            vertices.push(Vec2::new(1.0, y));
        }

        let start_of_horizontal = vertices.len();

        // Generate the horizontal part of the "L" shape
        for i in 0..Self::CLIPMAP_VERT_RESOLUTION {
            let x = (i + 1) as f32;
            vertices.push(Vec2::new(x, 0.0));
            vertices.push(Vec2::new(x, 1.0));
        }

        // --- 2. Center the Mesh ---
        let offset = Vec2::splat(0.5 * (Self::CLIPMAP_VERT_RESOLUTION as f32 + 1.0));
        for v in &mut vertices {
            *v -= offset;
        }

        // --- 3. Generate Indices ---
        let mut indices: Vec<u32> = Vec::with_capacity(index_capacity);

        // Generate indices for the vertical strip
        for i in 0..Self::CLIPMAP_VERT_RESOLUTION {
            let base = i * 2;
            // Two triangles forming a quad, with clockwise (CW) winding
            indices.extend_from_slice(&[
                base + 1,
                base + 0,
                base + 2, // Triangle 1
                base + 3,
                base + 1,
                base + 2, // Triangle 2
            ]);
        }

        // Generate indices for the horizontal strip
        for i in 0..(Self::CLIPMAP_VERT_RESOLUTION - 1) {
            let base = start_of_horizontal as u32 + i * 2;
            // Two triangles forming a quad, with clockwise (CW) winding
            indices.extend_from_slice(&[
                base + 1,
                base + 0,
                base + 2, // Triangle 1
                base + 3,
                base + 1,
                base + 2, // Triangle 2
            ]);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trim vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Trim index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh2d {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as _,
        }
    }

    pub fn generate_cross_mesh(device: &wgpu::Device) -> Mesh2d {
        // --- Capacity Calculation ---
        let vert_capacity = (Self::PATCH_VERT_RESOLUTION * 8) as usize;
        let index_capacity = (Self::TILE_RESOLUTION * 24 + 6) as usize;

        let mut vertices: Vec<Vec2> = Vec::with_capacity(vert_capacity);
        let mut indices: Vec<u32> = Vec::with_capacity(index_capacity);

        // --- 1. Generate Vertices ---
        let tile_res_f32 = Self::TILE_RESOLUTION as f32;

        // Generate vertices for the horizontal strip
        for i in 0..(Self::PATCH_VERT_RESOLUTION * 2) {
            let x = i as f32 - tile_res_f32;
            vertices.push(Vec2::new(x, 0.0));
            vertices.push(Vec2::new(x, 1.0));
        }

        let start_of_vertical = vertices.len() as u32;

        // Generate vertices for the vertical strip
        for i in 0..(Self::PATCH_VERT_RESOLUTION * 2) {
            let y = i as f32 - tile_res_f32;
            vertices.push(Vec2::new(0.0, y));
            vertices.push(Vec2::new(1.0, y));
        }

        // --- 2. Generate Indices ---

        // Generate indices for the horizontal strip
        for i in 0..(Self::TILE_RESOLUTION * 2 + 1) {
            let bl = i * 2;
            let br = i * 2 + 1;
            let tl = i * 2 + 2;
            let tr = i * 2 + 3;
            indices.extend_from_slice(&[
                br, bl, tr, // Triangle 1
                bl, tl, tr, // Triangle 2
            ]);
        }

        // Generate indices for the vertical strip
        for i in 0..(Self::TILE_RESOLUTION * 2 + 1) {
            // Skip the center quad to create the "cross" shape and avoid overlap
            if i == Self::TILE_RESOLUTION {
                continue;
            }
            let bl = i * 2;
            let br = i * 2 + 1;
            let tl = i * 2 + 2;
            let tr = i * 2 + 3;
            indices.extend_from_slice(&[
                start_of_vertical + br,
                start_of_vertical + tr,
                start_of_vertical + bl, // Triangle 1
                start_of_vertical + bl,
                start_of_vertical + tr,
                start_of_vertical + tl, // Triangle 2
            ]);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cross vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cross index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh2d {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as _,
        }
    }

    pub fn generate_seam_mesh(device: &wgpu::Device) -> Mesh2d {
        let res = Self::CLIPMAP_VERT_RESOLUTION;
        let num_vertices = (res * 4) as usize;

        // --- 1. Generate Vertices ---
        // Pre-allocate the vector with default values to allow writing by index,
        // which directly matches the C++ implementation's style.
        let mut vertices: Vec<Vec2> = vec![Vec2::ZERO; num_vertices];
        let res_f32 = res as f32;

        for i in 0..res {
            let i_usize = i as usize;
            let i_f32 = i as f32;
            let res_usize = res as usize;

            // Bottom edge (from left to right)
            vertices[res_usize * 0 + i_usize] = Vec2::new(i_f32, 0.0);
            // Right edge (from bottom to top)
            vertices[res_usize * 1 + i_usize] = Vec2::new(res_f32, i_f32);
            // Top edge (from right to left)
            vertices[res_usize * 2 + i_usize] = Vec2::new(res_f32 - i_f32, res_f32);
            // Left edge (from top to bottom)
            vertices[res_usize * 3 + i_usize] = Vec2::new(0.0, res_f32 - i_f32);
        }

        // --- 2. Generate Indices ---
        let num_indices = (res * 6) as usize;
        let mut indices: Vec<u32> = Vec::with_capacity(num_indices);
        let num_vertices_u32 = num_vertices as u32;

        // Generate triangles for the entire strip, creating pairs like (v1,v0,v2), (v3,v2,v4), etc.
        for i in (0..num_vertices_u32).step_by(2) {
            indices.extend_from_slice(&[i + 1, i, i + 2]);
        }

        // The original C++ code fixed the last index to wrap around. We do the same here.
        // The last triangle should connect the last two vertices to the first one (v0).
        if let Some(last_index) = indices.last_mut() {
            *last_index = 0;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Seam vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Seam index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh2d {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as _,
        }
    }
}

pub struct Mesh2d {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl VertexAttribute for Vec2 {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x2, // position
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}
