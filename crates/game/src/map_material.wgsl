#import sekiro_map_coverage::camera_coverage
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace, calculate_F0, calculate_F0_dielectric, calculate_diffuse_color},
    lighting::{EnvBRDFApprox, F_AB, LightingInput, LAYER_BASE, perceptualRoughnessToRoughness},
    mesh_view_bindings::{lights, view},
    pbr_types::{STANDARD_MATERIAL_FLAGS_UNLIT_BIT, PbrInput},
    mesh_functions,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
    pbr_bindings,
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

#import sekiro_map_bindings::{MapGame, Layers}
#ifdef BINDLESS
#import bevy_render::bindless::{bindless_textures_2d, bindless_textures_3d, bindless_textures_cube_array, bindless_samplers_filtering}
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::pbr_bindings::material_indices
#import sekiro_map_bindings::{layer_array, layer_indices, giiv_buffers}
#else
#import sekiro_map_bindings::{layers, overlay, overlay_sampler, snow, snow_sampler, layer_c, c_sampler, overlay_normal, overlay_normal_sampler, snow_normal, snow_normal_sampler, c_normal, c_normal_sampler, height_mask, height_sampler, giiv_buffer, gi_specular, gi_specular_sampler, gi_diffuse, gi_diffuse_sampler, gi_voxels, gi_voxels_sampler, base_normal, base_normal_sampler, detail, detail_sampler, mask_l, mask_l_sampler, wet, wet_sampler}
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

fn sample_base_normal(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
return textureSample(bindless_textures_2d[layer_indices[slot].base_normal], bindless_samplers_filtering[layer_indices[slot].base_normal_sampler], uv);
#else
return textureSample(base_normal, base_normal_sampler, uv);
#endif
}
fn sample_detail(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
return textureSample(bindless_textures_2d[layer_indices[slot].detail], bindless_samplers_filtering[layer_indices[slot].detail_sampler], uv);
#else
return textureSample(detail, detail_sampler, uv);
#endif
}
fn sample_mask_l(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
return textureSample(bindless_textures_2d[layer_indices[slot].mask_l], bindless_samplers_filtering[layer_indices[slot].mask_l_sampler], uv);
#else
return textureSample(mask_l, mask_l_sampler, uv);
#endif
}
fn sample_wet(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
return textureSample(bindless_textures_2d[layer_indices[slot].wet], bindless_samplers_filtering[layer_indices[slot].wet_sampler], uv);
#else
return textureSample(wet, wet_sampler, uv);
#endif
}
fn sample_base_albedo(uv: vec2<f32>, slot: u32) -> vec4<f32> {
#ifdef BINDLESS
return textureSample(bindless_textures_2d[material_indices[slot].base_color_texture], bindless_samplers_filtering[material_indices[slot].base_color_sampler], uv);
#else
return textureSample(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv);
#endif
}
#ifdef GAME_LAYERS
struct MapVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef GAME_LAYERS
    @location(8) blend2: vec4<f32>,
#endif
}

struct MapVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) blend2: vec4<f32>,
    @location(9) uv_scale: f32,
}

@vertex
fn vertex(v: MapVertex) -> MapVertexOutput {
    var out: MapVertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(v.instance_index);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(v.normal, v.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_UVS_A
    out.uv = v.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = v.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, v.tangent, v.instance_index);
#endif
#ifdef VERTEX_COLORS
    out.color = v.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = v.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(v.instance_index, world_from_local[3]);
#endif
#ifdef GAME_LAYERS
    out.blend2 = v.blend2;
    out.uv_scale = length(world_from_local[0].xyz);
#else
    out.blend2 = vec4(1.0, 1.0, 1.0, 0.0);
    out.uv_scale = 1.0;
#endif
    return out;
}


#endif
// hb: the game's height blend of a vertex weight against a mask height.
fn hb(w: f32, h: f32, map_game: MapGame) -> f32 {
    return saturate((w - h * (1.0 - w)) * (1.0 + map_game.p[6].y));
}

// H: a mask's height for one layer: the material's float3 picks the channels, k scales it.
fn H(mask: vec3<f32>, sel: vec3<f32>, k: f32, map_game: MapGame) -> f32 {
    return (dot(mask, sel) * k + 0.01) * map_game.p[6].x;
}

// FLV: a game normal map's xy (z rebuilt), DirectX green flipped into Bevy's tangent frame.
fn flv(n: vec4<f32>) -> vec3<f32> {
    let xy = vec2(n.r * 2.0 - 1.0, 1.0 - n.g * 2.0);
    return vec3(xy, sqrt(max(1.0 - dot(xy, xy), 0.0)));
}

fn has(bit: u32, map_game: MapGame) -> bool {
    return (map_game.flags.x & bit) != 0u;
}

#ifdef VERTEX_UVS_A
fn game_layers(in: VertexOutput, b: vec4<f32>, b2: vec4<f32>, uv_scale: f32, pbr_in: PbrInput, map_game: MapGame, slot: u32) -> PbrInput {
    var pbr_input = pbr_in;
    let p = map_game.p;
    let ground = map_game.flags.y == 2u;
    let uv0 = in.uv;
    var uv1 = in.uv;
#ifdef VERTEX_UVS_B
    uv1 = in.uv_b;
#endif
    let uvs = uv0 * uv_scale;
    var m9 = vec3(0.0);
    if has(128u, map_game) { m9 = sample_height_mask(uvs, slot).rgb; }
    // M[Multiple]'s layer L / wet / tint mask (slot 6) is on uv1; M[MultipleGround] has none and
    // uses mask 9 for every layer.
    var m6 = m9;
    if has(512u, map_game) { m6 = sample_mask_l(uv1, slot).rgb; }

    var wOv = 0.0;
    if has(2u, map_game) { wOv = hb(b.x, H(m9, p[9].xyz, p[7].x, map_game), map_game); }
    var wL = 0.0;
    if has(32u, map_game) {
        if ground { wL = hb(b.y, H(m9, p[10].xyz, p[7].y, map_game), map_game); } else { wL = hb(b.y, H(m6, p[10].xyz, p[7].y, map_game), map_game); }
    }
    var wWet = 0.0;
    if has(1024u, map_game) {
        if ground { wWet = saturate(b2.w * p[7].z); } else { wWet = hb(b2.w, H(m6, p[11].xyz, p[7].z, map_game), map_game); }
    }
    let lum = dot(1.0 - b2.rgb, vec3(0.29891, 0.58661, 0.11448));
    let wTint = hb(lum, H(m6, p[12].xyz, p[8].x, map_game), map_game);

    // Normals: base / overlay (B gloss), the detail normal's xy added, layer L (ground).
    var nb = vec4(0.5, 0.5, 1.0, 0.15);
    if has(1u, map_game) { nb = sample_base_normal(uvs, slot); }
    if has(4u, map_game) { nb = mix(nb, sample_overlay_normal(uvs, slot), wOv); }
    if ground && has(64u, map_game) { nb = mix(nb, sample_c_normal(uvs, slot), wL); }
    var nt = flv(nb);
    var gloss = nb.b;
    if has(256u, map_game) {
        let d = mix(vec3(0.502, 0.502, 1.0), sample_detail(uv0, slot).rgb, b.w);
        nt = vec3(nt.xy + flv(vec4(d, 1.0)).xy * p[6].z, nt.z);
    }
    var Nw = pbr_input.world_normal;
#ifdef VERTEX_TANGENTS
    let N0 = pbr_input.world_normal;
    let T = in.world_tangent.xyz;
    let B = in.world_tangent.w * cross(N0, T);
    Nw = normalize(nt.x * T + nt.y * B + normalize(nt).z * N0);
#endif
    // Snow: how much the bumped world normal faces the material's snow direction (not
    // normalised), times the vertex weight, height-blended on mask 9.
    var wSnow = 0.0;
    if has(8u, map_game) {
        let sd = dot(p[14].xyz, Nw) * b.z;
        wSnow = hb(sd, H(m9, p[13].xyz, p[7].w, map_game), map_game);
    }
    // Wet: a flow normal scrolled on uv1, gloss toward the wet gloss.
    if wWet > 0.0 {
        let wn = sample_wet(uv1 + globals.time * p[15].xy, slot);
        nt = vec3(nt.xy + flv(wn).xy * p[6].w * wWet, nt.z);
        gloss = mix(gloss, p[8].y, wWet);
    }
    if wSnow > 0.0 && has(16u, map_game) {
        let sn = sample_snow_normal(uvs, slot);
        if ground {
            nt = vec3(nt.xy + flv(sn).xy * p[9].w * wSnow, nt.z);
        } else {
            nt = mix(nt, flv(sn), wSnow);
        }
        gloss = mix(gloss, sn.b, wSnow);
    }

    // Albedo.
    var a = vec3(1.0);
    if has(2048u, map_game) { a = sample_base_albedo(uvs, slot).rgb; }
    var alpha = pbr_in.material.base_color.a;
    if ground {
        if has(2u, map_game) { a = mix(a, sample_overlay(uvs, slot).rgb, wOv); }
        if has(32u, map_game) { a = mix(a, sample_layer_c(uvs, slot).rgb, wL); }
        a = a * mix(vec3(1.0), b2.rgb, p[8].x);
        // gap: M[MultipleGround] wet darkening read as a lerp toward color_1 (kb/map.md).
        a = mix(a, p[4].rgb, wWet * p[10].w);
    } else {
        a = a * mix(p[0].rgb, p[1].rgb, b.x);
        if has(2u, map_game) { a = mix(a, sample_overlay(uvs, slot).rgb * p[2].rgb, wOv); }
        if has(32u, map_game) { a = mix(a, sample_layer_c(uv1 + p[8].zw, slot).rgb * p[3].rgb, wL); }
        a = mix(a, a * b2.rgb * p[4].rgb, wTint);
    }
    if has(8u, map_game) { a = mix(a, sample_snow(uvs, slot).rgb * p[5].rgb, wSnow); }
    a = saturate(a);
    if map_game.flags.z != 0u {
        // Debug (SHINOBI_MAP_SHOW_BLEND=1): overlay red, layer L green, snow blue.
        a = vec3(wOv, wL, wSnow);
    }
    pbr_input.material.base_color = vec4(a, alpha);
    pbr_input.material.perceptual_roughness = clamp(1.0 - saturate(gloss), 0.089, 1.0);
#ifdef VERTEX_TANGENTS
    var mapped = normalize(nt.x * T + nt.y * B + nt.z * N0);
    if dot(N0, pbr_input.V) < 0.0 {
        mapped = mapped - 2.0 * dot(mapped, N0) / dot(N0, N0) * N0;
    }
    pbr_input.N = mapped;
#endif
    return pbr_input;
}
#endif


@fragment
fn fragment(
#ifdef GAME_LAYERS
m: MapVertexOutput,
#else
vertex: VertexOutput,
#endif
@builtin(front_facing) is_front: bool) -> FragmentOutput {
#ifdef BINDLESS
#ifdef GAME_LAYERS
    var vertex: VertexOutput;
    vertex.position = m.position;
    vertex.world_position = m.world_position;
    vertex.world_normal = m.world_normal;
#ifdef VERTEX_UVS_A
    vertex.uv = m.uv;
#endif
#ifdef VERTEX_UVS_B
    vertex.uv_b = m.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    vertex.world_tangent = m.world_tangent;
#endif
#ifdef VERTEX_COLORS
    vertex.color = m.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    vertex.instance_index = m.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    vertex.visibility_range_dither = m.visibility_range_dither;
#endif
#endif
    let slot = mesh[vertex.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let layers = layer_array[layer_indices[slot].layers];
#else
#ifdef GAME_LAYERS
    var vertex: VertexOutput;
    vertex.position = m.position;
    vertex.world_position = m.world_position;
    vertex.world_normal = m.world_normal;
#ifdef VERTEX_UVS_A
    vertex.uv = m.uv;
#endif
#ifdef VERTEX_UVS_B
    vertex.uv_b = m.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    vertex.world_tangent = m.world_tangent;
#endif
#ifdef VERTEX_COLORS
    vertex.color = m.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    vertex.instance_index = m.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    vertex.visibility_range_dither = m.visibility_range_dither;
#endif
#endif
    let slot = 0u;
#endif
    var input = vertex;
#ifdef CAMERA_FADE
    camera_coverage(input.position.xy, input.world_position.xyz, layers.camera_fade);
#endif
    var b = vec4(0.0);
#ifdef VERTEX_COLORS
    b = input.color;
#endif
    var weights = vec3(0.0);
#ifdef VERTEX_COLORS
    if layers.albedo_enabled.w > 0.0 {
        weights = input.color.rgb;
        // Exported layer colours carry weights; ordinary vertex tint is preserved.
        input.color = vec4(1.0);
    }
#endif
    var pbr = pbr_input_from_standard_material(input, is_front);
#ifdef GAME_LAYERS
    pbr = game_layers(input, b, m.blend2, m.uv_scale, pbr, layers.game, slot);
#else
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
