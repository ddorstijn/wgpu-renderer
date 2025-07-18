// terrain_instanced_yup.wgsl

@group(0) @binding(0) var<uniform> u_view_proj : mat4x4<f32>;

@group(1) @binding(0) var u_heightmap : texture_2d<f32>;
@group(1) @binding(1) var u_sampler   : sampler;

struct VSOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       world_pos: vec3<f32>,
    @location(1)       uv: vec2<f32>,
    @location(2)       color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) a_pos: vec2<f32>,
    @location(1) m0: vec4<f32>,
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
    @location(5) c: vec4<f32>
) -> VSOut {
    var out: VSOut;

    // Reconstruct 4×4 model matrix
    let model = mat4x4<f32>(m0, m1, m2, m3);

    // 1) Lift 2D (X,Z,0,1)
    let local = vec4<f32>(a_pos.x, 0.0, a_pos.y, 1.0);

    // 2) Transform to world XZ
    let world_xz4 = model * local;
    let world_xz = world_xz4.xz;

    // 3) Sample heightmap at integer XZ
    let dim = vec2<f32>(textureDimensions(u_heightmap));
    let uv = (world_xz + vec2<f32>(0.5, 0.5)) / dim;
    let samp = textureLoad(u_heightmap, vec2<i32>(world_xz), 0).rg;
    let height = samp.r * 256.0 + samp.g;

    // 4) Assemble Y-up world position
    let world_pos3 = vec3<f32>(world_xz.x, 0.0, world_xz.y);

    // 5) Final clip‐space
    out.clip_pos = u_view_proj * vec4<f32>(world_pos3, 1.0);
    out.world_pos = world_pos3;
    out.uv = uv;
    out.color = c;

    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
    // Simple height‐based shading
    let shade = in.world_pos.y * 0.001;
    return vec4<f32>(shade, shade, shade, 1.0);
}