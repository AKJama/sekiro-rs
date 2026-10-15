use bevy::{
    asset::Assets,
    render::render_resource::{DownlevelFlags, WgpuFeatures},
    shader::{
        Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader,
    },
};
use std::path::{Path, PathBuf};

fn accept_module(_: &(), _: ShaderCacheSource, _: &ValidateShader) -> Result<(), ShaderCacheError> {
    Ok(())
}

fn shader_files(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            shader_files(&path, out);
        } else if path.extension().is_some_and(|s| s == "wgsl") {
            out.push(path);
        }
    }
}

#[test]
fn layered_map_forward_depth_and_motion_shaders_validate() {
    // Use the exact dependency sources Cargo installed, including Bevy's real PBR interfaces.
    let cargo = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .unwrap(),
            )
            .join(".cargo")
        });
    let mut files = Vec::new();
    for registry in std::fs::read_dir(cargo.join("registry/src")).unwrap() {
        for package in std::fs::read_dir(registry.unwrap().path()).unwrap() {
            let package = package.unwrap().path();
            let name = package.file_name().unwrap().to_string_lossy();
            if name.starts_with("bevy_") && name.ends_with("-0.19.1") {
                shader_files(&package.join("src"), &mut files);
            }
        }
    }
    let mut cache = ShaderCache::new(
        (),
        WgpuFeatures::all(),
        DownlevelFlags::all(),
        accept_module,
    );
    let capabilities = cache.composer.capabilities;
    cache.composer = Default::default();
    cache.composer = cache.composer.with_capabilities(capabilities);
    let mut assets = Assets::<Shader>::default();
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        if !source.contains("#define_import_path") {
            continue;
        }
        let shader = Shader::from_wgsl(source, path.to_string_lossy());
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
    }
    let coverage = Shader::from_wgsl(
        include_str!("../../src/map_coverage.wgsl"),
        "map_coverage.wgsl",
    );
    let id = assets.add(coverage.clone()).id();
    cache.set_shader(id, coverage);
    let bindings = Shader::from_wgsl(
        include_str!("../../src/map_bindings.wgsl"),
        "map_bindings.wgsl",
    );
    let id = assets.add(bindings.clone()).id();
    cache.set_shader(id, bindings);
    for (name, source, prepass) in [
        (
            "map_material",
            include_str!("../../src/map_material.wgsl"),
            false,
        ),
        (
            "map_prepass",
            include_str!("../../src/map_prepass.wgsl"),
            true,
        ),
        ("map_sweep", include_str!("../../src/map_sweep.wgsl"), false),
        (
            "map_fog_surface",
            include_str!("../../src/map_fog_surface.wgsl"),
            false,
        ),
    ] {
        let shader = Shader::from_wgsl(source, format!("{name}.wgsl"));
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
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
                ("MAX_DIRECTIONAL_LIGHTS", 10),
                ("MAX_CASCADES_PER_LIGHT", 4),
                ("MAX_RECT_LIGHTS", 64),
                ("AVAILABLE_STORAGE_BUFFER_BINDINGS", 8),
                ("MSAA_SAMPLES", 1),
            ] {
                defs.push(ShaderDefVal::UInt(key.into(), n));
            }
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
                        ("MAX_DIRECTIONAL_LIGHTS", 10),
                        ("MAX_CASCADES_PER_LIGHT", 4),
                        ("MAX_RECT_LIGHTS", 64),
                        ("AVAILABLE_STORAGE_BUFFER_BINDINGS", 8),
                        ("MSAA_SAMPLES", msaa),
                    ] {
                        defs.push(ShaderDefVal::UInt(key.into(), n));
                    }
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
