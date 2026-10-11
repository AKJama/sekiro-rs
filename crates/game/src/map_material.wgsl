#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace, calculate_F0, calculate_diffuse_color},
    lighting::{EnvBRDFApprox, F_AB},
    mesh_view_bindings::{lights, view},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

struct Layers {
    enabled: vec4<f32>,
    albedo_enabled: vec4<f32>,
    albedo_uv: vec4<f32>,
    normal_uv: vec4<f32>,
    normal_enabled: vec4<f32>,
    hemi_axis: vec4<f32>,
    hemi_delta: vec4<f32>,
}
#ifdef BINDLESS
#import bevy_render::bindless::{bindless_textures_2d, bindless_samplers_filtering}
#import bevy_pbr::mesh_bindings::mesh
struct LayerIndices {
    layers: u32,
    overlay: u32,
    overlay_sampler: u32,
    snow: u32,
    snow_sampler: u32,
    layer_c: u32,
    c_sampler: u32,
    overlay_normal: u32,
    overlay_normal_sampler: u32,
    snow_normal: u32,
    snow_normal_sampler: u32,
    c_normal: u32,
    c_normal_sampler: u32,
    height_mask: u32,
    height_sampler: u32,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage> layer_indices: array<LayerIndices>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<storage> layer_array: array<Layers>;
#else
@group(#{MATERIAL_BIND_GROUP}) @binding(50) var<uniform> layers: Layers;
@group(#{MATERIAL_BIND_GROUP}) @binding(51) var overlay: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(52) var overlay_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(53) var snow: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(54) var snow_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(55) var layer_c: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(56) var c_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(57) var overlay_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(58) var overlay_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(59) var snow_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(60) var snow_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(61) var c_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(62) var c_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(63) var height_mask: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(64) var height_sampler: sampler;
#endif

fn sample_overlay(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].overlay], bindless_samplers_filtering[layer_indices[slot].overlay_sampler], uv);
#else
    return textureSample(overlay, overlay_sampler, uv);
#endif
}

fn sample_snow(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].snow], bindless_samplers_filtering[layer_indices[slot].snow_sampler], uv);
#else
    return textureSample(snow, snow_sampler, uv);
#endif
}

fn sample_layer_c(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].layer_c], bindless_samplers_filtering[layer_indices[slot].c_sampler], uv);
#else
    return textureSample(layer_c, c_sampler, uv);
#endif
}

fn sample_overlay_normal(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].overlay_normal], bindless_samplers_filtering[layer_indices[slot].overlay_normal_sampler], uv);
#else
    return textureSample(overlay_normal, overlay_normal_sampler, uv);
#endif
}

fn sample_snow_normal(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].snow_normal], bindless_samplers_filtering[layer_indices[slot].snow_normal_sampler], uv);
#else
    return textureSample(snow_normal, snow_normal_sampler, uv);
#endif
}

fn sample_c_normal(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].c_normal], bindless_samplers_filtering[layer_indices[slot].c_normal_sampler], uv);
#else
    return textureSample(c_normal, c_normal_sampler, uv);
#endif
}

fn sample_height_mask(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[layer_indices[slot].height_mask], bindless_samplers_filtering[layer_indices[slot].height_sampler], uv);
#else
    return textureSample(height_mask, height_sampler, uv);
#endif
}

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
#ifdef BINDLESS
    let slot = mesh[vertex.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let layers = layer_array[layer_indices[slot].layers];
#else
    let slot = 0u;
#endif
    var input = vertex;
    var weights = vec3(0.0);
#ifdef VERTEX_COLORS
    if layers.albedo_enabled.w > 0.0 {
        weights = input.color.rgb;
        // Exported layer colours carry weights; ordinary vertex tint is preserved.
        input.color = vec4(1.0);
    }
#endif
    var pbr = pbr_input_from_standard_material(input, is_front);
    var height = 0.5;
    if layers.enabled.w > 0.0 {
        height = sample_height_mask(layer_uv(input, layers.albedo_uv.w), slot).r;
    }
    let facing = smoothstep(0.2, 0.7, normalize(input.world_normal).y);
    let w = vec3(coverage(weights.x, height), coverage(weights.y, 1.0-height) * facing, coverage(weights.z, height)) * layers.enabled.xyz;
    var color = pbr.material.base_color.rgb;
    if layers.albedo_enabled.x > 0.0 { color = mix(color, sample_overlay(layer_uv(input, layers.albedo_uv.x), slot).rgb, w.x); }
    if layers.albedo_enabled.z > 0.0 { color = mix(color, sample_layer_c(layer_uv(input, layers.albedo_uv.z), slot).rgb, w.z); }
    if layers.albedo_enabled.y > 0.0 { color = mix(color, sample_snow(layer_uv(input, layers.albedo_uv.y), slot).rgb, w.y); }
    pbr.material.base_color = vec4(color, pbr.material.base_color.a);
#ifdef VERTEX_TANGENTS
    let tbn = calculate_tbn_mikktspace(input.world_normal, input.world_tangent);
    if layers.normal_enabled.x > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(sample_overlay_normal(layer_uv(input, layers.normal_uv.x), slot).rg)), w.x)); }
    if layers.normal_enabled.z > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(sample_c_normal(layer_uv(input, layers.normal_uv.z), slot).rg)), w.z)); }
    if layers.normal_enabled.y > 0.0 { pbr.N = normalize(mix(pbr.N, normalize(tbn * unpack_normal(sample_snow_normal(layer_uv(input, layers.normal_uv.y), slot).rg)), w.y)); }
#endif
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    var out: FragmentOutput;
    let unlit = (pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u;
    out.color = pbr.material.base_color;
    if !unlit { out.color = apply_pbr_lighting(pbr); }
    if layers.hemi_axis.w > 0.0 && !unlit {
        // Verified 310630 hemisphere: mean + dot(axis,N)*(up-down)/2.
        // Inferred bridge retains Bevy's ambient BRDF and replaces its flat colour.
        let mean = lights.ambient_color.rgb;
        let delta = layers.hemi_delta.xyz;
        let n_dot_v = max(dot(pbr.N,pbr.V),0.0001);
        let diffuse = calculate_diffuse_color(pbr.material.base_color.rgb,pbr.material.metallic,pbr.material.specular_transmission,pbr.material.diffuse_transmission);
        let f0 = calculate_F0(pbr.material.base_color.rgb,pbr.material.metallic,pbr.material.reflectance);
        let response = EnvBRDFApprox(diffuse,F_AB(1.0,n_dot_v))
            + EnvBRDFApprox(f0,F_AB(pbr.material.perceptual_roughness,n_dot_v))*clamp(dot(f0,vec3(50.0*0.33)),0.0,1.0);
        // Evaluate the colour difference directly, avoiding a division by a dark mean.
        let irradiance = max(mean+dot(layers.hemi_axis.xyz,pbr.N)*delta,vec3(0.0));
        out.color = vec4(out.color.rgb + response*(irradiance-mean)*pbr.diffuse_occlusion*view.exposure,out.color.a);
    }
    if layers.normal_enabled.w > 0.0 { out.color = vec4(weights, 1.0); }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
