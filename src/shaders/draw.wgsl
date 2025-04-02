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
    let angle = -atan2(particle_vel.x, particle_vel.y);
    let pos = vec2<f32>(
        position.x* 0.01 * cos(angle) - position.y * 0.01 * sin(angle),
        position.x* 0.01 * sin(angle) + position.y * 0.01 * cos(angle),
    );

    // return vec4<f32>(position + vec3<f32>(particle_pos, 0.0), 1.0);
    var output: VertexOutput;
    output.clip_position = vec4<f32>((pos + particle_pos), position.z * 0.01, 1.0);
    output.height = position.z * 0.01; // Pass the height (z component) to the fragment shader
    return output;
}

@fragment
fn main_fs(
    @location(0) height: f32, // Receive the height from the vertex shader
) -> @location(0) vec4<f32> {
    // Map the height to a color gradient (e.g., blue for low, red for high)
    let color = vec3<f32>(
        height * 0.5 + 0.5, // Red channel (scaled height)
        0.0,                // Green channel (constant)
        1.0 - (height * 0.5 + 0.5), // Blue channel (inverse of height)
    );
    return vec4<f32>(color, 1.0); // Return the color with full opacity
}