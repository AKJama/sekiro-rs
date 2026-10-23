import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct Glow {
    // Smoothness, composite scale FCS * fade / sum s^i, level count, blur sigma.
    params: vec4<f32>,
    // Sky dome g_GlowScale.rgb * saturate(w).
    source: vec4<f32>,
}
@group(0) @binding(0) var a: texture_2d<f32>;
@group(0) @binding(1) var b: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;
@if(MULTISAMPLED_DEPTH)
    @group(0) @binding(3) var depth_texture: texture_depth_multisampled_2d;
@if(!MULTISAMPLED_DEPTH)
    @group(0) @binding(3) var depth_texture: texture_depth_2d;
@group(0) @binding(4) var<uniform> glow: Glow;

// The dome's glow target: its colour times g_GlowScale where the sky is seen (depth at
// infinity), premultiplied by alpha with NaNs zeroed as EmissiveToGlowPS does.
fn glow_source(pixel: vec2<i32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(depth_texture));
    let p = clamp(pixel, vec2(0), size - 1);
    if textureLoad(depth_texture, p, 0) > 0.0 { return vec3(0.0); }
    let c = textureLoad(a, p, 0);
    var g = max(c.rgb * c.a, vec3(0.0)) * glow.source.rgb;
    if any(g != g) { g = vec3(0.0); }
    return g;
}

// Stand-in for the untraced Sample13WeightforGlow: a standard 13-tap Gaussian.
fn blur(uv: vec2<f32>, axis: vec2<f32>) -> vec3<f32> {
    let texel = axis / vec2<f32>(textureDimensions(a));
    let s = glow.params.w;
    var sum = vec3(0.0);
    var total = 0.0;
    for (var i = -6; i <= 6; i++) {
        let w = exp(-f32(i * i) / (2.0 * s * s));
        sum += textureSampleLevel(a, linear_sampler, uv + texel * f32(i), 0.0).rgb * w;
        total += w;
    }
    return sum / total;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    @if(SOURCE) {
        // Level 0 is half the scene: average the 2x2 full-resolution glow texels.
        let p = vec2<i32>(in.position.xy) * 2;
        let g = glow_source(p) + glow_source(p + vec2(1, 0))
            + glow_source(p + vec2(0, 1)) + glow_source(p + vec2(1, 1));
        return vec4(g * 0.25, 1.0);
    }
    @if(DOWN) {
        return vec4(textureSampleLevel(a, linear_sampler, in.uv, 0.0).rgb, 1.0);
    }
    @if(BLUR_H) {
        return vec4(blur(in.uv, vec2(1.0, 0.0)), 1.0);
    }
    @if(BLUR_V) {
        return vec4(blur(in.uv, vec2(0.0, 1.0)), 1.0);
    }
    @if(UP) {
        // Blend constant (Smoothness, 1): this level plus s times the folded smaller ones.
        return vec4(textureSampleLevel(a, linear_sampler, in.uv, 0.0).rgb
            + glow.params.x * textureSampleLevel(b, linear_sampler, in.uv, 0.0).rgb, 1.0);
    }
    @if(COMPOSITE) {
        // GlowCompositeSinglePS: FinalColorScale * fade / sum s^i, alpha 1 (added ONE+ONE).
        return vec4(textureSampleLevel(a, linear_sampler, in.uv, 0.0).rgb * glow.params.y, 0.0);
    }
}
