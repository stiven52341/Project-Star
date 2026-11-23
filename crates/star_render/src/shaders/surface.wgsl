// Draws 3D meshes with their material. Each draw binds its own transform
// and material through a dynamic uniform offset.

struct Draw {
    model_view_projection: mat4x4<f32>,
    base_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> draw: Draw;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = draw.model_view_projection * vec4<f32>(input.position, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return draw.base_color;
}
