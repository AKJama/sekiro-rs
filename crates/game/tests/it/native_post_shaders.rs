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
            vec![
                vec!["BRIGHT"],
                vec!["BLUR_H"],
                vec!["BLUR_V"],
                vec!["TONE"],
                vec!["COPY"],
                vec!["UPSAMPLE"],
            ],
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

/// Execute the production curve against native mode-7 table samples, before output gamma.
/// DXBC `exp` is base two; a natural exponential passes shader validation but fails here.
#[test]
fn sensitometric_curve_matches_native_table_samples() {
    use std::borrow::Cow;
    use wgpu::util::DeviceExt;

    let instance = wgpu::Instance::default();
    let adapter = bevy::tasks::block_on(instance.request_adapter(&Default::default()))
        .expect("native tone regression needs a GPU adapter");
    let (device, queue) = bevy::tasks::block_on(adapter.request_device(&Default::default()))
        .expect("native tone regression device");
    let production = include_str!("../../src/tonemap.wgsl");
    let curve = production.split("fn tone(").nth(1).unwrap();
    let curve = curve.split("@fragment").next().unwrap();
    let source = format!(
        r#"
struct Settings {{ transfer: vec4<f32>, curve: vec4<f32>, reinhard: vec4<f32> }}
const settings = Settings(vec4(1.0, 1.0, 7.0, 4.0), vec4(1.0, 0.5, 0.5, 1.6), vec4(1.7, 0.0, 0.0, 0.0));
fn tone({curve}
@group(0) @binding(0) var<storage, read_write> result: array<vec4<f32>, 5>;
@compute @workgroup_size(1)
fn main() {{
    let inputs = array<f32, 5>(0.0, 0.01, 0.18, 1.0, 4.0);
    for (var i = 0u; i < 5u; i++) {{ result[i] = vec4(tone(vec3(inputs[i])), 1.0); }}
}}
"#
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("native sensitometric regression"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("native sensitometric regression"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[0; 80],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 80,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 80);
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let bytes = readback.slice(..).get_mapped_range();
    for (i, expected) in [0.0, 0.0049891566, 0.1211626855, 0.7508116073, 0.9908357271]
        .into_iter()
        .enumerate()
    {
        for channel in 0..3 {
            let offset = i * 16 + channel * 4;
            let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            assert!(
                (f64::from(actual) - expected).abs() < 0.000002,
                "native tone sample {i} channel {channel}: {actual} vs {expected}"
            );
        }
    }
}
