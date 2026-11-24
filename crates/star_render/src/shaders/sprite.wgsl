// Draws textured, tinted sprites lit by the frame's 2D lights. All the
// sprites of a frame share one vertex buffer, the camera and the lights;
// each batch binds the texture of its sprites.

const MAX_LIGHTS: u32 = 16u;
// `w` of a light's place for spots; 1 is a point light.
const SPOT: f32 = 2.0;
// Keeps divisions away from zero.
const EPSILON: f32 = 0.0001;

struct Camera {
    view_projection: mat4x4<f32>,
}

struct Light {
    // xy: position; z: range; w: kind.
    place: vec4<f32>,
    // rgb: color times intensity.
    radiance: vec4<f32>,
    // xy: where a spot points; z: cosine of its cone's edge; w: of where
    // it starts to fade.
    cone: vec4<f32>,
}

struct Lights {
    ambient: vec4<f32>,
    count: vec4<u32>,
    lights: array<Light, MAX_LIGHTS>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var<uniform> lights: Lights;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) texture_point: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_point: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 0.0, 1.0);
    output.texture_point = input.texture_point;
    output.color = input.color;
    output.world_position = input.position;
    return output;
}

// How much of a light reaches `point`: all at the light, nothing at its
// range, smoothly between; and for spots, only inside their cone.
fn reach(light: Light, point: vec2<f32>) -> f32 {
    let offset = point - light.place.xy;
    let distance = length(offset);
    let ratio = distance / light.place.z;
    let window = clamp(1.0 - ratio * ratio, 0.0, 1.0);
    var amount = window * window;
    if light.place.w == SPOT {
        let cosine = dot(offset / max(distance, EPSILON), light.cone.xy);
        let fading = clamp((cosine - light.cone.z) / max(light.cone.w - light.cone.z, EPSILON), 0.0, 1.0);
        amount = amount * fading * fading;
    }
    return amount;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(sprite_texture, sprite_sampler, input.texture_point) * input.color;
    var lit = lights.ambient.rgb;
    for (var index = 0u; index < min(lights.count.x, MAX_LIGHTS); index++) {
        let light = lights.lights[index];
        lit += light.radiance.rgb * reach(light, input.world_position);
    }
    return vec4<f32>(color.rgb * lit, color.a);
}
