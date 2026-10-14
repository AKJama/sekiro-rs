#define_import_path sekiro_map_coverage
#import bevy_pbr::mesh_view_bindings::view

fn camera_coverage(position: vec2<f32>, world_position: vec3<f32>, fade: vec4<f32>) {
    if fade.x > 0.0 {
        let opacity = distance(world_position, view.world_position) * fade.z - fade.y;
        let p = vec2<u32>(position) & vec2(7u);
        let a = ((p.x & 1u) ^ (p.y & 1u)) * 2u + (p.y & 1u);
        let b = (((p.x >> 1u) & 1u) ^ ((p.y >> 1u) & 1u)) * 2u + ((p.y >> 1u) & 1u);
        let c = (((p.x >> 2u) & 1u) ^ ((p.y >> 2u) & 1u)) * 2u + ((p.y >> 2u) & 1u);
        let noise = (f32(a * 16u + b * 4u + c) + 0.5) / 64.0;
        if opacity < 0.01 + 0.99 * noise { discard; }
    }
}
