struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, // Clip-space position
    @location(1) world_position: vec3<f32>,     // Custom output for world position
};

@vertex
fn main_vs(
    @location(0) particle_pos: vec2<f32>,
    @location(1) particle_vel: vec2<f32>,
    @location(2) position: vec3<f32>,
) -> VertexOutput {
    let angle = -atan2(particle_vel.x, particle_vel.y);
    let pos = vec2<f32>(
        position.x * cos(angle) - position.y * sin(angle),
        position.x * sin(angle) + position.y * cos(angle),
    );

    let world_pos = vec3<f32>(pos + particle_pos, position.z);

    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    output.world_position = vec3<f32>(pos + particle_pos, position.z);

    return output;
}

@fragment
fn main_fs(
    @location(1) world_position: vec3<f32>, // Receive world position
) -> @location(0) vec4<f32> {
    // Use the world position for color calculations
    let red = world_position.x * 0.5 + 0.5;
    let green = world_position.y * 0.5 + 0.5;
    let blue = world_position.z * 1.5;

    let color = vec3<f32>(red, green, blue);
    return vec4<f32>(color, 1.0); // Return the color with full opacity
}