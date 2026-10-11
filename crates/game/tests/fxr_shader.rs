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
        ("fxr", include_str!("../src/fxr.wgsl")),
    ] {
        let shader = Shader::from_wgsl(source, format!("{name}.wgsl"));
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
        if name == "fxr" {
            for additive in [false, true] {
                let mut definitions = vec![
                    ShaderDefVal::UInt("MATERIAL_BIND_GROUP".into(), 3),
                    "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
                ];
                if additive {
                    definitions.push("BLEND_PREMULTIPLIED_ALPHA".into());
                }
                cache.get(usize::from(additive), id, &definitions).unwrap();
            }
        }
    }
}
