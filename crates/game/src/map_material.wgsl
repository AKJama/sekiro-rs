#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace},
}

struct Layers {
    enabled: vec4<f32>,
    albedo_enabled: vec4<f32>,
    albedo_uv: vec4<f32>,
    normal_uv: vec4<f32>,
    normal_enabled: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> layers: Layers;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var overlay: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var overlay_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var snow: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var snow_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var layer_c: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var c_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var overlay_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var overlay_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var snow_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var snow_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var c_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var c_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var height_mask: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(114) var height_sampler: sampler;

fn layer_uv(in: VertexOutput, channel: f32) -> vec2<f32> {
#ifdef VERTEX_UVS_B
    return mix(in.uv, in.uv_b, channel);
#else
    return in.uv;
#endif
}

// Endpoint-preserving height coverage, an approximation until native sat shaders are traced.
fn coverage(weight: f32, height: f32) -> f32 {
    if weight <= 0.0 { return 0.0; }
    if weight >= 1.0 { return 1.0; }
    return smoothstep(height - 0.25, height + 0.25, weight);
}

fn unpack_normal(sample: vec2<f32>) -> vec3<f32> {
    let xy = sample * 2.0 - 1.0;
    return normalize(vec3(xy, sqrt(max(1.0 - dot(xy, xy), 0.001))));
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var input = vertex;
    var weights = vec3(0.0);
#ifdef VERTEX_COLORS
    weights = input.color.rgb;
    // These colours carry blend weights, never a tint on the base StandardMaterial.
    input.color = vec4(1.0);
#endif
    var pbr = pbr_input_from_standard_material(input, is_front);
    var height = 0.5;
    if layers.enabled.w > 0.0 {
        height = textureSample(height_mask, height_sampler, layer_uv(input, layers.albedo_uv.w)).r;
    }
    let facing = smoothstep(0.2, 0.7, normalize(input.world_normal).y);
    let w = vec3(coverage(weights.x, height), coverage(weights.y, 1.0-height) * facing, coverage(weights.z, height)) * layers.enabled.xyz;
    var color = pbr.material.base_color.rgb;
    if layers.albedo_enabled.x > 0.0 { color = mix(color, textureSample(overlay, overlay_sampler, layer_uv(input, layers.albedo_uv.x)).rgb, w.x); }
    if layers.albedo_enabled.z > 0.0 { color = mix(color, textureSample(layer_c, c_sampler, layer_uv(input, layers.albedo_uv.z)).rgb, w.z); }
    if layers.albedo_enabled.y > 0.0 { color = mix(color, textureSample(snow, snow_sampler, layer_uv(input, layers.albedo_uv.y)).rgb, w.y); }
    pbr.material.base_color = vec4(color, pbr.material.base_color.a);
#ifdef VERTEX_TANGENTS
    let tbn = calculate_tbn_mikktspace(input.world_normal, input.world_tangent);
    if layers.normal_enabled.x > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(textureSample(overlay_normal, overlay_normal_sampler, layer_uv(input, layers.normal_uv.x)).rg)), w.x)); }
    if layers.normal_enabled.z > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(textureSample(c_normal, c_normal_sampler, layer_uv(input, layers.normal_uv.z)).rg)), w.z)); }
    if layers.normal_enabled.y > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(textureSample(snow_normal, snow_normal_sampler, layer_uv(input, layers.normal_uv.y)).rg)), w.y)); }
#endif
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    if layers.normal_enabled.w > 0.0 { out.color = vec4(weights, 1.0); }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
