//! Validate the authored billboard shader stage interfaces and bindings.
use bevy::shader::ShaderDefVal;

#[test]
fn billboard_shader_composes_and_validates() {
    let (mut cache, mut assets) = super::shader_support::cache();
    for (name, source) in [
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
    let production = include_str!("../../src/fxr_multi.wesl");
    let alpha_op = production.split("fn alpha_op(").nth(1).unwrap();
    let alpha_op = alpha_op.split("@fragment").next().unwrap();
    let source = format!(
        r#"
fn alpha_op({alpha_op}
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
    let values = super::shader_support::run_compute(source, 7);
    // mode 0: a x (1 + la (lum - 1)); 1: a + la lum; 2: overlay of a and lum, lerped by la.
    for (i, expected) in [0.08, 0.0008, 0.0, 0.5, 1.0, 0.24, 0.67]
        .into_iter()
        .enumerate()
    {
        let actual = values[i];
        assert!(
            (actual - expected).abs() < 0.000002,
            "sample {i}: {actual} vs {expected}"
        );
    }
}
