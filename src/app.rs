use bevy_math::Vec3;
use wgpu::util::DeviceExt;

use crate::{
    camera::Camera, model::texture::Texture, particle_compute::ParticleCompute, util::load_obj,
};

pub struct App {
    num_particles: u32,
    frame_num: usize,
    last_update_time: std::time::Instant, // Track the last compute update time
    interpolation_factor: f32,

    camera: Camera,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    depth_texture: Texture,

    compute: ParticleCompute,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl App {
    pub fn new(state: &crate::State) -> Self {
        let num_particles = 1;
        let particles_per_group = 64;

        let camera = Camera {
            eye: (-2.0, 0.0, 1.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: Vec3::Z,
            aspect: state.size.width as f32 / state.size.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let draw_shader = state
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/draw.wgsl"));

        let sim_param_data = [
            0.04f32, // deltaT
            0.1,     // rule1Distance
            0.025,   // rule2Distance
            0.025,   // rule3Distance
            0.02,    // rule1Scale
            0.05,    // rule2Scale
            0.005,   // rule3Scale
        ]
        .to_vec();

        let compute =
            ParticleCompute::new(state, num_particles, particles_per_group, &sim_param_data);

        let camera_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(
                    &camera.build_view_projection_matrix().to_cols_array_2d(),
                ),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    label: Some("camera_bind_group_layout"),
                });

        let camera_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let depth_texture =
            Texture::create_depth_texture(&state.device, &state.size, "depth_texture");

        let render_pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline = state.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &draw_shader,
                    entry_point: Some("main_vs"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: 4 * 4,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: 3 * 4,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![2 => Float32x3],
                        },
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &draw_shader,
                    entry_point: Some("main_fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(state.surface_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        );

        // buffer for the three 2d triangle vertices of each instance
        let (vertices, indices) = load_obj("./src/meshes/Car.obj");
        let vertex_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            num_particles,
            last_update_time: std::time::Instant::now(),
            interpolation_factor: 0.0,

            camera,
            camera_bind_group,
            camera_buffer,
            depth_texture,

            compute,

            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,

            render_pipeline,
            frame_num: 0,
        }
    }

    pub fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
        // create render pass descriptor and its color attachments
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                // Not clearing here in order to test wgpu's zero texture initialization on a surface texture.
                // Users should avoid loading uninitialized memory since this can cause additional overhead.
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        // get command encoder
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        command_encoder.push_debug_group("render boids");
        {
            // render pass
            let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.render_pipeline);
            // render dst particles
            rpass.set_vertex_buffer(0, self.compute.get_particle_buffer().slice(..));
            // set the camera bind group
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            // the three instance-local vertices
            rpass.set_vertex_buffer(1, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..self.index_count, 0, 0..self.num_particles);
        }
        command_encoder.pop_debug_group();

        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_update_time).as_secs_f32();
        let fixed_update_interval = 1.0 / 60.0;

        if delta_time >= fixed_update_interval {
            self.last_update_time = now;

            self.compute.render(&mut command_encoder);

            // update frame count
            self.frame_num += 1;
        }

        // Calculate interpolation factor (t)
        self.interpolation_factor = (delta_time % fixed_update_interval) / fixed_update_interval;

        // done
        queue.submit(Some(command_encoder.finish()));
    }

    pub fn resize(&mut self, state: &crate::State) {
        self.depth_texture =
            Texture::create_depth_texture(&state.device, &state.size, "depth_texture");
    }
}
