use bevy::shader::ShaderDefVal;

#[test]
fn snow_coverage_matches_native_height_and_wet_operations() {
    // Native Ground Gbuf 16/18 and Multiple Gbuf 20: fixed instruction-derived
    // samples guard the order of wet exclusion, height subtraction and saturation.
    let production = include_str!("../../src/map_material.wesl");
    let coverage = production.split("fn snow_coverage(").nth(1).unwrap();
    let coverage = coverage.split("// FLV:").next().unwrap();
    let normal = production.split("fn layer_world_normal(").nth(1).unwrap();
    let normal = normal.split("fn has(").next().unwrap();
    let source = format!(
        r#"
fn saturate(x: f32) -> f32 {{ return clamp(x, 0.0, 1.0); }}
fn snow_coverage({coverage}
fn layer_world_normal({normal}
@group(0) @binding(0) var<storage, read_write> result: array<f32, 6>;
@compute @workgroup_size(1)
fn main() {{
    let height = (dot(vec3(0.2, 0.6, 0.4), vec3(0.0, 0.5, 0.7)) + 0.01) * 0.5;
    result[0] = snow_coverage(0.85, 0.4, 0.0, height, 3.0);
    result[1] = snow_coverage(0.85, 0.4, 0.8, height, 3.0);
    result[2] = snow_coverage(0.85, 0.4, 1.25, height, 3.0);
    result[3] = snow_coverage(1.4, 1.0, 0.5, 0.2, 0.0);
    result[4] = snow_coverage(-0.2, 1.0, 1.5, 0.05, 0.0);
    let n = layer_world_normal(vec3(0.6, 0.8, 1.2), vec3(0.0, 0.8, 0.0), vec4(0.7, 0.0, 0.0, 1.0), true);
    result[5] = snow_coverage(n.y, 1.0, 0.0, 0.2, 0.0);
}}
"#
    );
    let values = super::shader_support::run_compute(source, 6);
    for (i, expected) in [0.5812, 0.0, 0.0, 0.64, 0.055, 0.7218655]
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

/// Foliage translucency follows the normal-alpha mask per pixel (#367): `k = clamp(float_0 *
/// mask, 0, 10)` times the A/B switch, and the sun/fill response takes that `k`.
#[test]
fn foliage_translucency_follows_the_mask_per_pixel() {
    let production = include_str!("../../src/map_material.wesl");
    let body = |name: &str| {
        let rest = production
            .split(&format!("fn {name}("))
            .nth(1)
            .unwrap_or_else(|| panic!("{name}"));
        format!("fn {name}({}\n}}", rest.split("\n}").next().unwrap())
    };
    let source = format!(
        r#"
{}
{}
@group(0) @binding(0) var<storage, read_write> result: array<f32, 10>;
@compute @workgroup_size(1)
fn main() {{
    // The gatehouse grass scale (MTD float_0 3) over four texels of its mask.
    result[0] = foliage_translucency(3.0, 0.0, 1.0);
    result[1] = foliage_translucency(3.0, 0.25, 1.0);
    result[2] = foliage_translucency(3.0, 0.73, 1.0);
    result[3] = foliage_translucency(3.0, 1.0, 1.0);
    // Saturates at 10 like the UNORM G-buffer, and the knob turns it off.
    result[4] = foliage_translucency(20.0, 1.0, 1.0);
    result[5] = foliage_translucency(3.0, 0.73, 0.0);
    // A blade lit from behind (N.L -0.5) seen against the light (L = -V): the response grows
    // with the texel's k, and k = 0 is plain saturate(N.L).
    for (var i = 0; i < 4; i++) {{
        result[6 + i] = translucent_diffuse(-0.5, 1.0, -1.0, result[i]);
    }}
}}
"#,
        body("foliage_translucency"),
        body("translucent_diffuse"),
    );
    let values = super::shader_support::run_compute(source, 10);
    let back = |k: f32| (k - 0.5f32).max(0.) / (1. + 2. * k) + k / (1. + k);
    let expected = [
        0.,
        0.75,
        2.19,
        3.,
        10.,
        0.,
        back(0.),
        back(0.75),
        back(2.19),
        back(3.),
    ];
    for (i, (actual, expected)) in values.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() < 1e-5,
            "sample {i}: {actual} vs {expected}"
        );
    }
    assert_eq!(values[6], 0.);
    assert!(values[7] < values[8] && values[8] < values[9], "{values:?}");
}

#[test]
fn layered_map_forward_depth_and_motion_shaders_validate() {
    let (mut cache, mut assets) = super::shader_support::cache();
    super::shader_support::register(
        &mut cache,
        &mut assets,
        "map_coverage",
        include_str!("../../src/map_coverage.wesl"),
    );
    super::shader_support::register(
        &mut cache,
        &mut assets,
        "map_bindings",
        include_str!("../../src/map_bindings.wesl"),
    );
    for (name, source, prepass) in [
        (
            "map_material",
            include_str!("../../src/map_material.wesl"),
            false,
        ),
        (
            "map_prepass",
            include_str!("../../src/map_prepass.wesl"),
            true,
        ),
        ("map_sweep", include_str!("../../src/map_sweep.wesl"), false),
        (
            "map_fog_surface",
            include_str!("../../src/map_fog_surface.wesl"),
            false,
        ),
    ] {
        let id = super::shader_support::register(&mut cache, &mut assets, name, source);
        if name == "map_sweep" {
            // Collision has positions/normals but no UVs, tangents or colour streams.
            let mut defs: Vec<ShaderDefVal> = [
                "VERTEX_NORMALS",
                "VERTEX_OUTPUT_INSTANCE_INDEX",
                "SHADOW_FILTER_METHOD_HARDWARE_2X2",
            ]
            .into_iter()
            .map(Into::into)
            .collect();
            for (key, n) in [
                ("MATERIAL_BIND_GROUP", 3),
                ("AVAILABLE_STORAGE_BUFFER_BINDINGS", 8),
                ("MSAA_SAMPLES", 1),
            ] {
                defs.push(ShaderDefVal::UInt(key.into(), n));
            }
            defs.extend(super::shader_support::bevy_light_defs());
            cache
                .get(0, id, &defs)
                .unwrap_or_else(|e| panic!("sweep collision shader: {e:?}"));
        }
        for bindless in [false, true] {
            for game in [false, true] {
                for msaa in [1, 4] {
                    let mut defs: Vec<ShaderDefVal> = [
                        "VERTEX_UVS",
                        "VERTEX_UVS_A",
                        "VERTEX_UVS_B",
                        "VERTEX_NORMALS",
                        "VERTEX_TANGENTS",
                        "VERTEX_COLORS",
                        "VERTEX_OUTPUT_INSTANCE_INDEX",
                        "CAMERA_FADE",
                        "SHADOW_FILTER_METHOD_HARDWARE_2X2",
                    ]
                    .into_iter()
                    .map(Into::into)
                    .collect();
                    for (key, n) in [
                        ("MATERIAL_BIND_GROUP", 3),
                        ("AVAILABLE_STORAGE_BUFFER_BINDINGS", 8),
                        ("MSAA_SAMPLES", msaa),
                    ] {
                        defs.push(ShaderDefVal::UInt(key.into(), n));
                    }
                    defs.extend(super::shader_support::bevy_light_defs());
                    if msaa > 1 {
                        defs.push("MULTISAMPLED".into());
                    }
                    if bindless {
                        defs.push("BINDLESS".into());
                    }
                    if game {
                        defs.push("GAME_MATERIAL".into());
                        if !prepass {
                            defs.push("GAME_LAYERS".into());
                        }
                    }
                    if prepass {
                        defs.extend(
                            [
                                "PREPASS_PIPELINE",
                                "PREPASS_FRAGMENT",
                                "MOTION_VECTOR_PREPASS",
                                "NORMAL_PREPASS",
                                "NORMAL_PREPASS_OR_DEFERRED_PREPASS",
                            ]
                            .into_iter()
                            .map(Into::into),
                        );
                    }
                    if name == "map_fog_surface" {
                        defs.push("DEPTH_PREPASS".into());
                        defs.push("ENVIRONMENT_MAP".into());
                    }
                    let native_variants: &[bool] = if name == "map_material" {
                        &[false, true]
                    } else {
                        &[false]
                    };
                    for native_direct in native_variants {
                        let mut variant = defs.clone();
                        if *native_direct {
                            variant.push("NATIVE_DIRECT".into());
                        }
                        cache.get(0, id, &variant).unwrap_or_else(|e| {
                            panic!("{name}: bindless={bindless}, game={game}, msaa={msaa}, native_direct={native_direct}: {e:?}")
                        });
                    }
                    if name == "map_fog_surface" {
                        defs.retain(|d| !matches!(d, ShaderDefVal::Bool(name, _) if name == "DEPTH_PREPASS"));
                        defs.push("FOG_DIAGNOSTIC".into());
                        cache
                            .get(0, id, &defs)
                            .unwrap_or_else(|e| panic!("fog surface sweep diagnostic: {e:?}"));
                    }
                }
            }
        }
    }
}
