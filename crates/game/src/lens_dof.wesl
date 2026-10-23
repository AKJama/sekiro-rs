import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;

struct Dof {
    // FarStart, FarEnd, LinearCoCFactorFar, sky CoC weight.
    far: vec4<f32>,
    // Radius at CoC 1 as a fraction of the view height.
    blur: vec4<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@if(MULTISAMPLED_DEPTH)
    @group(0) @binding(2) var depth_texture: texture_depth_multisampled_2d;
@if(!MULTISAMPLED_DEPTH)
    @group(0) @binding(2) var depth_texture: texture_depth_2d;
@group(0) @binding(3) var<uniform> view: View;
@group(0) @binding(4) var<uniform> dof: Dof;

// LensSimu2_ComputeCoC linear far CoC; MakeBlurBuffer weights the sky's far CoC.
fn far_coc(pixel: vec2<i32>) -> f32 {
    let size = vec2<i32>(textureDimensions(depth_texture));
    let depth = textureLoad(depth_texture, clamp(pixel, vec2(0), size - 1), 0);
    if depth <= 0.0 { return dof.far.z*dof.far.w; }
    // Bevy's infinite reverse-Z projection: depth = near / view depth.
    let z = view.clip_from_view[3][2]/depth;
    return saturate(saturate((z - dof.far.x)/(dof.far.y - dof.far.x))*dof.far.z);
}

// LensSimu2_MakeBlurBuffer's two rings: 16 taps at the radius and 8 at half of it.
const RING: array<vec2<f32>,24> = array<vec2<f32>,24>(
    vec2(0.0,1.0), vec2(0.382683,0.923880), vec2(0.707107,0.707107), vec2(0.923880,0.382683),
    vec2(1.0,0.0), vec2(0.923880,-0.382683), vec2(0.707107,-0.707107), vec2(0.382683,-0.923880),
    vec2(0.0,-1.0), vec2(-0.382683,-0.923880), vec2(-0.707107,-0.707107), vec2(-0.923880,-0.382683),
    vec2(-1.0,0.0), vec2(-0.923880,0.382683), vec2(-0.707107,0.707107), vec2(-0.382683,0.923880),
    vec2(0.0,0.5), vec2(0.353553,0.353553), vec2(0.5,0.0), vec2(0.353553,-0.353553),
    vec2(0.0,-0.5), vec2(-0.353553,-0.353553), vec2(-0.5,0.0), vec2(-0.353553,0.353553),
);

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    let centre = textureLoad(scene, pixel, 0);
    let scale = dof.blur.x*view.viewport.w;
    let radius = far_coc(pixel)*scale;
    // MakeBlurBuffer keeps the unblurred sample below a quarter-pixel radius.
    if radius < 0.25 { return centre; }
    let texel = 1.0/vec2<f32>(textureDimensions(scene));
    var ring = RING;
    var sum = centre.rgb;
    var weight = 1.0;
    for (var i = 0u; i < 24u; i += 1u) {
        let offset = ring[i]*radius;
        let tap = in.position.xy + offset;
        // A sharper (nearer) tap only contributes as far as its own blur reaches, so in-focus
        // silhouettes do not bleed into the blurred distance behind them.
        let reach = far_coc(vec2<i32>(tap))*scale;
        let w = saturate(reach - length(offset) + 1.0);
        sum += textureSampleLevel(scene, scene_sampler, tap*texel, 0.0).rgb*w;
        weight += w;
    }
    return vec4(sum/weight, centre.a);
}
