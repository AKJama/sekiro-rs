//! Validate the authored billboard shader stage interfaces and bindings.
use bevy::shader::ShaderDefVal;

#[test]
fn billboard_shader_composes_and_validates() {
    let (mut cache, mut assets) = super::shader_support::cache();
    for (name, source) in [
        ("fx_shadow", include_str!("../../src/fx_shadow.wesl")),
        ("fxr", include_str!("../../src/fxr.wesl")),
        ("fxr_multi", include_str!("../../src/fxr_multi.wesl")),
    ] {
        let id = super::shader_support::register(&mut cache, &mut assets, name, source);
        if name == "fxr" || name == "fxr_multi" {
            for (index, (additive, prepass)) in
                [(false, false), (true, false), (false, true), (true, true)]
                    .into_iter()
                    .enumerate()
            {
                let mut definitions = vec![
                    ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), 3),
                    "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
                    "VERTEX_POSITIONS".into(),
                    "VERTEX_NORMALS".into(),
                    "VERTEX_UVS_A".into(),
                    "VERTEX_TANGENTS".into(),
                    "VERTEX_COLORS".into(),
                ];
                definitions.extend(super::shader_support::bevy_light_defs());
                if additive {
                    definitions.push("BLEND_PREMULTIPLIED_ALPHA".into());
                }
                if prepass {
                    // Single-sample captures read Bevy's depth prepass for the soft fade.
                    definitions.push("DEPTH_PREPASS".into());
                }
                cache.get(index, id, &definitions).unwrap();
            }
        }
    }
}

#[test]
fn multi_texture_alpha_ops_use_the_saturated_colour_average() {
    // GXFfxtessellateBlendSoftMultiTexture.ppo: lum = mul_sat((r + g + b), 1/3) of the layer
    // colour, not of its direction. A normalised colour stepped from 0.577 to 0 as a layer's
    // filtered colour reached zero, cutting texel-shaped holes into the 604 haze sheets (#365).
    use super::shader_support::{assert_close, run_compute, wgsl_fn};
    let production = include_str!("../../src/fxr_multi.wesl");
    let overlay = wgsl_fn(production, "fn overlay(", "fn color_op(");
    let alpha_op = wgsl_fn(production, "fn alpha_op(", "@fragment");
    let source = format!(
        r#"
{overlay}
{alpha_op}
@group(0) @binding(0) var<storage, read_write> result: array<f32, 7>;
@compute @workgroup_size(1)
fn main() {{
    result[0] = alpha_op(0.8, vec3(0.1), 1.0, 0u);
    result[1] = alpha_op(0.8, vec3(0.001), 1.0, 0u);
    result[2] = alpha_op(0.8, vec3(0.0), 1.0, 0u);
    result[3] = alpha_op(0.2, vec3(0.9, 0.6, 0.3), 0.5, 1u);
    result[4] = alpha_op(0.0, vec3(2.0), 1.0, 1u);
    result[5] = alpha_op(0.3, vec3(0.4), 1.0, 2u);
    result[6] = alpha_op(0.7, vec3(0.4), 0.5, 2u);
}}
"#
    );
    // mode 0: a x (1 + la (lum - 1)); 1: a + la lum; 2: overlay of a and lum, lerped by la.
    assert_close(
        &run_compute(source, 7),
        &[0.08, 0.0008, 0.0, 0.5, 1.0, 0.24, 0.67],
    );
}

#[test]
fn shadowed_sun_keeps_one_minus_shadow_darkness() {
    // #366: lit 603/604 particles take sun (1 + shadowDarkness (shadow - 1)) + rest; the vertex
    // `shade` is shadowDarkness x sun / light per channel.
    use super::shader_support::{assert_close, run_compute, wgsl_fn};
    let production = include_str!("../../src/fx_shadow.wesl");
    let factor = wgsl_fn(production, "fn sun_shadow_factor(", "fn sun_shadow(");
    let source = format!(
        r#"
{factor}
@group(0) @binding(0) var<storage, read_write> result: array<f32, 4>;
@compute @workgroup_size(1)
fn main() {{
    // All sun, shadowDarkness 0.9, full shadow: 10% of the sun is left.
    result[0] = sun_shadow_factor(vec3(0.9), 0.0).x;
    // shadowDarkness 0: unchanged.
    result[1] = sun_shadow_factor(vec3(0.0), 0.0).x;
    // Half sun at shadowDarkness 0.9, full shadow: 0.5 rest + 0.05 sun.
    result[2] = sun_shadow_factor(vec3(0.45), 0.0).x;
    // In the light: unchanged.
    result[3] = sun_shadow_factor(vec3(0.9), 1.0).x;
}}
"#
    );
    assert_close(&run_compute(source, 4), &[0.1, 1.0, 0.55, 1.0]);
}
