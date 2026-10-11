#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

struct Fog {
    density: vec4<f32>, depths: vec4<f32>, sky: vec4<f32>,
    direct: vec4<f32>, opposite: vec4<f32>, direction: vec4<f32>, extinction: vec4<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
#ifdef MULTISAMPLED_DEPTH
@group(0) @binding(1) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(1) var depth_texture: texture_depth_2d;
#endif
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> fog: Fog;

// Renderer quality and background distance choices, not recovered CPU uploads.
const SLICE_COUNT: u32 = 48u;
const SKY_DISTANCE_METRES: f32 = 20000.0;

fn profile(q: f32) -> f32 {
    return select(2.0 - exp(min(q, 0.0)), exp(-max(q, 0.0)), q >= 0.0);
}
fn primitive(h: f32, k: f32) -> f32 {
    if h >= 0.0 { return -exp(-k*h)/k; }
    return 2.0*h - exp(k*h)/k;
}
fn column(h: f32, k: f32) -> f32 {
    if k <= 0.000001 { return 0.0; }
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
fn source_at(h: f32, z0: f32, z1: f32, ray: vec3<f32>) -> vec3<f32> {
    let rho = fog.density.x + fog.density.z*profile(fog.density.y*h);
    let angular = clamp(0.5 + 0.5*dot(ray,fog.direction.xyz),0.0,1.0);
    let cd = mix(fog.opposite.xyz,fog.direct.xyz,angular);
    let optical_column = fog.density.z*column(h,fog.density.y);
    let sky_gate = clamp((z1-fog.depths.z)/(z1-z0),0.0,1.0);
    let dir_gate = clamp((z1-fog.depths.w)/(z1-z0),0.0,1.0);
    return rho*(fog.sky.xyz*exp(-optical_column*fog.sky.w)*sky_gate
        + cd*exp(-optical_column*fog.direction.w*fog.direct.w*fog.extinction.xyz)*dir_gate);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let surface = textureLoad(scene,pixel,0);
    var depth = textureLoad(depth_texture,pixel,0);
#ifdef MULTISAMPLED_DEPTH
    for (var sample = 1u; sample < textureNumSamples(depth_texture); sample += 1u) {
        depth = max(depth,textureLoad(depth_texture,pixel,i32(sample)));
    }
#endif
    let ndc = vec2(in.uv.x*2.0-1.0,1.0-in.uv.y*2.0);
    let reconstructed = view.world_from_clip*vec4(ndc,max(depth,0.000001),1.0);
    let relative = reconstructed.xyz/reconstructed.w - view.world_position;
    let ray = normalize(relative);
    // Integration slices use view depth; segment lengths use the complete ray.
    let forward = -view.world_from_view[2].xyz;
    let ray_cos = max(dot(ray,forward),0.0001);
    let distance = select(length(relative),SKY_DISTANCE_METRES,depth <= 0.0);
    let end_depth = distance*ray_cos;
    let cap = min(end_depth,fog.depths.y);
    let near = min(fog.depths.x,cap);
    let log_range = log(1.0+cap/near);
    var tau = 0.0;
    var light = vec3(0.0);
    var z0 = 0.0;
    // Logarithmic boundaries and deterministic midpoint integration avoid stochastic banding.
    for (var i = 0u; i < SLICE_COUNT; i += 1u) {
        let z1 = near*(exp(log_range*f32(i+1u)/f32(SLICE_COUNT))-1.0);
        let ds = (z1-z0)/ray_cos;
        let h = view.world_position.y + ray.y*(z0+z1)*0.5/ray_cos + fog.density.w;
        let rho = fog.density.x + fog.density.z*profile(fog.density.y*h);
        let dt = rho*ds;
        light += source_at(h,z0,z1,ray)*ds*exp(-fog.extinction.xyz*(tau+0.5*dt));
        tau += dt;
        z0 = z1;
    }
    if end_depth > fog.depths.y {
        let start_length = fog.depths.y/ray_cos;
        let h0 = view.world_position.y + ray.y*start_length + fog.density.w;
        let h1 = view.world_position.y + ray.y*distance + fog.density.w;
        let terminal_light = light;
        let delta = optical_segment(h0,h1,distance-start_length)*(1.0-exp(-tau));
        // Zero extinction channels use the continuous delta limit, not a division by zero.
        let e = max(fog.extinction.xyz,vec3(0.000001));
        let integral = select((vec3(1.0)-exp(-e*delta))/e,vec3(delta),fog.extinction.xyz < vec3(0.000001));
        light += exp(-fog.extinction.xyz*tau)*terminal_light*integral;
        tau += delta;
    }
    return vec4(surface.rgb*exp(-fog.extinction.xyz*tau)+light,surface.a);
}
