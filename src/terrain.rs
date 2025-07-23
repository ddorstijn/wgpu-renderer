use crate::{model::VertexAttribute, texture, util::create_render_pipeline};
use const_for::const_for;
use glam::{Mat4, Quat, Vec2, Vec3Swizzles, Vec4, quat};
use std::path::Path;
use wgpu::util::DeviceExt;

const SCALE_OFFSET: usize = 5;
const N_LEVELS: usize = 10;
const N_TILES: usize = N_LEVELS * 16; // 4x4 tiles per level
const N_FILLERS: usize = N_LEVELS; // 1 filler per level
const N_TRIMS: usize = N_LEVELS - 1; // no trim for finest
const N_SEAMS: usize = N_LEVELS - 1; // no seam for finest
const N_CROSS: usize = 1; // 1 cross at camera position

const ROTATIONS: [Quat; 4] = [
    Quat::IDENTITY,
    quat(0.0, 0.0, 0.70710677, -0.70710677), // 270 degrees
    quat(0.0, 0.0, 0.70710677, 0.70710677),  // 90 degrees
    quat(0.0, 0.0, 1.0, 0.0),                // 180 degrees
];

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

    tile_vertex: wgpu::Buffer,
    tile_index: wgpu::Buffer,

    seam_vertex: wgpu::Buffer,
    seam_index: wgpu::Buffer,

    trim_vertex: wgpu::Buffer,
    trim_index: wgpu::Buffer,

    cross_vertex: wgpu::Buffer,
    cross_index: wgpu::Buffer,

    filler_vertex: wgpu::Buffer,
    filler_index: wgpu::Buffer,
}

impl TerrainSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        render_format: wgpu::TextureFormat,
        heightmap_path: &Path,
    ) -> anyhow::Result<Self> {
        let heightmap =
            texture::Texture::from_heightmap("Heightmap", device, queue, heightmap_path)?;

        // --- Bind Group Layouts ---
        let heightmap_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Terrain Shared BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
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
            &[Mesh2d::desc(), InstanceData::desc()],
            wgpu::include_wgsl!("terrain.wgsl"),
        );

        // --- Pre‐allocate instance buffers for maximum possible instances ---
        let alloc = |count| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: std::mem::size_of::<InstanceData>() as u64 * count as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        Ok(Self {
            heightmap,
            render_pipeline,
            shared_bind_group,

            tile_instances: alloc(N_TILES), // 4x4 tiles per level
            filler_instances: alloc(N_FILLERS),
            trim_instances: alloc(N_TRIMS), // no trim for finest level
            seam_instances: alloc(N_SEAMS), // no seam for finest level
            cross_instances: alloc(N_CROSS), // 1 cross at camera position

            tile_vertex: TILE_MESH.vertex_buffer(device),
            tile_index: TILE_MESH.index_buffer(device),
            seam_vertex: SEAM_MESH.vertex_buffer(device),
            seam_index: SEAM_MESH.index_buffer(device),
            trim_vertex: TRIM_MESH.vertex_buffer(device),
            trim_index: TRIM_MESH.index_buffer(device),
            cross_vertex: CROSS_MESH.vertex_buffer(device),
            cross_index: CROSS_MESH.index_buffer(device),
            filler_vertex: FILLER_MESH.vertex_buffer(device),
            filler_index: FILLER_MESH.index_buffer(device),
        })
    }

    pub fn update_terrain_system(&mut self, queue: &wgpu::Queue, camera_position: Vec2) {
        // We'll accumulate instance data here
        let mut tile_data = Vec::new();
        let mut filler_data = Vec::new();
        let mut trim_data = Vec::new();
        let mut cross_data = Vec::new();
        let mut seam_data = Vec::new();

        // The main 4×4 tile ring & filler/trim/seam per level
        for i in 0..N_LEVELS {
            let scale = (1u32 << i + SCALE_OFFSET) as f32;
            let tile_size = Vec2::splat((TILE_RES << i + SCALE_OFFSET) as f32);

            let v_scale = Vec2::splat(scale).extend(1.0);
            // snapped camera for this LOD
            let snapped_pos = (camera_position / scale).floor() * scale;
            // bottom‐left corner of 4×4 grid
            let base = snapped_pos - tile_size * 2.0;

            // --- Cross ---
            if i == 0 {
                let xf = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    snapped_pos.extend(0.0),
                );
                cross_data.push(InstanceData {
                    transform: xf,
                    color: Vec4::new(0.0, 0.0, 0.0, 1.0),
                });
            }

            // --- 4×4 Tiles (skip middle 2×2 if not finest) ---
            for x in 0..4 {
                for y in 0..4 {
                    if i != 0 && (matches!(x, 1 | 2)) && (matches!(y, 1 | 2)) {
                        continue;
                    }

                    let pos = Vec2::new(x as f32, y as f32);
                    let fill = Vec2::new(
                        if x >= 2 { 1.0 } else { 0.0 },
                        if y >= 2 { 1.0 } else { 0.0 },
                    ) * scale;

                    let bl = base + pos * tile_size + fill;
                    let transform = Mat4::from_scale_rotation_translation(
                        v_scale,
                        Quat::IDENTITY,
                        bl.extend(0.0),
                    );
                    tile_data.push(InstanceData {
                        transform,
                        color: Vec4::new((x % 2) as f32, (y % 2) as f32, 0.0, 1.0),
                    });
                }
            }

            // --- Filler ring ---
            {
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    snapped_pos.extend(0.0),
                );
                filler_data.push(InstanceData {
                    transform,
                    color: Vec4::new(0.0, 0.0, 1.0, 1.0),
                });
            }

            // Trim and seam are not generated for the finest level
            if i < N_LEVELS - 1 {
                let next_scale = scale * 2.0;
                let next_snap = (camera_position / next_scale).floor() * next_scale;

                // --- Seam ---
                let next_base =
                    next_snap - Vec2::splat((TILE_RES << (i + SCALE_OFFSET + 1)) as f32);
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    next_base.extend(0.0),
                );

                seam_data.push(InstanceData {
                    transform,
                    color: Vec4::new(1.0, 0.0, 0.0, 1.0),
                });

                // --- Trim ---
                let d = camera_position - next_snap;
                let r = (if d.x < scale { 2 } else { 0 }) | (if d.y < scale { 1 } else { 0 });

                let center = snapped_pos + 0.5 * v_scale.xy();
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    ROTATIONS[r],
                    center.extend(0.0),
                );

                trim_data.push(InstanceData {
                    transform,
                    color: Vec4::new(0.0, 1.0, 1.0, 1.0),
                });
            }
        }

        // 3) Upload each to its GPU buffer
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
        rpass.set_vertex_buffer(0, self.tile_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.tile_instances.slice(..));
        rpass.set_index_buffer(self.tile_index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..I_TILE as _, 0, 0..N_TILES as u32);

        // DRAW CROSS INSTANCES
        rpass.set_vertex_buffer(0, self.cross_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.cross_instances.slice(..));
        rpass.set_index_buffer(self.cross_index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..I_CROSS as _, 0, 0..N_CROSS as u32);

        // DRAW SEAM INSTANCES
        rpass.set_vertex_buffer(0, self.seam_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.seam_instances.slice(..));
        rpass.set_index_buffer(self.seam_index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..I_SEAM as _, 0, 0..N_SEAMS as u32);

        // DRAW TRIM INSTANCES
        rpass.set_vertex_buffer(0, self.trim_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.trim_instances.slice(..));
        rpass.set_index_buffer(self.trim_index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..I_TRIM as _, 0, 0..N_TRIMS as u32);

        // DRAW FILLER INSTANCES
        rpass.set_vertex_buffer(0, self.filler_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.filler_instances.slice(..));
        rpass.set_index_buffer(self.filler_index.slice(..), wgpu::IndexFormat::Uint32);
        rpass.draw_indexed(0..I_FILL as _, 0, 0..N_FILLERS as u32);
    }
}

#[derive(Debug)]
pub struct Mesh2d<const V: usize, const I: usize> {
    pub label: &'static str,
    pub vertices: [Vec2; V],
    pub indices: [u32; I],
}

impl<const V: usize, const I: usize> Mesh2d<V, I> {
    pub fn vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some((self.label.to_owned() + "Vertex Buffer").as_str()),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    pub fn index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some((self.label.to_owned() + "Index Buffer").as_str()),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}

impl VertexAttribute for Mesh2d<V_TILE, I_TILE> {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x2,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vec2>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

const TILE_RES: usize = 64;
const PATCH_RES: usize = TILE_RES + 1;
const CLIP_RES: usize = TILE_RES * 4 + 1;
const CLIP_VERT_RES: usize = CLIP_RES + 1;

const V_TILE: usize = PATCH_RES * PATCH_RES;
const I_TILE: usize = 6 * (TILE_RES * TILE_RES);

const V_FILL: usize = 8 * PATCH_RES;
const I_FILL: usize = 24 * TILE_RES;

const V_TRIM: usize = (CLIP_VERT_RES * 2 + 1) * 2;
const I_TRIM: usize = (CLIP_VERT_RES * 2 - 1) * 6;

const V_CROSS: usize = 8 * PATCH_RES;
const I_CROSS: usize = 24 * TILE_RES + 6;

const V_SEAM: usize = 4 * CLIP_VERT_RES;
const I_SEAM: usize = 6 * CLIP_VERT_RES;

const fn generate_tile_mesh() -> Mesh2d<V_TILE, I_TILE> {
    let mut vertices = [Vec2::ZERO; V_TILE];
    const_for!(v in 0..V_TILE => {
        let x = (v % PATCH_RES) as f32;
        let y = (v / PATCH_RES) as f32;
        vertices[v] = Vec2::new(x, y);
    });

    let mut indices = [0u32; I_TILE];
    let mut idx = 0;
    const_for!(y in 0..TILE_RES => {
        const_for!(x in 0..TILE_RES => {
            let base = (y * PATCH_RES + x) as u32;
            let right = base + 1;
            let up = base + PATCH_RES as u32;
            let up_right = up + 1;

            // [base, up_right, up] and [base, right, up_right]
            let quad_indices = [base, up_right, up, base, right, up_right];
            indices[idx..idx + 6].copy_from_slice(&quad_indices);
            idx += 6;
        });
    });

    Mesh2d {
        label: "Tile",
        vertices,
        indices,
    }
}

const fn generate_filler_mesh() -> Mesh2d<V_FILL, I_FILL> {
    let mut verts = [Vec2::ZERO; V_FILL];
    let mut v = 0;
    let offset = TILE_RES as f32;

    // A single loop generates vertices for all 4 arms
    const_for!(arm in 0..4 => {
        const_for!(i in 0..PATCH_RES => {
            let i_f = i as f32;
            verts[v..v + 2].copy_from_slice(&match arm {
                0 => {
                    // +X arm
                    let x = offset + i_f + 1.0;
                    [Vec2::new(x, 0.0), Vec2::new(x, 1.0)]
                }
                1 => {
                    // +Y arm
                    let y = offset + i_f + 1.0;
                    [Vec2::new(1.0, y), Vec2::new(0.0, y)]
                }
                2 => {
                    // -X arm
                    let x = -(offset + i_f);
                    [Vec2::new(x, 1.0), Vec2::new(x, 0.0)]
                }
                _ => {
                    // -Y arm
                    let y = -(offset + i_f);
                    [Vec2::new(0.0, y), Vec2::new(1.0, y)]
                }
            });
            v += 2;
        });
    });

    let mut idxs = [0u32; I_FILL];
    let mut idx = 0;
    const_for!(a in 0..(TILE_RES * 4) => {
        let arm = a / TILE_RES;
        let local = a % TILE_RES;
        let arm_start = (arm * PATCH_RES * 2) as u32;

        let bl = arm_start + (local * 2) as u32;
        let br = bl + 1;
        let tl = bl + 2;
        let tr = tl + 1;

        // Apply correct winding order based on arm orientation
        let quad_indices = if arm % 2 == 0 {
            // Horizontal arms: [br,bl,tr], [bl,tl,tr]
            [br, bl, tr, bl, tl, tr]
        } else {
            // Vertical arms: [br,bl,tl], [br,tl,tr] - FIXED
            [br, bl, tl, br, tl, tr]
        };
        idxs[idx..idx + 6].copy_from_slice(&quad_indices);
        idx += 6;
    });

    Mesh2d {
        label: "Filler",
        vertices: verts,
        indices: idxs,
    }
}

// -------------------------------------------------------------------
// 4) generate_trim_mesh
// -------------------------------------------------------------------

const fn generate_trim_mesh() -> Mesh2d<V_TRIM, I_TRIM> {
    let mut vertices = [Vec2::ZERO; V_TRIM];
    let mut indices = [0u32; I_TRIM];

    // precompute half the total extent (to center at origin)
    let extent_f = CLIP_VERT_RES as f32 + 1.0;
    let half = 0.5 * extent_f;

    // 1) build the L shape vertices (vertical then horizontal), already offset
    let mut v = 0;
    // vertical bar: from y = +R down to y = −(half−1)
    const_for!(i in 0..CLIP_VERT_RES + 1 => {
        let y = CLIP_VERT_RES as f32 - i as f32 - half;
        vertices[v] = Vec2::new(0.0 - half, y);
        vertices[v + 1] = Vec2::new(1.0 - half, y);
        v += 2;
    });
    // mark where the horizontal strip starts
    let start_h = v as u32;

    // horizontal bar: from x = 1−half up to x = R−half
    const_for!(i in 0..CLIP_VERT_RES => {
        let x = (i as f32 + 1.0) - half;
        vertices[v] = Vec2::new(x, 0.0 - half);
        vertices[v + 1] = Vec2::new(x, 1.0 - half);
        v += 2;
    });

    // 2) build indices for the two strips of quads (6 idx per segment)

    // vertical strip
    let mut idx = 0;
    const_for!(i in 0..CLIP_VERT_RES => {
        let base = (i as u32) * 2;
        // [br, bl, next_bl], [next_br, br, next_bl]
        indices[idx + 0] = base + 1;
        indices[idx + 1] = base + 0;
        indices[idx + 2] = base + 2;
        indices[idx + 3] = base + 3;
        indices[idx + 4] = base + 1;
        indices[idx + 5] = base + 2;
        idx += 6;
    });

    // horizontal strip
    const_for!(i in 0..(CLIP_VERT_RES - 1) => {
        let base = start_h + (i as u32) * 2;
        indices[idx + 0] = base + 1;
        indices[idx + 1] = base + 0;
        indices[idx + 2] = base + 2;
        indices[idx + 3] = base + 3;
        indices[idx + 4] = base + 1;
        indices[idx + 5] = base + 2;
        idx += 6;
    });

    Mesh2d {
        label: "Trim",
        vertices,
        indices,
    }
}

const fn generate_cross_mesh() -> Mesh2d<V_CROSS, I_CROSS> {
    let mut verts = [Vec2::ZERO; V_CROSS];
    let mut idxs = [0u32; I_CROSS];

    // 1) horizontal bar vertices
    let tile_f = TILE_RES as f32;
    let mut v = 0;
    const_for!(i in 0..(PATCH_RES * 2) => {
        let x = i as f32 - tile_f;
        verts[v] = Vec2::new(x, 0.0);
        verts[v + 1] = Vec2::new(x, 1.0);
        v += 2;
    });

    // remember where the vertical bar starts
    let vert_base = v as u32;

    // 2) vertical bar vertices
    const_for!(i in 0..(PATCH_RES * 2) => {
        let y = i as f32 - tile_f;
        verts[v] = Vec2::new(0.0, y);
        verts[v + 1] = Vec2::new(1.0, y);
        v += 2;
    });

    // 3) horizontal‐strip indices
    let mut idx = 0;
    const_for!(i in 0..TILE_RES * 2 + 1 => {
        let bl = (i * 2) as u32;
        let br = bl + 1;
        let tl = bl + 2;
        let tr = br + 2;
        idxs[idx + 0] = br;
        idxs[idx + 1] = bl;
        idxs[idx + 2] = tr;
        idxs[idx + 3] = bl;
        idxs[idx + 4] = tl;
        idxs[idx + 5] = tr;
        idx += 6;
    });

    // 4) vertical‐strip indices (skip the center line at i == TILE_RES)
    const_for!(i in 0..TILE_RES * 2 + 1 => {
        if i != TILE_RES {
            let bl = (i * 2) as u32;
            let br = bl + 1;
            let tl = bl + 2;
            let tr = br + 2;
            idxs[idx + 0] = vert_base + br;
            idxs[idx + 1] = vert_base + tr;
            idxs[idx + 2] = vert_base + bl;
            idxs[idx + 3] = vert_base + bl;
            idxs[idx + 4] = vert_base + tr;
            idxs[idx + 5] = vert_base + tl;
            idx += 6;
        }
    });

    Mesh2d {
        label: "Cross",
        vertices: verts,
        indices: idxs,
    }
}

const fn generate_seam_mesh() -> Mesh2d<V_SEAM, I_SEAM> {
    let mut vertices = [Vec2::ZERO; V_SEAM];
    let mut indices = [0u32; I_SEAM];

    // 1) ring of CLIP_VERT_RES verts on each side
    let res_f = CLIP_VERT_RES as f32;
    const_for!(i in 0..CLIP_VERT_RES => {
        let f = i as f32;
        vertices[i] = Vec2::new(f, 0.0);
        vertices[CLIP_VERT_RES + i] = Vec2::new(res_f, f);
        vertices[CLIP_VERT_RES * 2 + i] = Vec2::new(res_f - f, res_f);
        vertices[CLIP_VERT_RES * 3 + i] = Vec2::new(0.0, res_f - f);
    });

    // 2) triangle‐strip indices, wrapping via `% V_SEAM`
    let mut idx = 0;
    let vcount = V_SEAM as u32;
    const_for!(pair in 0..(V_SEAM / 2) => {
        let j = (pair * 2) as u32;
        indices[idx + 0] = j + 1;
        indices[idx + 1] = j;
        indices[idx + 2] = (j + 2) % vcount;
        idx += 3;
    });

    Mesh2d {
        label: "Seam",
        vertices,
        indices,
    }
}

const TILE_MESH: Mesh2d<V_TILE, I_TILE> = generate_tile_mesh();
const FILLER_MESH: Mesh2d<V_FILL, I_FILL> = generate_filler_mesh();
const TRIM_MESH: Mesh2d<V_TRIM, I_TRIM> = generate_trim_mesh();
const CROSS_MESH: Mesh2d<V_CROSS, I_CROSS> = generate_cross_mesh();
const SEAM_MESH: Mesh2d<V_SEAM, I_SEAM> = generate_seam_mesh();
