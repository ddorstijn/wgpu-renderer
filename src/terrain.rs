use crate::{camera::Camera, model::VertexAttribute, texture, util::create_render_pipeline};
use const_for::const_for;
use glam::{Mat4, Quat, Vec2, Vec3Swizzles, quat};
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
}

struct TerrainComponent {
    instance_count: usize,
    instance_bg: wgpu::BindGroup,
    instance_bf: wgpu::Buffer,
    vertex_bf: wgpu::Buffer,
    index_bf: wgpu::Buffer,
    index_count: usize,
}

impl TerrainComponent {
    pub fn new(device: &wgpu::Device, instance_bgl: &wgpu::BindGroupLayout, mesh: Mesh2d) -> Self {
        let instance_bf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: std::mem::size_of::<InstanceData>() as u64 * mesh.instance_count as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TerrainComponent BG"),
            layout: &instance_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_bf.as_entire_binding(),
            }],
        });

        Self {
            instance_bf,
            instance_bg,
            vertex_bf: mesh.vertex_buffer(device),
            index_bf: mesh.index_buffer(device),
            index_count: mesh.index_count,
            instance_count: mesh.instance_count,
        }
    }
}

trait DrawTerrainComponent<'a> {
    #[allow(unused)]
    fn draw_terrain(&mut self, component: &'a TerrainComponent);
}

impl<'a, 'b> DrawTerrainComponent<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_terrain(&mut self, component: &'a TerrainComponent) {
        self.set_bind_group(2, &component.instance_bg, &[]);
        self.set_vertex_buffer(0, component.vertex_bf.slice(..));
        self.set_index_buffer(component.index_bf.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(
            0..component.index_count as u32,
            0,
            0..component.instance_count as u32,
        );
    }
}

// In your main render state or engine structure
pub struct TerrainSystem {
    #[allow(unused)]
    heightmap_bf: texture::Texture, // Later used for editing
    heightmap_bg: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,

    tile: TerrainComponent,
    cross: TerrainComponent,
    fill: TerrainComponent,
    trim: TerrainComponent,
    seam: TerrainComponent,
}

impl TerrainSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bgl: &wgpu::BindGroupLayout,
        render_format: wgpu::TextureFormat,
        heightmap_path: &Path,
    ) -> anyhow::Result<Self> {
        let heightmap_bf =
            texture::Texture::from_heightmap("Heightmap", device, queue, heightmap_path)?;

        // --- Bind Group Layouts ---
        let heightmap_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain Heightmap BGL"),
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

        let instance_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("TerrainComponent BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // --- Bind Groups ---
        let heightmap_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Heightmap BG"),
            layout: &heightmap_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&heightmap_bf.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&heightmap_bf.sampler),
                },
            ],
        });

        // --- Render Pipeline ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Pipeline Layout"),
                bind_group_layouts: &[camera_bgl, &heightmap_bgl, &instance_bgl],
                push_constant_ranges: &[],
            });

        let render_pipeline = create_render_pipeline(
            device,
            &render_pipeline_layout,
            render_format,
            &[Mesh2d::desc()],
            wgpu::include_wgsl!("terrain.wgsl"),
        );

        let tile = TerrainComponent::new(device, &instance_bgl, TILE_MESH);
        let cross = TerrainComponent::new(device, &instance_bgl, CROSS_MESH);
        let fill = TerrainComponent::new(device, &instance_bgl, FILLER_MESH);
        let trim = TerrainComponent::new(device, &instance_bgl, TRIM_MESH);
        let seam = TerrainComponent::new(device, &instance_bgl, SEAM_MESH);

        Ok(Self {
            heightmap_bf,
            heightmap_bg,

            render_pipeline,

            tile,
            cross,
            fill,
            trim,
            seam,
        })
    }

    pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        // We'll accumulate instance data here
        let mut tile_data = Vec::new();
        let mut filler_data = Vec::new();
        let mut trim_data = Vec::new();
        let mut cross_data = Vec::new();
        let mut seam_data = Vec::new();

        let camera_position = camera.eye.xy();

        // The main 4×4 tile ring & filler/trim/seam per level
        for level in 0..N_LEVELS {
            let scale = (1u32 << level + SCALE_OFFSET) as f32;
            let tile_size = Vec2::splat((TILE_RES << level + SCALE_OFFSET) as f32);

            let v_scale = Vec2::splat(scale).extend(1.0);
            // snapped camera for this LOD
            let snapped_pos = (camera_position / scale).floor() * scale;
            // bottom‐left corner of 4×4 grid
            let base = snapped_pos - tile_size * 2.0;

            // --- Cross ---
            if level == 0 {
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    snapped_pos.extend(0.0),
                );
                cross_data.push(InstanceData { transform });
            }

            // --- 4×4 Tiles (skip middle 2×2 if not finest) ---
            for x in 0..4 {
                for y in 0..4 {
                    if level != 0 && (matches!(x, 1 | 2)) && (matches!(y, 1 | 2)) {
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
                    tile_data.push(InstanceData { transform });
                }
            }

            // --- Filler ring ---
            {
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    snapped_pos.extend(0.0),
                );
                filler_data.push(InstanceData { transform });
            }

            // Trim and seam are not generated for the finest level
            if level < N_LEVELS - 1 {
                let next_scale = scale * 2.0;
                let next_snap = (camera_position / next_scale).floor() * next_scale;

                // --- Seam ---
                let next_base =
                    next_snap - Vec2::splat((TILE_RES << (level + SCALE_OFFSET + 1)) as f32);
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    Quat::IDENTITY,
                    next_base.extend(0.0),
                );

                seam_data.push(InstanceData { transform });

                // --- Trim ---
                let d = camera_position - next_snap;
                let r = (if d.x < scale { 2 } else { 0 }) | (if d.y < scale { 1 } else { 0 });

                let center = snapped_pos + 0.5 * v_scale.xy();
                let transform = Mat4::from_scale_rotation_translation(
                    v_scale,
                    ROTATIONS[r],
                    center.extend(0.0),
                );

                trim_data.push(InstanceData { transform });
            }
        }

        // 3) Upload each to its GPU buffer
        queue.write_buffer(&self.tile.instance_bf, 0, bytemuck::cast_slice(&tile_data));
        queue.write_buffer(
            &self.cross.instance_bf,
            0,
            bytemuck::cast_slice(&cross_data),
        );
        queue.write_buffer(&self.seam.instance_bf, 0, bytemuck::cast_slice(&seam_data));
        queue.write_buffer(&self.trim.instance_bf, 0, bytemuck::cast_slice(&trim_data));
        queue.write_buffer(
            &self.fill.instance_bf,
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
        rpass.set_bind_group(1, &self.heightmap_bg, &[]);

        rpass.draw_terrain(&self.tile);
        rpass.draw_terrain(&self.cross);
        rpass.draw_terrain(&self.fill);
        rpass.draw_terrain(&self.trim);
        rpass.draw_terrain(&self.seam);
    }
}

// Const generated meshes

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

// TILE always has the most vertices and indices. Means we are wasting some space for the others, but makes implementation much easier on our side.
const V_MAX: usize = V_TILE;
const I_MAX: usize = I_TILE;

#[derive(Debug)]
pub struct Mesh2d {
    pub label: &'static str,
    pub vertices: [Vec2; V_MAX],
    pub indices: [u32; I_MAX],
    pub vertex_count: usize,
    pub index_count: usize,
    pub instance_count: usize,
}

impl Mesh2d {
    pub fn vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some((self.label.to_owned() + "Vertex Buffer").as_str()),
            contents: bytemuck::cast_slice(&self.vertices[0..self.vertex_count]),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    pub fn index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some((self.label.to_owned() + "Index Buffer").as_str()),
            contents: bytemuck::cast_slice(&self.indices[0..self.index_count]),
            usage: wgpu::BufferUsages::INDEX,
        })
    }
}

impl VertexAttribute for Mesh2d {
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

const fn generate_tile_mesh() -> Mesh2d {
    let mut vertices = [Vec2::ZERO; V_MAX];
    const_for!(v in 0..V_TILE => {
        let x = (v % PATCH_RES) as f32;
        let y = (v / PATCH_RES) as f32;
        vertices[v] = Vec2::new(x, y);
    });

    let mut indices = [0u32; I_MAX];
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
        vertex_count: V_TILE,
        index_count: I_TILE,
        instance_count: N_TILES,
    }
}

const fn generate_filler_mesh() -> Mesh2d {
    let mut vertices = [Vec2::ZERO; V_MAX];
    let mut v = 0;
    let offset = TILE_RES as f32;

    // A single loop generates vertices for all 4 arms
    const_for!(arm in 0..4 => {
        const_for!(i in 0..PATCH_RES => {
            let i_f = i as f32;
            vertices[v..v + 2].copy_from_slice(&match arm {
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

    let mut indices = [0u32; I_MAX];
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
        indices[idx..idx + 6].copy_from_slice(&quad_indices);
        idx += 6;
    });

    Mesh2d {
        label: "Filler",
        vertices,
        indices,
        vertex_count: V_FILL,
        index_count: I_FILL,
        instance_count: N_FILLERS,
    }
}

const fn generate_trim_mesh() -> Mesh2d {
    let mut vertices = [Vec2::ZERO; V_MAX];
    let mut indices = [0u32; I_MAX];

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
        vertex_count: V_TRIM,
        index_count: I_TRIM,
        instance_count: N_TRIMS,
    }
}

const fn generate_cross_mesh() -> Mesh2d {
    let mut vertices = [Vec2::ZERO; V_MAX];
    let mut indices = [0u32; I_MAX];

    // 1) horizontal bar vertices
    let tile_f = TILE_RES as f32;
    let mut v = 0;
    const_for!(i in 0..(PATCH_RES * 2) => {
        let x = i as f32 - tile_f;
        vertices[v] = Vec2::new(x, 0.0);
        vertices[v + 1] = Vec2::new(x, 1.0);
        v += 2;
    });

    // remember where the vertical bar starts
    let vert_base = v as u32;

    // 2) vertical bar vertices
    const_for!(i in 0..(PATCH_RES * 2) => {
        let y = i as f32 - tile_f;
        vertices[v] = Vec2::new(0.0, y);
        vertices[v + 1] = Vec2::new(1.0, y);
        v += 2;
    });

    // 3) horizontal‐strip indices
    let mut idx = 0;
    const_for!(i in 0..TILE_RES * 2 + 1 => {
        let bl = (i * 2) as u32;
        let br = bl + 1;
        let tl = bl + 2;
        let tr = br + 2;
        indices[idx + 0] = br;
        indices[idx + 1] = bl;
        indices[idx + 2] = tr;
        indices[idx + 3] = bl;
        indices[idx + 4] = tl;
        indices[idx + 5] = tr;
        idx += 6;
    });

    // 4) vertical‐strip indices (skip the center line at i == TILE_RES)
    const_for!(i in 0..TILE_RES * 2 + 1 => {
        if i != TILE_RES {
            let bl = (i * 2) as u32;
            let br = bl + 1;
            let tl = bl + 2;
            let tr = br + 2;
            indices[idx + 0] = vert_base + br;
            indices[idx + 1] = vert_base + tr;
            indices[idx + 2] = vert_base + bl;
            indices[idx + 3] = vert_base + bl;
            indices[idx + 4] = vert_base + tr;
            indices[idx + 5] = vert_base + tl;
            idx += 6;
        }
    });

    Mesh2d {
        label: "Cross",
        vertices,
        indices,
        vertex_count: V_CROSS,
        index_count: I_CROSS,
        instance_count: N_CROSS,
    }
}

const fn generate_seam_mesh() -> Mesh2d {
    let mut vertices = [Vec2::ZERO; V_MAX];
    let mut indices = [0u32; I_MAX];

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
        vertex_count: V_SEAM,
        index_count: I_SEAM,
        instance_count: N_SEAMS,
    }
}

const TILE_MESH: Mesh2d = generate_tile_mesh();
const FILLER_MESH: Mesh2d = generate_filler_mesh();
const TRIM_MESH: Mesh2d = generate_trim_mesh();
const CROSS_MESH: Mesh2d = generate_cross_mesh();
const SEAM_MESH: Mesh2d = generate_seam_mesh();
