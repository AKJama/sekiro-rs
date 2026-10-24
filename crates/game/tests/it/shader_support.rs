//! Compose production WESL with Cargo's Bevy sources, then validate the resulting WGSL.
use bevy::{
    asset::{AssetId, Assets},
    shader::{
        Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader,
    },
};
use std::path::{Path, PathBuf};

pub fn validate(
    _: &(),
    source: ShaderCacheSource,
    _: &ValidateShader,
) -> Result<(), ShaderCacheError> {
    let ShaderCacheSource::Wgsl(source) = source else {
        panic!("expected WGSL");
    };
    let module = wgpu::naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    Ok(())
}

fn load(path: &Path, root: &Path, package: &str, shaders: &mut Vec<Shader>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            load(&path, root, package, shaders);
        } else if path.extension().is_some_and(|s| s == "wesl") {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            shaders.push(Shader::from_wesl(
                std::fs::read_to_string(&path).unwrap(),
                format!("embedded://{package}/{relative}"),
            ));
        }
    }
}

/// Register a production module with the game's embedded asset path.
pub fn register(
    cache: &mut ShaderCache<(), ()>,
    assets: &mut Assets<Shader>,
    name: &str,
    source: &'static str,
) -> AssetId<Shader> {
    let shader = Shader::from_wesl(source, format!("embedded://sekiro_rs/{name}.wesl"));
    let id = assets.add(shader.clone()).id();
    cache.set_shader(id, shader);
    id
}

fn bevy_version() -> &'static str {
    include_str!("../../../../Cargo.lock")
        .split("[[package]]")
        .find(|package| package.lines().any(|line| line == "name = \"bevy\""))
        .and_then(|package| {
            package
                .lines()
                .find_map(|line| line.strip_prefix("version = \""))
        })
        .and_then(|version| version.strip_suffix('"'))
        .expect("locked Bevy version")
}

pub fn cache() -> (ShaderCache<(), ()>, Assets<Shader>) {
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
    let mut shaders = Vec::new();
    for registry in std::fs::read_dir(cargo.join("registry/src")).unwrap() {
        for package in std::fs::read_dir(registry.unwrap().path()).unwrap() {
            let package = package.unwrap().path();
            let name = package.file_name().unwrap().to_string_lossy();
            if let Some(name) = name
                .strip_suffix(&format!("-{}", bevy_version()))
                .filter(|s| s.starts_with("bevy_"))
            {
                let root = package.join("src");
                load(&root, &root, name, &mut shaders);
            }
        }
    }
    assert!(
        !shaders.is_empty(),
        "locked Bevy shader sources must be available"
    );
    let mut cache = ShaderCache::new((), validate);
    let mut assets = Assets::<Shader>::default();
    for shader in shaders {
        let id = assets.add(shader.clone()).id();
        cache.set_shader(id, shader);
    }
    (cache, assets)
}

/// The light-array sizes Bevy's MeshRenderPlugin gives `mesh_view_types`.
pub fn bevy_light_defs() -> Vec<ShaderDefVal> {
    [
        ("MAX_DIRECTIONAL_LIGHTS", bevy::pbr::MAX_DIRECTIONAL_LIGHTS),
        ("MAX_CASCADES_PER_LIGHT", bevy::pbr::MAX_CASCADES_PER_LIGHT),
        ("MAX_RECT_LIGHTS", bevy::pbr::MAX_RECT_LIGHTS),
    ]
    .into_iter()
    .map(|(key, n)| ShaderDefVal::UInt(key.into(), n as u32))
    .collect()
}

/// Runs a one-thread WGSL compute shader whose `main` writes `floats` f32 values to the
/// storage buffer at group 0 binding 0, and reads them back.
pub fn run_compute(source: String, floats: usize) -> Vec<f32> {
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
    let size = (floats * 4) as u64;
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &vec![0; floats * 4],
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
    let (values, _) = bytes.as_chunks::<4>();
    values.iter().map(|v| f32::from_le_bytes(*v)).collect()
}
