struct Uniforms {
    rect: vec4<f32>,      // x, y, width, height
    color: vec4<f32>,     // r, g, b, a
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let pos = positions[vertex_index];
    
    // Convert rect position and size to screen coordinates
    let x = uniforms.rect.x + pos.x * uniforms.rect.z;
    let y = uniforms.rect.y + pos.y * uniforms.rect.w;
    
    // Convert to clip space (-1 to 1)
    let clip_x = (x / uniforms.screen_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (y / uniforms.screen_size.y) * 2.0;

    var output: VertexOutput;
    output.position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return uniforms.color;
}

