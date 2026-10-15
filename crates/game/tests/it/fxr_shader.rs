//! Validate the authored billboard shader's stage interfaces and bindings.
use bevy::asset::Assets;
use bevy::render::render_resource::{DownlevelFlags, WgpuFeatures};
use bevy::shader::{
    Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader,
};

fn accept(_: &(), _: ShaderCacheSource, _: &ValidateShader) -> Result<(), ShaderCacheError> {
    Ok(())
}

#[test]
fn billboard_shader_composes_and_validates() {
    let mut cache = ShaderCache::new((), WgpuFeatures::empty(), DownlevelFlags::all(), accept);
    cache.composer = Default::default();
    let mut assets = Assets::<Shader>::default();
    for (name, source) in [
        (
            "interface",
            "#define_import_path bevy_pbr::forward_io\nstruct Vertex { @builtin(instance_index) instance_index:u32, @location(0) position:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) uv:vec2<f32>, @location(4) tangent:vec4<f32>, @location(5) color:vec4<f32> }\nstruct VertexOutput { @builtin(position) position:vec4<f32>, @location(0) world_position:vec4<f32>, @location(1) world_normal:vec3<f32>, @location(2) uv:vec2<f32>, @location(4) world_tangent:vec4<f32>, @location(5) color:vec4<f32>, @location(6) @interpolate(flat) instance_index:u32 }",
        ),
        (
            "view",
            "#define_import_path bevy_pbr::mesh_view_bindings\nstruct View { clip_from_world:mat4x4<f32>, exposure:f32 }\n@group(0) @binding(0) var<uniform> view:View;",
        ),
        (
            "transforms",
            "#define_import_path bevy_pbr::view_transformations
fn depth_ndc_to_view_z(z: f32) -> f32 { return z; }
fn position_world_to_clip(p: vec3<f32>) -> vec4<f32> { return vec4(p, 1.0); }",
        ),
        (
            "mesh_functions",
            "#define_import_path bevy_pbr::mesh_functions
fn get_world_from_local(i: u32) -> mat4x4<f32> { return mat4x4<f32>(); }",
        ),
        (
            "prepass_utils",
            "#define_import_path bevy_pbr::prepass_utils
#ifdef DEPTH_PREPASS
fn prepass_depth(frag_coord: vec4<f32>, sample_index: u32) -> f32 { return frag_coord.z + f32(sample_index); }
#endif",
        ),
        ("fxr", include_str!("../../src/fxr.wgsl")),
        ("fxr_multi", include_str!("../../src/fxr_multi.wgsl")),
    ] {
        let shader = Shader::from_wgsl(source, format!("{name}.wgsl"));
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
        if name == "fxr" || name == "fxr_multi" {
            for (index, (additive, prepass)) in
                [(false, false), (true, false), (false, true), (true, true)]
                    .into_iter()
                    .enumerate()
            {
                let mut definitions = vec![
                    ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), 3),
                    "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
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
