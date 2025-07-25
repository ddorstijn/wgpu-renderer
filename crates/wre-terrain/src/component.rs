use crate::consts::Mesh2d;

pub struct TerrainComponent {
    pub(crate) instance_count: usize,
    pub(crate) instance_bg: wgpu::BindGroup,
    pub(crate) instance_bf: wgpu::Buffer,
    pub(crate) vertex_bf: wgpu::Buffer,
    pub(crate) index_bf: wgpu::Buffer,
    pub(crate) index_count: usize,
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
