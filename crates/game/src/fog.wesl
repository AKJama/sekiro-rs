import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
import bevy_pbr::render::mesh_view_types::{Lights, DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT};

struct Fog {
    density: vec4<f32>, depths: vec4<f32>, sky: vec4<f32>,
    direct: vec4<f32>, opposite: vec4<f32>, direction: vec4<f32>, extinction: vec4<f32>,
    slices: vec4<f32>,
    background_control: vec4<f32>,
    background: array<vec4<f32>,16>,
    radiance_scale: vec4<f32>,
    // DirLight ShadowSampleCount (0 = unshadowed), DirLight ShadowDither, VolumeWidth, unused.
    shadow: vec4<f32>,
}
// SHADOW_VOLUME is the DirLightShadow compute pass: one shadow mean per froxel cell, which the
// fragment variants read back filtered, as PreShadowed reads the native shadow volume.
@if(SHADOW_VOLUME) {
    @group(0) @binding(0) var<uniform> view: View;
    @group(0) @binding(1) var<uniform> fog: Fog;
    @group(0) @binding(2) var<uniform> lights: Lights;
    @group(0) @binding(3) var shadow_maps: texture_depth_2d_array;
    @group(0) @binding(4) var shadow_sampler: sampler_comparison;
    @group(0) @binding(5) var shadow_out: texture_storage_3d<rgba16float, write>;
}
@if(MULTISAMPLED_DEPTH) {
    @group(0) @binding(0) var depth_texture: texture_depth_multisampled_2d;
    @group(0) @binding(1) var<uniform> view: View;
    @group(0) @binding(2) var<uniform> fog: Fog;
    @group(0) @binding(3) var shadow_volume: texture_3d<f32>;
    @group(0) @binding(4) var volume_sampler: sampler;
}
@if(!MULTISAMPLED_DEPTH && !SHADOW_VOLUME) {
    @group(0) @binding(0) var scene: texture_2d<f32>;
    @group(0) @binding(1) var depth_texture: texture_depth_2d;
    @group(0) @binding(2) var<uniform> view: View;
    @group(0) @binding(3) var<uniform> fog: Fog;
    @group(0) @binding(4) var shadow_volume: texture_3d<f32>;
    @group(0) @binding(5) var volume_sampler: sampler;
}

// First shadow-casting directional light: the sun whose cascades DirLightShadow reads.
@if(SHADOW_VOLUME)
fn shadow_light() -> u32 {
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        if (lights.directional_lights[i].flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u { return i; }
    }
    return 0xffffffffu;
}
// One native `sample_c_lz` tap: the cascade is the count of cut points nearer than the tap's view
// depth (vCascadeParam.xyz), with no shader bias. Taps past the last cascade or outside its
// projection are lit, like our sun receivers; the native border behaviour there was not traced.
@if(SHADOW_VOLUME)
fn shadow_tap(light: u32, position: vec3<f32>, view_depth: f32) -> f32 {
    let sun = &lights.directional_lights[light];
    var cascade = 0u;
    loop {
        if cascade >= (*sun).num_cascades { return 1.0; }
        if view_depth < (*sun).cascades[cascade].far_bound { break; }
        cascade += 1u;
    }
    let clip = (*sun).cascades[cascade].clip_from_world*vec4(position,1.0);
    if clip.w <= 0.0 { return 1.0; }
    let ndc = clip.xyz/clip.w;
    if any(ndc.xy < vec2(-1.0)) || ndc.z < 0.0 || any(ndc > vec3(1.0)) { return 1.0; }
    let uv = ndc.xy*vec2(0.5,-0.5) + vec2(0.5);
    return textureSampleCompareLevel(shadow_maps,shadow_sampler,uv,
        i32((*sun).depth_texture_base_index + cascade),ndc.z);
}
// DirLightShadow: the mean of ShadowSampleCount taps across the cell's view depth,
// tap i at (i + 0.5 + (dither - 0.5)*ShadowDither)/count of the way from z0 to z1.
@if(SHADOW_VOLUME)
fn cell_shadow(light: u32, ray: vec3<f32>, ray_cos: f32, z0: f32, z1: f32, dither: f32) -> f32 {
    let count = u32(fog.shadow.x);
    if light == 0xffffffffu || count == 0u { return 1.0; }
    let jitter = (dither - 0.5)*fog.shadow.y + 0.5;
    var sum = 0.0;
    for (var i = 0u; i < count; i += 1u) {
        let z = mix(z0,z1,(f32(i) + jitter)/f32(count));
        sum += shadow_tap(light,view.world_position + ray*(z/ray_cos),z);
    }
    return sum/f32(count);
}
// Native g_DitherTex is a 4x4 tile addressed by cell xy & 3; its texels were not exported, so an
// ordered 4x4 Bayer tile stands in.
fn bayer4(pixel: vec2<i32>) -> f32 {
    let p = vec2<u32>(pixel) & vec2(3u);
    let index = array<u32,16>(0u,8u,2u,10u,12u,4u,14u,6u,3u,11u,1u,9u,15u,7u,13u,5u);
    return (f32(index[p.y*4u + p.x]) + 0.5)/16.0;
}
// View depth of geometric slice boundary i, the verified 39fa70 grid the fragment integrates.
fn slice_boundary(i: u32) -> f32 {
    let offset = fog.slices.x;
    let base = max(fog.depths.x+offset, 0.000001);
    return base*pow((fog.depths.y+offset)/base,f32(i)/fog.slices.y)-offset;
}
// One thread per cell, on the cell-centre ray, spanning the whole slice like the native texel.
@if(SHADOW_VOLUME)
@compute @workgroup_size(8,8,1)
fn shadow_volume_cells(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(shadow_out);
    if any(id >= size) { return; }
    let uv = (vec2<f32>(id.xy) + 0.5)/vec2<f32>(size.xy);
    let near_point = view.world_from_clip*vec4(uv.x*2.0-1.0,1.0-uv.y*2.0,1.0,1.0);
    let ray = normalize(near_point.xyz/near_point.w - view.world_position);
    let ray_cos = max(dot(ray,-view.world_from_view[2].xyz),0.0001);
    let shadow = cell_shadow(shadow_light(),ray,ray_cos,slice_boundary(id.z),slice_boundary(id.z+1u),
        bayer4(vec2<i32>(id.xy)));
    textureStore(shadow_out,id,vec4(shadow,0.0,0.0,1.0));
}

// Intersect the live authored dome's polygon-ring profile, in camera-centred world space.
// Native SkyDome forward permutations use this finite geometry depth for fog continuation.
fn background_distance(ray: vec3<f32>, ray_cos: f32) -> f32 {
    let count = u32(fog.background_control.x);
    if count < 2u { return fog.depths.y/ray_cos; }
    let angle = atan2(ray.z,ray.x);
    let step = fog.background_control.y;
    let sector = floor((angle-fog.background_control.z)/step);
    let normal_angle = fog.background_control.z+(sector+0.5)*step;
    let radial = length(ray.xz)*cos(angle-normal_angle);
    var result = 1.0e30;
    var found = false;
    for (var i=1u; i<count; i+=1u) {
        let a = fog.background[i-1u].xy;
        let b = fog.background[i].xy;
        let delta = b-a;
        let denominator = radial*delta.y-ray.y*delta.x;
        if abs(denominator) < 0.000001 { continue; }
        let distance = (a.x*delta.y-a.y*delta.x)/denominator;
        if distance <= 0.0 { continue; }
        let point = vec2(radial,ray.y)*distance;
        let along = dot(point-a,delta)/dot(delta,delta);
        if along >= -0.00001 && along <= 1.00001 {
            result = min(result,distance);
            found = true;
        }
    }
    // Clear pixels outside an exported dome have no finite surface endpoint.
    // Use the terminal authored volume there; do not invent a continuation distance.
    return select(fog.depths.y/ray_cos,result,found);
}

fn profile(q: f32) -> f32 {
    return select(2.0 - exp(min(q, 0.0)), exp(-max(q, 0.0)), q >= 0.0);
}
fn primitive(h: f32, k: f32) -> f32 {
    if h >= 0.0 { return -exp(-k*h)/k; }
    return 2.0*h - exp(k*h)/k;
}
fn column(h: f32, k: f32) -> f32 {
    if h > 0.0 { return exp(-k*h)/k; }
    return exp(k*h)/k - 2.0*h;
}
fn optical_segment(h0: f32, h1: f32, length: f32) -> f32 {
    let k = fog.density.y;
    var height_integral = profile(k*h0)*length;
    if abs(k*(h1-h0)) >= 0.0001 {
        height_integral = abs((primitive(h1,k)-primitive(h0,k))/(h1-h0))*length;
    }
    return max(0.0, fog.density.x*length + fog.density.z*height_integral);
}
// `shadow` scales only the DirectColor/OppositeColor term, as in WriteInScatteringShadowed;
// sky light stays unshadowed.
fn source_at(h: f32, z0: f32, z1: f32, ray: vec3<f32>, shadow: f32) -> vec3<f32> {
    let rho = fog.density.x + fog.density.z*profile(fog.density.y*h);
    let angular = clamp(0.5 + 0.5*dot(ray,fog.direction.xyz),0.0,1.0);
    let cd = mix(fog.opposite.xyz,fog.direct.xyz,angular);
    let optical_column = fog.density.z*column(h,fog.density.y);
    let sky_gate = clamp((z1-fog.depths.z)/(z1-z0),0.0,1.0);
    let dir_gate = clamp((z1-fog.depths.w)/(z1-z0),0.0,1.0)*shadow;
    return rho*(fog.sky.xyz*exp(-optical_column*fog.sky.w)*sky_gate
        + cd*exp(-optical_column*fog.direction.w*fog.direct.w*fog.extinction.xyz)*dir_gate)*fog.radiance_scale.x;
}
fn terminal_source(h: f32, ray: vec3<f32>) -> vec3<f32> {
    let angular = clamp(0.5 + 0.5*dot(ray,fog.direction.xyz),0.0,1.0);
    let cd = mix(fog.opposite.xyz,fog.direct.xyz,angular);
    let optical_column = fog.density.z*column(h,fog.density.y);
    // SolveVolumetricFog's final padding slice stores source colour, not integrated light.
    return (fog.sky.xyz*exp(-optical_column*fog.sky.w)
        + cd*exp(-optical_column*fog.direction.w*fog.direct.w*fog.extinction.xyz))*fog.radiance_scale.x;
}

@if(!SHADOW_VOLUME)
@fragment
fn fragment(in: FullscreenVertexOutput,
@if(MULTISAMPLED_DEPTH)
    @builtin(sample_index) sample: u32,
) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
@if(MULTISAMPLED_DEPTH)
    let depth = textureLoad(depth_texture,pixel,i32(sample));
    @if(!MULTISAMPLED_DEPTH) {
        let surface = textureLoad(scene,pixel,0);
        let depth = textureLoad(depth_texture,pixel,0);
    }
    let ndc = vec2(in.uv.x*2.0-1.0,1.0-in.uv.y*2.0);
    let reconstructed = view.world_from_clip*vec4(ndc,max(depth,0.000001),1.0);
    let relative = reconstructed.xyz/reconstructed.w - view.world_position;
    let ray = normalize(relative);
    // Native volume integration uses view-depth thickness; height samples use world space.
    let forward = -view.world_from_view[2].xyz;
    let ray_cos = max(dot(ray,forward),0.0001);
    var distance = length(relative);
    if depth <= 0.0 { distance = background_distance(ray,ray_cos); }
    let end_depth = distance*ray_cos;
    let cap = min(end_depth,fog.depths.y);
    let near = min(fog.depths.x,cap);
    // Verified 39fa70 geometric grid, independent of the surface depth.
    let offset = fog.slices.x;
    let base = max(fog.depths.x+offset, 0.000001);
    let ratio = (fog.depths.y+offset)/base;
    let count = u32(fog.slices.y);
    var tau = 0.0;
    var light = vec3(0.0);
    var z0 = near;
    // Inferred ray integration replacement for the native padded volume texture.
    for (var i = 0u; i < count; i += 1u) {
        let boundary = base*pow(ratio,f32(i+1u)/f32(count))-offset;
        let z1 = min(cap,boundary);
        if z1 <= z0 { break; }
        let ds = z1-z0;
        let sample_depth = clamp(base*pow(ratio,(f32(i)+fog.slices.z)/f32(count))-offset,z0,z1);
        let h = fog.extinction.w*(view.world_position.y + ray.y*sample_depth/ray_cos) + fog.density.w;
        let rho = fog.density.x + fog.density.z*profile(fog.density.y*h);
        let dt = rho*ds;
        // PreShadowed: the cell's shadow mean, filtered across cells in xy only.
        let shadow = textureSampleLevel(shadow_volume,volume_sampler,
            vec3(in.uv,(f32(i)+0.5)/f32(count)),0.0).x;
        light += source_at(h,z0,z1,ray,shadow)*ds*exp(-fog.extinction.xyz*(tau+0.5*dt));
        tau += dt;
        z0 = z1;
    }
    if end_depth > fog.depths.y && fog.slices.w > 0.5 {
        let start_length = fog.depths.y/ray_cos;
        let h0 = fog.extinction.w*(view.world_position.y + ray.y*start_length) + fog.density.w;
        let h1 = fog.extinction.w*(view.world_position.y + ray.y*distance) + fog.density.w;
        let terminal_light = terminal_source(h0,ray);
        // The analytical source padding slice has alpha zero, so (1 - padding alpha) is one.
        let delta = optical_segment(h0,h1,distance-start_length);
        // Zero extinction channels use the continuous delta limit, not a division by zero.
        let e = max(fog.extinction.xyz,vec3(0.000001));
        let integral = select((vec3(1.0)-exp(-e*delta))/e,vec3(delta),fog.extinction.xyz < vec3(0.000001));
        light += exp(-fog.extinction.xyz*tau)*terminal_light*integral;
        tau += delta;
    }
    let transmission = exp(-fog.extinction.xyz*tau);
    // Multiply the existing sample colour using destination * source blending.
@if(MULTISAMPLED_DEPTH && FOG_ATTENUATION)
    return vec4(transmission,1.0);
    // Add inscattering to that same sample before the render attachment resolves.
@if(MULTISAMPLED_DEPTH && !FOG_ATTENUATION)
    return vec4(light,0.0);
@if(!MULTISAMPLED_DEPTH)
    return vec4(surface.rgb*transmission+light,surface.a);
}
