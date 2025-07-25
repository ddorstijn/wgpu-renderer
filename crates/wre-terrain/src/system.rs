use std::path::Path;

use crate::component::TerrainComponent;

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
