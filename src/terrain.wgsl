struct CameraUniforms {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniforms;

@group(1) @binding(0) var t_heightmap: texture_2d<f32>;
@group(1) @binding(1) var s_heightmap: sampler;

struct LevelUniforms {
    offset_scale: vec4<f32>,
}
@group(2) @binding(0) var<uniform> level: LevelUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    let M = 255.0;
    var world_pos_xz = model.position * M * level.offset_scale.z + level.offset_scale.xy;

    let texture_dims = vec2<f32>(255.0, 255.0);//vec2<f32>(textureDimensions(t_heightmap));
    let uv = world_pos_xz / texture_dims;
    let height = 0.0; //textureSample(t_heightmap, s_heightmap, uv).r;

    let world_position = vec3<f32>(world_pos_xz.x, height * 200.0, world_pos_xz.y);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.5, 0.2, 1.0);
}