struct Camera {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Instance {
    transform: mat4x4<f32>,
}
@group(1) @binding(0)
var<storage, read> instances: array<Instance>;

@group(2) @binding(0)
var t_heightmap: texture_2d<f32>;
@group(2) @binding(1)
var s_heightmap: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normals: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput, @builtin(instance_index) instance_idx: u32) -> VertexOutput {
    let model_matrix = instances[instance_idx].transform;

    // Sample the heightmap to get the height
    let height_value = textureSample(t_heightmap, s_heightmap, model.tex_coords).r;
    let final_height = height_value * height_scale;

    // Offset the Y-coordinate
    var world_pos = input.position;
    world_pos.y = final_height;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_heightmap, s_heightmap, in.tex_coords);
}