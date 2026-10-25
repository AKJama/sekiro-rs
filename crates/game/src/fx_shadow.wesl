import bevy_pbr::{
    render::mesh_view_bindings::{lights, view},
    render::mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    render::shadows::{get_cascade_index, world_to_directional_light_local},
    render::shadow_sampling::sample_shadow_map_hardware,
};

// The shadowed sun on lit effect particles (#366, fx_lighting.rs `sun_shade`). The game's lit
// effect pixel shaders scale the sun term by 1 + g_ps_BillboardShadowIntensity (shadow - 1).
// The vertex colour arrives with the sun already summed into the light, so `shade` carries, per
// channel, shadowDarkness x sun / light: the share of the colour to shadow (0 = none).

// The factor on the lit colour: sun (1 + d (shadow - 1)) + rest = light (1 + shade (shadow - 1)).
fn shadow_mix(shade: vec3<f32>, shadow: f32) -> vec3<f32> {
    return vec3(1.0) + shade * (shadow - 1.0);
}

// The sun's shadow at a particle: one hardware compare tap in the cascade Bevy's map shader
// would pick, with its normal and depth bias, but no PCF and no cascade blend. The game reads
// one sample of its coarse fog sun-shadow volume (g_shadowVolumeTexture); the effect pass here
// has no fog volume bound. The normal only sets the bias: a billboard faces the camera.
fn sun_shadow(world: vec4<f32>) -> f32 {
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        let light = &lights.directional_lights[i];
        if ((*light).flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u {
            continue;
        }
        let cascade_index = get_cascade_index(i, (view.view_from_world * world).z);
        if cascade_index >= (*light).num_cascades {
            return 1.0;
        }
        let normal = normalize(view.world_position - world.xyz);
        let offset = (*light).shadow_normal_bias * (*light).cascades[cascade_index].texel_size
            * normal + (*light).shadow_depth_bias * (*light).direction_to_light;
        let local = world_to_directional_light_local(
            i,
            cascade_index,
            vec4(world.xyz + offset, world.w),
        );
        if local.w == 0.0 {
            return 1.0;
        }
        let layer = i32((*light).depth_texture_base_index + cascade_index);
        return sample_shadow_map_hardware(local.xy, local.z, layer);
    }
    return 1.0;
}

// The factor on a lit particle fragment's colour; unlit or unshadowed particles skip the lookup.
fn sun_shade_factor(shade: vec3<f32>, world: vec4<f32>) -> vec3<f32> {
    if all(shade <= vec3(0.0)) {
        return vec3(1.0);
    }
    return shadow_mix(shade, sun_shadow(world));
}
