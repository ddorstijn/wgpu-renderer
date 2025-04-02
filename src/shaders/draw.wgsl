struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, // Clip-space position
    @location(0) height: f32,                   // Pass height to the fragment shader
};

@vertex
fn main_vs(
    @location(0) particle_pos: vec2<f32>,
    @location(1) particle_vel: vec2<f32>,
    @location(2) position: vec3<f32>,
) -> VertexOutput {
    // let angle = -atan2(particle_vel.x, particle_vel.y);
    // let pos = vec2<f32>(
    //     position.x * cos(angle) - position.z * sin(angle),
    //     position.x * sin(angle) + position.z * cos(angle),
    // );

    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(position + vec3<f32>(particle_pos.x, 0.0, particle_pos.y), 1.0);
    output.height = position.y * 0.1;
    return output;
}

@fragment
fn main_fs(
    @location(0) height: f32, // Receive the height from the vertex shader
) -> @location(0) vec4<f32> {
    // Map the height to a color gradient (e.g., blue for low, red for high)
    let color = vec3<f32>(height, 0.0, 0.01);
    return vec4<f32>(color, 1.0); // Return the color with full opacity
}