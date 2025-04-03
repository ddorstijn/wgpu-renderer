use nanorand::Rng;
use wgpu::util::DeviceExt;

use crate::State;

pub struct ParticleCompute {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
    pub work_group_count: u32,
    pub particle_buffers: Vec<wgpu::Buffer>,
    pub particle_bind_groups: Vec<wgpu::BindGroup>,

    frame_num: usize,
}

impl ParticleCompute {
    pub fn new(
        state: &State,
        num_particles: u32,
        particles_per_group: u32,
        sim_param_data: &[f32],
    ) -> Self {
        let compute_shader = state
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/compute.wgsl"));

        let sim_param_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Simulation Parameter Buffer"),
                contents: bytemuck::cast_slice(&sim_param_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // create compute bind layout group and compute pipeline layout

        let compute_bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(
                                    (sim_param_data.len() * size_of::<f32>()) as _,
                                ),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new((num_particles * 16) as _),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new((num_particles * 16) as _),
                            },
                            count: None,
                        },
                    ],
                    label: None,
                });
        let compute_pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("compute"),
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            state
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_shader,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });

        // buffer for all particles data of type [(posx,posy,velx,vely),...]

        let mut initial_particle_data = vec![0.0f32; (4 * num_particles) as usize];
        let mut rng = nanorand::WyRand::new_seed(42); // Seeded RNG
        let mut unif = || rng.generate::<f32>() * 2f32 - 1f32; // Generate a num (-1, 1)
        for particle_instance_chunk in initial_particle_data.chunks_mut(4) {
            particle_instance_chunk[0] = unif(); // posx
            particle_instance_chunk[1] = unif(); // posy
            particle_instance_chunk[2] = unif() * 0.1; // velx
            particle_instance_chunk[3] = unif() * 0.1; // vely
        }

        // creates two buffers of particle data each of size num_particles
        // the two buffers alternate as dst and src for each frame

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            particle_buffers.push(state.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Particle Buffer {i}")),
                    contents: bytemuck::cast_slice(&initial_particle_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst

        for i in 0..2 {
            particle_bind_groups.push(state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: None,
            }));
        }

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count =
            ((num_particles as f32) / (particles_per_group as f32)).ceil() as u32;

        Self {
            compute_pipeline,
            bind_group: particle_bind_groups[0].clone(),
            work_group_count,
            particle_buffers,
            particle_bind_groups,
            frame_num: 0,
        }
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder) {
        encoder.push_debug_group("compute boid movement");
        {
            // compute pass
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);
        }
        encoder.pop_debug_group();

        self.frame_num ^= 1; // Toggle between 0 and 1
    }

    pub fn get_particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffers[self.frame_num]
    }
}
