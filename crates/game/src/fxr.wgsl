#import bevy_pbr::{forward_io::{Vertex, VertexOutput}, mesh_view_bindings::view}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var particle_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var particle_sampler: sampler;

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    // Batch corners are in world space; the entity transform is identity.
    out.position = view.clip_from_world * vec4(v.position, 1.0);
    out.world_position = vec4(v.position, 1.0);
    out.world_normal = v.normal;
    out.uv = v.uv;
    out.color = v.color;
    // This attribute carries alpha controls, not a surface tangent.
    out.world_tangent = v.tangent;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = v.instance_index;
#endif
    return out;
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    var texture_color = textureSample(particle_texture, particle_sampler, v.uv);
    let fade = clamp(v.world_tangent.x, 0.0, 1.0);
    texture_color.a = clamp((texture_color.a - fade) / max(1.0 - fade, 0.00001), 0.0, 1.0);
    if texture_color.a < v.world_tangent.y { discard; }
    let color = texture_color * v.color;
#ifdef BLEND_PREMULTIPLIED_ALPHA
    // Bevy routes AlphaMode::Add through premultiplied blending. Zero coverage keeps
    // the destination, while source alpha weights the emitted radiance.
    return vec4(color.rgb * view.exposure * clamp(color.a, 0.0, 1.0), 0.0);
#else
    return vec4(color.rgb * view.exposure, clamp(color.a, 0.0, 1.0));
#endif
}
