#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace, calculate_F0, calculate_F0_dielectric, calculate_diffuse_color},
    lighting::{EnvBRDFApprox, F_AB, LightingInput, LAYER_BASE, perceptualRoughnessToRoughness},
    mesh_view_bindings::{lights, view},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#ifdef ENVIRONMENT_MAP
#import bevy_pbr::environment_map::environment_map_light
#endif
#import bevy_pbr::clustered_forward::{view_fragment_cluster_index, unpack_clusterable_object_index_ranges}

struct GiScatter { FssEss: vec3<f32>, FmsEms: vec3<f32>, Edss: vec3<f32>, }
fn gi_multiscatter(f0: vec3<f32>, f_ab: vec2<f32>, missing: f32, occlusion: f32) -> GiScatter {
    let single = EnvBRDFApprox(f0,f_ab)*occlusion;
    let average = f0+(1.0-f0)/21.0;
    let multiple = single*average/(1.0-missing*average)*missing;
    return GiScatter(single,multiple,1.0-single-multiple);
}

struct Layers {
    enabled: vec4<f32>,
    albedo_enabled: vec4<f32>,
    albedo_uv: vec4<f32>,
    normal_uv: vec4<f32>,
    normal_enabled: vec4<f32>,
    hemi_axis: vec4<f32>,
    hemi_delta: vec4<f32>,
    camera_fade: vec4<f32>,
}
#ifdef BINDLESS
#import bevy_render::bindless::{bindless_textures_2d, bindless_textures_3d, bindless_textures_cube_array, bindless_samplers_filtering}
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
    irradiance: u32,
    specular: u32,
    specular_sampler: u32,
    diffuse: u32,
    diffuse_sampler: u32,
    voxels: u32,
    voxels_sampler: u32,
}
struct GiivBuffer { words: array<u32>, }
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<storage, read> giiv_buffers: binding_array<GiivBuffer>;
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
struct GiivBuffer { words: array<u32>, }
@group(#{MATERIAL_BIND_GROUP}) @binding(65) var<storage, read> giiv_buffer: GiivBuffer;
@group(#{MATERIAL_BIND_GROUP}) @binding(66) var gi_specular: texture_cube_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(67) var gi_specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(68) var gi_diffuse: texture_cube_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(69) var gi_diffuse_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(70) var gi_voxels: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(71) var gi_voxels_sampler: sampler;
#endif

fn gi_cube(slot: u32, index: u32, direction: vec3<f32>, mip: f32, diffuse: bool) -> vec3<f32> {
    let d = vec3(direction.xy,-direction.z);
#ifdef BINDLESS
    if diffuse { return textureSampleLevel(bindless_textures_cube_array[layer_indices[slot].diffuse],bindless_samplers_filtering[layer_indices[slot].diffuse_sampler],d,i32(index),0.0).rgb; }
    return textureSampleLevel(bindless_textures_cube_array[layer_indices[slot].specular],bindless_samplers_filtering[layer_indices[slot].specular_sampler],d,i32(index),mip).rgb;
#else
    if diffuse { return textureSampleLevel(gi_diffuse,gi_diffuse_sampler,d,i32(index),0.0).rgb; }
    return textureSampleLevel(gi_specular,gi_specular_sampler,d,i32(index),mip).rgb;
#endif
}

fn gi_word(slot: u32, index: u32) -> u32 {
#ifdef BINDLESS
    return giiv_buffers[layer_indices[slot].irradiance].words[index];
#else
    return giiv_buffer.words[index];
#endif
}

fn gi_vec(slot: u32, index: u32) -> vec4<f32> {
    return bitcast<vec4<f32>>(vec4(gi_word(slot,index), gi_word(slot,index+1u), gi_word(slot,index+2u), gi_word(slot,index+3u)));
}

// Native sampling interpolates the exponent-packed values BEFORE decoding.
fn gi_sample(slot: u32, record: u32, uv: vec3<f32>, bias: vec3<f32>, axis: u32) -> vec3<f32> {
    let d = record+40u+axis*3u;
    let dimensions = vec3(gi_word(slot,d),gi_word(slot,d+1u),gi_word(slot,d+2u));
    let sample_uv = uv+bias/vec3<f32>(dimensions);
    // Clamp to the region's voxel centres so filtering never touches another atlas tile.
    let p = clamp(sample_uv*vec3<f32>(dimensions),vec3(0.5),vec3<f32>(dimensions)-0.5);
    let origin = vec3(gi_word(slot,record+20u)+axis*gi_word(slot,record+23u),gi_word(slot,record+21u),gi_word(slot,record+22u));
#ifdef BINDLESS
    let size = vec3<f32>(textureDimensions(bindless_textures_3d[layer_indices[slot].voxels]));
    let encoded = textureSampleLevel(bindless_textures_3d[layer_indices[slot].voxels],bindless_samplers_filtering[layer_indices[slot].voxels_sampler],(vec3<f32>(origin)+p)/size,0.0);
#else
    let size = vec3<f32>(textureDimensions(gi_voxels));
    let encoded = textureSampleLevel(gi_voxels,gi_voxels_sampler,(vec3<f32>(origin)+p)/size,0.0);
#endif
    return 2.0*(encoded.rgb-0.5)*exp2(20.0*(encoded.a-0.5));
}

struct NativeAmbient { irradiance: vec3<f32>, lambert: vec3<f32>, specular: vec3<f32>, coverage: f32, }

fn gi_lighting(slot: u32, position: vec3<f32>, normal: vec3<f32>, reflection: vec3<f32>, mip: f32) -> NativeAmbient {
    var result: NativeAmbient;
    let n = vec3(-normal.x, normal.y, normal.z);
    // Large enclosing boxes first; smaller authored regions fade over their parent.
    for (var i=0u; i<gi_word(slot,0u); i+=1u) {
        let r = 4u+i*52u;
        if any(position<gi_vec(slot,r+24u).xyz) || any(position>gi_vec(slot,r+28u).xyz) { continue; }
        let transform = mat4x4(gi_vec(slot,r),gi_vec(slot,r+4u),gi_vec(slot,r+8u),gi_vec(slot,r+12u));
        let uv = (transform*vec4(position,1.0)).xyz;
        let dimensions = vec3(gi_word(slot,r+16u),gi_word(slot,r+17u),gi_word(slot,r+18u));
        let edge = min(uv,1.0-uv)*vec3<f32>(dimensions);
        let fade = smoothstep(0.0,1.0,min(edge.x,min(edge.y,edge.z)));
        if fade<=0.0 { continue; }
        if gi_word(slot,1u)>0u {
            // One native voxel of normal bias in local volume coordinates.
            let local_n = (transform*vec4(normal,0.0)).xyz;
            let bias = normalize(local_n);
            let x = gi_sample(slot,r,uv,bias,0u);
            let y = gi_sample(slot,r,uv,bias,1u);
            let z = gi_sample(slot,r,uv,bias,2u);
            let w = gi_sample(slot,r,uv,bias,3u);
            let light = max(n.x*x+n.y*y+n.z*z+w,vec3(0.0));
            result.irradiance = mix(result.irradiance,light,fade);
        }
        let cube = gi_word(slot,r+19u);
        if gi_word(slot,1u)==0u {
            result.lambert = mix(result.lambert,gi_cube(slot,cube,normal,0.0,true),fade);
        }
        // Intersect the receiver's reflected ray with the authored oriented box.
        // Env_Point supplies the capture origin; separate native PCC bounds remain untraced.
        let ray = (transform*vec4(reflection,0.0)).xyz;
        let safe_ray = select(vec3(0.0000001),ray,abs(ray)>vec3(0.0000001));
        let times = (select(vec3(0.0),vec3(1.0),safe_ray>vec3(0.0))-uv)/safe_ray;
        let t = min(times.x,min(times.y,times.z));
        let hit = uv+ray*t;
        var specular = vec3(0.0);
        if t>=0.0 && all(hit>=vec3(-0.0001)) && all(hit<=vec3(1.0001)) {
            let direction = position+reflection*t-gi_vec(slot,r+36u).xyz;
            if dot(direction,direction)>0.000001 { specular = gi_cube(slot,cube,direction,mip,false); }
        }
        result.specular = mix(result.specular,specular,fade);
        result.coverage += (1.0-result.coverage)*fade;
    }
    return result;
}

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
    if layers.camera_fade.x > 0.0 {
        let opacity = distance(input.world_position.xyz, view.world_position) * layers.camera_fade.z - layers.camera_fade.y;
        // Native camera fade samples an 8x8 noise tile; ordered coverage is provisional.
        let p = vec2<u32>(input.position.xy) & vec2(7u);
        let a = ((p.x & 1u) ^ (p.y & 1u)) * 2u + (p.y & 1u);
        let b = (((p.x >> 1u) & 1u) ^ ((p.y >> 1u) & 1u)) * 2u + ((p.y >> 1u) & 1u);
        let c = (((p.x >> 2u) & 1u) ^ ((p.y >> 2u) & 1u)) * 2u + ((p.y >> 2u) & 1u);
        let noise = (f32(a * 16u + b * 4u + c) + 0.5) / 64.0;
        if opacity < 0.01 + 0.99 * noise { discard; }
    }
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
    if !unlit {
        let gi = gi_lighting(slot,pbr.world_position.xyz,pbr.N,reflect(-pbr.V,pbr.N),min(6.5*pbr.material.perceptual_roughness,6.0));
        if gi.coverage>0.0 {
            let nv = max(dot(pbr.N,pbr.V),0.0001);
            let diffuse = calculate_diffuse_color(pbr.material.base_color.rgb,pbr.material.metallic,pbr.material.specular_transmission,pbr.material.diffuse_transmission);
            var fallback = vec3(0.0);
            var fallback_specular = vec3(0.0);
#ifdef ENVIRONMENT_MAP
            var li: LightingInput;
            li.layers[LAYER_BASE].N = pbr.N;
            li.layers[LAYER_BASE].R = reflect(-pbr.V,pbr.N);
            li.layers[LAYER_BASE].NdotV = nv;
            li.layers[LAYER_BASE].perceptual_roughness = pbr.material.perceptual_roughness;
            li.layers[LAYER_BASE].roughness = perceptualRoughnessToRoughness(pbr.material.perceptual_roughness);
            li.P = pbr.world_position.xyz;
            li.V = pbr.V;
            li.diffuse_color = diffuse;
            li.metallic = pbr.material.metallic;
            li.F0_dielectric = calculate_F0_dielectric(pbr.material.reflectance);
            li.F0_metallic = pbr.material.base_color.rgb;
            li.F_ab = F_AB(pbr.material.perceptual_roughness,nv);
            let vz = (view.view_from_world*pbr.world_position).z;
            var ranges = unpack_clusterable_object_index_ranges(view_fragment_cluster_index(pbr.frag_coord.xy,vz,pbr.is_orthographic));
            let environment = environment_map_light(&li,&ranges,false);
            fallback = environment.diffuse;
            fallback_specular = environment.specular;
#endif
            if layers.hemi_axis.w>0.0 {
                fallback += EnvBRDFApprox(diffuse,F_AB(1.0,nv))*max(lights.ambient_color.rgb+dot(layers.hemi_axis.xyz,pbr.N)*layers.hemi_delta.xyz,vec3(0.0));
            }
            let f_ab = F_AB(pbr.material.perceptual_roughness,nv);
            let f0 = calculate_F0(pbr.material.base_color.rgb,pbr.material.metallic,pbr.material.reflectance);
            let specular_occlusion = clamp(dot(f0,vec3(50.0*0.33)),0.0,1.0);
            let dielectric = gi_multiscatter(calculate_F0_dielectric(pbr.material.reflectance),f_ab,1.0-f_ab.x-f_ab.y,specular_occlusion);
            let metallic = gi_multiscatter(pbr.material.base_color.rgb,f_ab,1.0-f_ab.x-f_ab.y,specular_occlusion);
            let fss = mix(dielectric.FssEss,metallic.FssEss,pbr.material.metallic);
            let fms = mix(dielectric.FmsEms,metallic.FmsEms,pbr.material.metallic);
            var bounce = (fms+diffuse*dielectric.Edss)*gi.lambert;
            if gi_word(slot,1u)>0u { bounce = diffuse*gi.irradiance; }
            let delta_diffuse = bounce*layers.hemi_delta.w-fallback*gi.coverage;
            let delta_specular = fss*gi.specular*layers.hemi_delta.w-fallback_specular*gi.coverage;
            out.color = vec4(out.color.rgb+(delta_diffuse*pbr.diffuse_occlusion+delta_specular*pbr.specular_occlusion)*view.exposure,out.color.a);
        }
    }
    if layers.normal_enabled.w > 0.0 { out.color = vec4(weights, 1.0); }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}
