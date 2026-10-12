//! Validate every native post permutation through Bevy's actual shader composer.
use bevy::asset::Assets;
use bevy::render::render_resource::{DownlevelFlags, WgpuFeatures};
use bevy::shader::{Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ValidateShader};

fn accept_module(_: &(), _: ShaderCacheSource, _: &ValidateShader) -> Result<(), ShaderCacheError> {
    Ok(())
}

#[test]
fn native_post_shaders_compose_and_validate() {
    let mut cache = ShaderCache::new(
        (),
        WgpuFeatures::empty(),
        DownlevelFlags::all(),
        accept_module,
    );
    // Release builds normally skip composer validation. Tests must retain it.
    cache.composer = Default::default();
    let mut assets = Assets::<Shader>::default();
    let fullscreen = Shader::from_wgsl(
        "#define_import_path bevy_core_pipeline::fullscreen_vertex_shader\nstruct FullscreenVertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }",
        "fullscreen_test.wgsl",
    );
    let id = assets.add(fullscreen.clone()).id();
    cache.set_shader(id, fullscreen);
    let view = Shader::from_wgsl(
        "#define_import_path bevy_render::view\nstruct View { world_from_clip: mat4x4<f32>, world_from_view: mat4x4<f32>, world_position: vec3<f32> }",
        "view_test.wgsl",
    );
    let id = assets.add(view.clone()).id();
    cache.set_shader(id, view);
    for (name, source, variants) in [
        (
            "exposure",
            include_str!("../../src/exposure.wgsl"),
            vec![vec![]],
        ),
        (
            "fog",
            include_str!("../../src/fog.wgsl"),
            vec![
                vec![],
                vec!["MULTISAMPLED_DEPTH"],
                vec!["MULTISAMPLED_DEPTH", "FOG_ATTENUATION"],
            ],
        ),
        (
            "tonemap",
            include_str!("../../src/tonemap.wgsl"),
            vec![vec!["BRIGHT"], vec!["BLUR_H"], vec!["BLUR_V"], vec!["TONE"]],
        ),
        (
            "grading",
            include_str!("../../src/grading_lut.wgsl"),
            vec![vec![]],
        ),
    ] {
        let shader = Shader::from_wgsl(source, format!("{name}.wgsl"));
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
        for variant in variants {
            let defs: Vec<_> = variant.iter().copied().map(Into::into).collect();
            cache
                .get(0, id, &defs)
                .unwrap_or_else(|e| panic!("{name} {variant:?}: {e:?}"));
        }
    }
}
