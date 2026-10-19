//! Resident GPU memory and asset accounting, sampled every five seconds under `--perf`.
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderSystems};
use std::collections::BTreeMap;
use std::time::Instant;

pub(crate) fn render_plugin() -> bevy::render::RenderPlugin {
    bevy::render::RenderPlugin {
        render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
            bevy::render::settings::WgpuSettings {
                // The default Vulkan allocator reserves 128-256 MiB blocks. Streamed
                // sources leave substantial slack in them even after their buffers drop.
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                ..default()
            },
        )),
        ..default()
    }
}

pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<Budget>() {
        return;
    }
    app.init_resource::<Budget>()
        .add_systems(Startup, choose_budget)
        .add_systems(Last, trim_textures);
    if crate::perf_log::enabled() {
        app.insert_resource(MemoryClock(Instant::now()))
            .add_systems(Last, cpu_assets);
    }
    if crate::perf_log::enabled()
        && let Some(render) = app.get_sub_app_mut(RenderApp)
    {
        render.add_systems(Render, gpu_assets.after(RenderSystems::Cleanup));
    }
}

const GIB: u64 = 1 << 30;

pub(crate) fn validate_args(args: &[String]) -> anyhow::Result<()> {
    for (flag, max) in [("--vram-budget", 1024.0), ("--perf-seconds", 86400.0)] {
        if args.iter().any(|a| a == flag) {
            let value = crate::arg_value(args, flag).and_then(|v| v.parse::<f64>().ok());
            anyhow::ensure!(
                value.is_some_and(|v| v.is_finite() && v > 0.0 && v <= max),
                "{flag} requires a positive number <= {max}"
            );
        }
    }
    Ok(())
}

#[derive(Resource, Default)]
struct Budget {
    bytes: u64,
    mips: u32,
    memory: Option<VideoMemory>,
    sampled: Option<Instant>,
}

impl Budget {
    fn decision(capacity: u64, driver: u64, requested: Option<f64>) -> (u64, u32) {
        let card = requested.map_or(capacity, |gb| ((gb * GIB as f64) as u64).min(capacity));
        let bytes = (card * 4 / 5).min(driver);
        // DXGI excludes a little reserved capacity on a marketed 16 GiB card.
        let mips = match card.div_ceil(GIB) {
            0..4 => 3,
            4..8 => 2,
            8..16 => 1,
            _ => 0,
        };
        (bytes, mips)
    }
}

fn choose_budget(adapter: Res<RenderAdapterInfo>, mut budget: ResMut<Budget>) {
    budget.memory = VideoMemory::open(&adapter);
    let sample = budget.memory.as_ref().and_then(VideoMemory::sample);
    let (capacity, _, driver) = sample.unwrap_or((10 * GIB, 0, 8 * GIB));
    let requested =
        crate::arg_value(crate::perf_log::args(), "--vram-budget").and_then(|v| v.parse().ok());
    (budget.bytes, budget.mips) = Budget::decision(capacity, driver, requested);
    info!(
        "VRAM budget: adapter={:?} dedicated_bytes={capacity} usable_bytes={} texture_mips_skipped={} override_GiB={requested:?} source={}",
        adapter.name,
        budget.bytes,
        budget.mips,
        if sample.is_some() {
            "DXGI"
        } else {
            "conservative fallback"
        }
    );
}

#[allow(clippy::too_many_arguments)]
fn trim_textures(
    mut budget: ResMut<Budget>,
    assets: Res<AssetServer>,
    mut events: MessageReader<bevy::asset::AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut layered: ResMut<Assets<crate::map::LayeredMaterial>>,
    mut levels: Local<std::collections::HashMap<AssetId<Image>, u32>>,
) {
    let mut pressure = false;
    if budget.sampled.is_none_or(|t| t.elapsed().as_secs() >= 5) {
        budget.sampled = Some(Instant::now());
        if let Some((_, usage, driver)) = budget.memory.as_ref().and_then(VideoMemory::sample)
            && usage > budget.bytes.min(driver)
            && budget.mips < 3
        {
            budget.mips += 1;
            pressure = true;
            warn!(
                "VRAM pressure: usage_bytes={usage}, skipping {} top texture mips",
                budget.mips
            );
        }
    }
    let mut ids: Vec<_> = events
        .read()
        .filter_map(|event| match event {
            bevy::asset::AssetEvent::Added { id } => Some(*id),
            bevy::asset::AssetEvent::Unused { id } => {
                levels.remove(id);
                None
            }
            _ => None,
        })
        .collect();
    if pressure {
        ids.extend(images.ids());
    }
    ids.sort_unstable();
    ids.dedup();
    let mut changed = false;
    for id in ids {
        let done = levels.get(&id).copied().unwrap_or(0);
        if done >= budget.mips
            || !assets.get_path(id).is_some_and(|p| {
                p.path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("dds"))
            })
        {
            continue;
        }
        if let Some(mut image) = images.get_mut(id) {
            let removed = skip_mips(&mut image, budget.mips - done);
            if removed > 0 {
                levels.insert(id, done + removed);
                changed = true;
            }
        }
    }
    if pressure && changed {
        // Rebuild descriptors that hold the former views, allowing old GPU images to drop.
        standard.iter_mut().for_each(|_| {});
        layered.iter_mut().for_each(|_| {});
    }
}

fn skip_mips(image: &mut Image, requested: u32) -> u32 {
    let d = &image.texture_descriptor;
    if d.dimension != wgpu::TextureDimension::D2
        || d.size.depth_or_array_layers != 1
        || d.size.width.max(d.size.height) < 1024
        || d.mip_level_count <= 1
        || image.texture_view_descriptor.is_some()
        || !d.usage.contains(wgpu::TextureUsages::TEXTURE_BINDING)
        || d.usage.intersects(
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::STORAGE_BINDING,
        )
    {
        return 0;
    }
    let Some(data) = &mut image.data else {
        return 0;
    };
    let count = requested.min(d.mip_level_count - 1);
    let offset = texture_bytes(d.size, count, d.format, d.dimension) as usize;
    if offset >= data.len() {
        return 0;
    }
    *data = data[offset..].to_vec();
    let d = &mut image.texture_descriptor;
    d.size.width = (d.size.width >> count).max(1);
    d.size.height = (d.size.height >> count).max(1);
    d.mip_level_count -= count;
    count
}

#[derive(Resource)]
struct MemoryClock(Instant);

fn cpu_assets(
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    clock: Res<MemoryClock>,
    mut last: Local<u64>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    let seconds = clock.0.elapsed().as_secs();
    if let Some(limit) = crate::arg_value(crate::perf_log::args(), "--perf-seconds")
        .and_then(|v| v.parse::<f64>().ok())
        && clock.0.elapsed().as_secs_f64() >= limit
    {
        exit.write(bevy::app::AppExit::Success);
    }
    if seconds / 5 == *last {
        return;
    }
    *last = seconds / 5;
    let mesh_bytes: usize = meshes
        .iter()
        .filter(|(_, m)| m.try_attribute(Mesh::ATTRIBUTE_POSITION).is_ok())
        .map(|(_, m)| {
            m.get_vertex_buffer_size() + m.get_index_buffer_bytes().map_or(0, <[u8]>::len)
        })
        .sum();
    let image_bytes: usize = images
        .iter()
        .map(|(_, i)| i.data.as_ref().map_or(0, Vec::len))
        .sum();
    info!(
        "VRAM CPU: t={seconds}s meshes={} mesh_bytes={mesh_bytes} images={} image_bytes={image_bytes}",
        meshes.len(),
        images.len()
    );
}

fn gpu_assets(
    device: Res<RenderDevice>,
    adapter: Res<RenderAdapterInfo>,
    images: Res<RenderAssets<GpuImage>>,
    meshes: Res<RenderAssets<bevy::render::mesh::RenderMesh>>,
    mut last: Local<Option<Instant>>,
    mut dxgi: Local<Option<VideoMemory>>,
) {
    if last.is_some_and(|t| t.elapsed().as_secs() < 5) {
        return;
    }
    if last.is_none() {
        *dxgi = VideoMemory::open(&adapter);
    }
    *last = Some(Instant::now());
    if let Some(memory) = dxgi.as_ref().and_then(VideoMemory::sample) {
        info!(
            "VRAM DXGI: dedicated_bytes={} usage_bytes={} budget_bytes={}",
            memory.0, memory.1, memory.2
        );
    }
    let mut textures = BTreeMap::<String, (usize, u64)>::new();
    let mut unique = std::collections::HashSet::new();
    for (_, image) in images.iter() {
        if !unique.insert(image.texture.id()) {
            continue;
        }
        let d = &image.texture_descriptor;
        let key = format!(
            "{:?}/{}x{}x{}/{}m",
            d.format, d.size.width, d.size.height, d.size.depth_or_array_layers, d.mip_level_count
        );
        let entry = textures.entry(key).or_default();
        entry.0 += 1;
        entry.1 += texture_bytes(d.size, d.mip_level_count, d.format, d.dimension)
            * u64::from(d.sample_count);
    }
    info!(
        "VRAM GPU: meshes={} textures={} texture_bytes={}",
        meshes.iter().count(),
        images.iter().count(),
        textures.values().map(|v| v.1).sum::<u64>()
    );
    for (kind, (count, bytes)) in textures {
        info!("VRAM texture: {kind} count={count} bytes={bytes}");
    }
    if let Some(report) = device.wgpu_device().generate_allocator_report() {
        info!(
            "VRAM allocator: allocated_bytes={} reserved_bytes={}",
            report.total_allocated_bytes, report.total_reserved_bytes
        );
        let mut kinds = BTreeMap::<String, (usize, u64)>::new();
        for allocation in report.allocations {
            let entry = kinds.entry(allocation.name).or_default();
            entry.0 += 1;
            entry.1 += allocation.size;
        }
        for (name, (count, bytes)) in kinds {
            info!("VRAM allocation: {name:?} count={count} bytes={bytes}");
        }
    }
}

fn texture_bytes(
    size: wgpu::Extent3d,
    mips: u32,
    format: wgpu::TextureFormat,
    dimension: wgpu::TextureDimension,
) -> u64 {
    let (bw, bh) = format.block_dimensions();
    (0..mips)
        .map(|mip| {
            let depth = if dimension == wgpu::TextureDimension::D3 {
                (size.depth_or_array_layers >> mip).max(1)
            } else {
                size.depth_or_array_layers
            };
            u64::from((size.width >> mip).max(1).div_ceil(bw))
                * u64::from((size.height >> mip).max(1).div_ceil(bh))
                * u64::from(depth)
                * u64::from(format.block_copy_size(None).unwrap_or(4))
        })
        .sum()
}

#[cfg(windows)]
struct VideoMemory {
    adapter: windows::Win32::Graphics::Dxgi::IDXGIAdapter3,
    dedicated: u64,
}

#[cfg(windows)]
impl VideoMemory {
    fn open(info: &wgpu::AdapterInfo) -> Option<Self> {
        use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
        use windows::core::Interface;
        // SAFETY: DXGI returns owned COM interfaces; all output pointers are valid locals.
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
            for index in 0.. {
                let Ok(adapter) = factory.EnumAdapters1(index) else {
                    break;
                };
                let desc = adapter.GetDesc1().ok()?;
                if desc.VendorId == info.vendor && desc.DeviceId == info.device {
                    return Some(Self {
                        adapter: adapter.cast().ok()?,
                        dedicated: desc.DedicatedVideoMemory as u64,
                    });
                }
            }
        }
        None
    }

    fn sample(&self) -> Option<(u64, u64, u64)> {
        use windows::Win32::Graphics::Dxgi::{
            DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO,
        };
        let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        // SAFETY: the COM interface is live and info points to a writable output structure.
        unsafe {
            self.adapter
                .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut info)
                .ok()?;
        }
        Some((self.dedicated, info.CurrentUsage, info.Budget))
    }
}

#[cfg(not(windows))]
struct VideoMemory;

#[cfg(not(windows))]
impl VideoMemory {
    fn open(_: &wgpu::AdapterInfo) -> Option<Self> {
        None
    }
    fn sample(&self) -> Option<(u64, u64, u64)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_respects_card_driver_and_simulated_capacity() {
        for (capacity, driver, requested, expected, mips) in [
            (
                16 * GIB - 5 * (1 << 20),
                14 * GIB,
                None,
                (16 * GIB - 5 * (1 << 20)) * 4 / 5,
                0,
            ),
            (24 * GIB, 20 * GIB, Some(10.0), 8 * GIB, 1),
            (6 * GIB, 5 * GIB, None, 6 * GIB * 4 / 5, 2),
            (10 * GIB, 7 * GIB, Some(24.0), 7 * GIB, 1),
        ] {
            assert_eq!(
                Budget::decision(capacity, driver, requested),
                (expected, mips)
            );
        }
        for invalid in ["0", "-1", "NaN", "inf", "--play"] {
            assert!(validate_args(&["--vram-budget".into(), invalid.into()]).is_err());
        }
        assert!(validate_args(&["--vram-budget".into()]).is_err());
    }

    #[test]
    fn mip_budget_removes_only_the_top_level_of_compressed_large_textures() {
        let size = wgpu::Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Bc1RgbaUnormSrgb;
        let top = texture_bytes(size, 1, format, wgpu::TextureDimension::D2) as usize;
        let length = texture_bytes(size, 11, format, wgpu::TextureDimension::D2) as usize;
        let mut image = Image::default();
        image.texture_descriptor.size = size;
        image.texture_descriptor.format = format;
        image.texture_descriptor.mip_level_count = 11;
        let mut data = vec![1; length];
        data[top..].fill(2);
        image.data = Some(data);
        assert_eq!(skip_mips(&mut image, 1), 1);
        assert_eq!(image.texture_descriptor.size.width, 512);
        assert_eq!(image.texture_descriptor.mip_level_count, 10);
        assert_eq!(image.data.as_ref().unwrap(), &vec![2; length - top]);
        assert_eq!(skip_mips(&mut image, 1), 0);
    }
}
