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
    if layers.albedo_enabled.w > 0.0 {
        weights = input.color.rgb;
        // Exported layer colours carry weights; ordinary vertex tint is preserved.
        input.color = vec4(1.0);
    }
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
