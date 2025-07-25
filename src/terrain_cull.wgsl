struct CameraUniforms {
    view_projection: mat4x4<f32>,
};

struct DrawIndexedIndirectCommand {
    vertex_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
};

struct GpuObjectData {
    model_matrix: mat4x4<f32>,
    bounding_aabb_min: vec3<f32>,
    bounding_aabb_max: vec3<f32>,
    padding: vec2<u32>,
};

@group(0) @binding(0)
var<storage, read> object_data_buffer: array<GpuObjectData>;

@group(0) @binding(1)
var<storage, read_write> indirect_draw_commands: array<DrawIndexedIndirectCommand>;

@group(0) @binding(2)
var<storage, read_write> visible_count: atomic<u32>;

@group(1) @binding(0)
var<uniform> camera: CameraUniforms;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    visible_count = 1;
}