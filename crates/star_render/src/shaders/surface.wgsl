// Draws 3D meshes with metallic-roughness materials (Cook-Torrance: GGX
// distribution, Smith geometry, Schlick's Fresnel) lit by the frame's
// lights. Each draw binds its own transforms and material through a dynamic
// uniform offset; the lights are shared by the whole frame.

const PI: f32 = 3.14159265;
const MAX_LIGHTS: u32 = 16u;
// Reflectance of non-metals seen straight on.
const DIELECTRIC_REFLECTANCE: f32 = 0.04;
// Rougher than nothing, so highlights never become infinitely small.
const MIN_ROUGHNESS: f32 = 0.04;
// Keeps divisions away from zero at grazing angles.
const EPSILON: f32 = 0.0001;
// `w` of a light's position for point lights and spots; 0 is a direction.
const POINT: f32 = 1.0;
const SPOT: f32 = 2.0;

struct Draw {
    model_view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    base_color: vec4<f32>,
    // metallic, roughness, unused, unused.
    surface: vec4<f32>,
}

struct Light {
    // xyz: direction (w = 0) or position (w = 1).
    position: vec4<f32>,
    // rgb: color times intensity; a: range of point lights and spots.
    radiance: vec4<f32>,
    // xyz: where a spot points.
    axis: vec4<f32>,
    // x: cosine of the edge of a spot's cone; y: of where it starts to fade.
    cone: vec4<f32>,
}

struct Lights {
    camera_position: vec4<f32>,
    ambient: vec4<f32>,
    count: vec4<u32>,
    lights: array<Light, MAX_LIGHTS>,
}

@group(0) @binding(0)
var<uniform> draw: Draw;

@group(1) @binding(0)
var<uniform> lights: Lights;

@group(2) @binding(0)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(1)
var base_color_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = draw.model_view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = (draw.model * vec4<f32>(input.position, 1.0)).xyz;
    output.world_normal = draw.normal_matrix * input.normal;
    output.uv = input.uv;
    return output;
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / (PI * denominator * denominator);
}

fn geometry_schlick(n_dot_x: f32, roughness: f32) -> f32 {
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    return n_dot_x / (n_dot_x * (1.0 - k) + k);
}

fn fresnel_schlick(cos_theta: f32, reflectance: vec3<f32>) -> vec3<f32> {
    return reflectance + (vec3<f32>(1.0) - reflectance) * pow(1.0 - cos_theta, 5.0);
}

// The light that leaves toward the camera (`view`) from light arriving
// along `to_light` with `radiance`. Scaled by PI, so a light of intensity 1
// on a white matte surface gives white.
fn reflected(
    normal: vec3<f32>,
    view: vec3<f32>,
    to_light: vec3<f32>,
    radiance: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
) -> vec3<f32> {
    let n_dot_l = max(dot(normal, to_light), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let halfway = normalize(view + to_light);
    let n_dot_v = max(dot(normal, view), EPSILON);
    let n_dot_h = max(dot(normal, halfway), 0.0);
    let reflectance = mix(vec3<f32>(DIELECTRIC_REFLECTANCE), albedo, metallic);
    let fresnel = fresnel_schlick(max(dot(halfway, view), 0.0), reflectance);
    let geometry = geometry_schlick(n_dot_v, roughness) * geometry_schlick(n_dot_l, roughness);
    let specular = distribution_ggx(n_dot_h, roughness) * geometry * fresnel
        / (4.0 * n_dot_v * n_dot_l + EPSILON);
    let diffuse = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic) * albedo / PI;
    return (diffuse + specular) * radiance * n_dot_l * PI;
}

// Light that fades with the square of the distance, smoothly reaching
// zero at `range`.
fn attenuation(distance: f32, range: f32) -> f32 {
    let ratio = distance / range;
    let window = clamp(1.0 - ratio * ratio * ratio * ratio, 0.0, 1.0);
    return window * window / max(distance * distance, EPSILON);
}

// How much of a spot reaches a point seen at `cosine` from its axis: all
// inside where it starts to fade (`edges.y`), nothing beyond its edge
// (`edges.x`), smoothly between them.
fn in_cone(cosine: f32, edges: vec2<f32>) -> f32 {
    let fading = clamp((cosine - edges.x) / max(edges.y - edges.x, EPSILON), 0.0, 1.0);
    return fading * fading;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Materials without a texture get a white one.
    let color = draw.base_color * textureSample(base_color_texture, base_color_sampler, input.uv);
    let metallic = clamp(draw.surface.x, 0.0, 1.0);
    let roughness = clamp(draw.surface.y, MIN_ROUGHNESS, 1.0);
    let normal = normalize(input.world_normal);
    let view = normalize(lights.camera_position.xyz - input.world_position);
    var lit = lights.ambient.rgb * color.rgb;
    for (var index = 0u; index < min(lights.count.x, MAX_LIGHTS); index++) {
        let light = lights.lights[index];
        var to_light = -light.position.xyz;
        var radiance = light.radiance.rgb;
        if light.position.w >= POINT {
            let offset = light.position.xyz - input.world_position;
            let distance = length(offset);
            to_light = offset / max(distance, EPSILON);
            radiance = radiance * attenuation(distance, light.radiance.a);
        }
        if light.position.w == SPOT {
            radiance = radiance * in_cone(dot(-to_light, light.axis.xyz), light.cone.xy);
        }
        lit += reflected(normal, view, to_light, radiance, color.rgb, metallic, roughness);
    }
    return vec4<f32>(lit, color.a);
}
