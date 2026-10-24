//! Validate every native post permutation through Bevy WESL and Naga.

#[test]
fn native_post_shaders_compose_and_validate() {
    let (mut cache, mut assets) = super::shader_support::cache();
    for (name, source, variants) in [
        (
            "exposure",
            include_str!("../../src/exposure.wesl"),
            vec![vec![]],
        ),
        (
            "fog",
            include_str!("../../src/fog.wesl"),
            vec![
                vec![],
                vec!["MULTISAMPLED_DEPTH"],
                vec!["MULTISAMPLED_DEPTH", "FOG_ATTENUATION"],
                vec!["SHADOW_VOLUME"],
            ],
        ),
        (
            "tonemap",
            include_str!("../../src/tonemap.wesl"),
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
            include_str!("../../src/grading_lut.wesl"),
            vec![vec![]],
        ),
    ] {
        let id = super::shader_support::register(&mut cache, &mut assets, name, source);
        for variant in variants {
            let mut defs: Vec<bevy::shader::ShaderDefVal> =
                variant.iter().copied().map(Into::into).collect();
            // Bevy's MeshRenderPlugin gives mesh_view_types these; fog reads its Lights (#345).
            for (key, n) in [
                ("MAX_DIRECTIONAL_LIGHTS", bevy::pbr::MAX_DIRECTIONAL_LIGHTS),
                ("MAX_CASCADES_PER_LIGHT", bevy::pbr::MAX_CASCADES_PER_LIGHT),
                ("MAX_RECT_LIGHTS", bevy::pbr::MAX_RECT_LIGHTS),
            ] {
                defs.push(bevy::shader::ShaderDefVal::UInt(key.into(), n as u32));
            }
            cache
                .get(0, id, &defs)
                .unwrap_or_else(|e| panic!("{name} {variant:?}: {e:?}"));
        }
    }
}

/// Runs a one-thread compute shader that writes `count` vec4 results and reads them back.
fn run_compute(source: String, count: usize) -> Vec<[f32; 4]> {
    use std::borrow::Cow;
    use wgpu::util::DeviceExt;

    let instance = wgpu::Instance::default();
    let adapter = bevy::tasks::block_on(instance.request_adapter(&Default::default()))
        .expect("shader regression needs a GPU adapter");
    let (device, queue) = bevy::tasks::block_on(adapter.request_device(&Default::default()))
        .expect("shader regression device");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader regression"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("shader regression"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let size = (count * 16) as u64;
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &vec![0; count * 16],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
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
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
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
    let bytes = readback.slice(..).get_mapped_range().unwrap();
    let (texels, _) = bytes.as_chunks::<16>();
    texels
        .iter()
        .map(|v| {
            std::array::from_fn(|i| f32::from_le_bytes(v[i * 4..i * 4 + 4].try_into().unwrap()))
        })
        .collect()
}

/// WriteInScatteringShadowed scales only the sun (DirectColor/OppositeColor) in-scatter by the
/// cell's shadow mean: a fully shadowed cell keeps the sky light and loses all sun light (#345).
#[test]
fn shadowed_fog_cell_keeps_sky_light_and_loses_sun_light() {
    let production = include_str!("../../src/fog.wesl").replace("\r\n", "\n");
    let fog_struct = production.split("struct Fog {").nth(1).unwrap();
    let fog_struct = fog_struct.split("\n}\n").next().unwrap();
    let functions = production.split("fn profile(").nth(1).unwrap();
    let functions = functions.split("fn terminal_source(").next().unwrap();
    let source_for = |sky: &str, direct: &str| {
        format!(
            r#"
struct Fog {{{fog_struct}
}}
const fog = Fog(
    vec4(0.002, 0.01, 0.01, 0.0), vec4(0.1, 400.0, 0.0, 0.0), vec4({sky}, 0.0),
    vec4({direct}, 0.0), vec4({direct}, 0.0), vec4(0.0, 1.0, 0.0, 1.0), vec4(1.0, 1.0, 1.0, 1.0),
    vec4(1.0, 32.0, 0.0, 0.0), vec4(0.0), array<vec4<f32>,16>(), vec4(1.0), vec4(8.0, 0.4, 0.0, 0.0),
);
fn profile({functions}
@group(0) @binding(0) var<storage, read_write> result: array<vec4<f32>, 3>;
@compute @workgroup_size(1)
fn main() {{
    let ray = normalize(vec3(0.3, 0.2, -1.0));
    for (var i = 0u; i < 3u; i++) {{
        result[i] = vec4(source_at(5.0, 20.0, 30.0, ray, f32(i)*0.5), 1.0);
    }}
}}
"#
        )
    };
    let lit = run_compute(source_for("0.2, 0.3, 0.4", "1.0, 0.6, 0.3"), 3);
    let sky_only = run_compute(source_for("0.2, 0.3, 0.4", "0.0, 0.0, 0.0"), 3);
    for channel in 0..3 {
        let sky = sky_only[0][channel];
        assert!(sky > 0.0, "sky light must reach the cell");
        // Fully shadowed: exactly the sky-only source.
        assert!(
            (lit[0][channel] - sky).abs() < 1e-7,
            "shadowed cell: {lit:?} vs {sky_only:?}"
        );
        // The sun term scales linearly with the shadow mean.
        let sun = lit[2][channel] - sky;
        assert!(sun > 0.0, "lit cell must gain sun light");
        assert!((lit[1][channel] - sky - 0.5 * sun).abs() < 1e-6 * sun.max(1.0));
        // Sky light itself is never shadowed.
        assert!((sky_only[2][channel] - sky).abs() < 1e-7);
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
    let production = include_str!("../../src/tonemap.wesl");
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
    let bytes = readback.slice(..).get_mapped_range().unwrap();
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
