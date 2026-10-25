import bevy_pbr::{
    render::mesh_view_bindings::{lights, view},
    render::mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    render::shadows::fetch_directional_shadow,
};

// The shadowed sun on lit effect particles (#366, fx_lighting.rs `sun_shade`). The game's lit
// effect pixel shaders scale the sun term by 1 + g_ps_BillboardShadowIntensity (shadow - 1).
// The vertex colour arrives with the sun already summed into the light, so `shade` carries, per
// channel, shadowDarkness x sun / light: the share of the colour to shadow (0 = none).

// The factor on the lit colour: sun (1 + d (shadow - 1)) + rest = light (1 + shade (shadow - 1)).
fn sun_shadow_factor(shade: vec3<f32>, shadow: f32) -> vec3<f32> {
    return vec3(1.0) + shade * (shadow - 1.0);
}

// The sun's shadow at a particle. The game reads its fog sun-shadow volume
// (g_shadowVolumeTexture); the effect pass here has no fog volume bound, so it reads the sun's
// cascades as the map shader does. The normal only sets the bias: a billboard faces the camera.
fn sun_shadow(world: vec4<f32>, frag: vec2<f32>) -> f32 {
    let view_z = (view.view_from_world * world).z;
    let normal = normalize(view.world_position - world.xyz);
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        if (lights.directional_lights[i].flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            return fetch_directional_shadow(i, world, normal, view_z, frag);
        }
    }
    return 1.0;
}

// The factor for a particle fragment; unlit or unshadowed particles skip the shadow lookup.
fn sun_shade(shade: vec3<f32>, world: vec4<f32>, frag: vec2<f32>) -> vec3<f32> {
    if all(shade <= vec3(0.0)) {
        return vec3(1.0);
    }
    return sun_shadow_factor(shade, sun_shadow(world, frag));
}
