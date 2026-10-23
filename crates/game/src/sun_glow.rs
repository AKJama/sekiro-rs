//! The game's Glow Filter (`GXGlowFilter`, #288): the sky dome's soft halo around the sun.
//!
//! Traced from the exe (RVAs, image base 0x140000000), see docs/LIGHTING.md:
//! - `GXGlowFilterParam` (binder 3b7a10): Enable, NumLevels (1-8), Blur Smoothness and
//!   FinalColorScale are the only fields that matter; the scheduler 2d7bb0 forces SampleScale,
//!   RgbScale and AlphaScale to 1, so mapbase's `Blur RgbScale` 1.5 has no effect.
//! - Forward materials write `colour * g_GlowScale.rgb * saturate(g_GlowScale.w)` to a glow
//!   target; `EmissiveToGlowPS` premultiplies it by alpha and zeroes NaNs.
//! - `GXGlowFilter::Draw` 2451f0 downsamples NumLevels times, folds the levels back from the
//!   smallest with blend constant (Smoothness, 1), so level 0 holds `sum s^i L_i`, and
//!   `GlowCompositeSinglePS` scales it by `FinalColorScale * fade / sum_{i<NumLevels} s^i`.
//! - The tone-map job (255c90 in 2508b0) draws that into the glare accumulation before tone
//!   mapping, which then reaches the scene as `glare * g_GlareLuminance`, like the bloom.
//!
//! Stand-ins, not traced (agreed for #288):
//! - The composite's blend Dest/Op is taken as additive ONE+ONE ([`COMPOSITE_BLEND`]).
//! - The CPU-generated 13-tap `Sample13WeightforGlow` weights are replaced by a standard
//!   13-tap separable Gaussian, sigma [`BLUR_SIGMA`] texels, at every level.
//!
//! Approximations of this port: only the sky dome feeds the glow (sky pixels, depth at
//! infinity, times the dome's `g_GlowScale`), read from the fogged HDR scene, so cloud and fog
//! in-scatter colour leak in at the dome's weight. The global fade is taken as 1, and each level
//! halves the previous one (the helper 1ca700's sizes are not traced).
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::renderer::RenderContext;
use bevy::render::{
    RenderApp, RenderStartup,
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    render_resource::{binding_types::*, *},
};
use sekiro_formats::gparam::Gparam;
use std::collections::HashMap;

const GROUP: &str = "Glow Filter-Editor";
/// Standard Gaussian sigma of the stand-in 13-tap blur, in texels of each level (untraced).
pub const BLUR_SIGMA: f32 = 2.0;
/// Stand-in for the untraced composite blend state +0x380 (built 24eb90): additive.
pub const COMPOSITE_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
};

#[derive(Component, Clone, ExtractComponent, ShaderType, Debug, PartialEq)]
#[extract_app(bevy::render::RenderApp)]
pub struct GameGlow {
    /// Smoothness s, composite scale FCS * fade / sum s^i, level count, blur sigma.
    pub params: Vec4,
    /// The sky dome's `g_GlowScale.rgb * saturate(w)`; w unused.
    pub source: Vec4,
}

fn field(g: &Gparam, name: &str, id: i32, hour: f32) -> Option<f32> {
    g.groups
        .iter()
        .filter(|group| group.name == GROUP)
        .flat_map(|group| &group.params)
        .find(|p| p.name == name)
        .and_then(|p| p.at(id, hour))
        .and_then(|v| v.first().copied())
        .filter(|v| v.is_finite())
}

/// `FinalColorScale * fade / sum_{i<levels} s^i`, the normaliser `GlowCompositeSinglePS` gets.
pub fn composite_scale(final_scale: f32, smoothness: f32, levels: u32, fade: f32) -> f32 {
    let sum: f32 = (0..levels).map(|i| smoothness.powi(i as i32)).sum();
    if sum > 0. {
        final_scale * fade / sum
    } else {
        0.
    }
}

impl GameGlow {
    /// The resolved Glow Filter, or `None` when it is off or nothing feeds it.
    /// Missing fields take the exe defaults (NumLevels 6, Smoothness 1, FinalColorScale 10).
    pub fn from_gparam(g: &Gparam, id: i32, hour: f32, sky: Vec3) -> Option<Self> {
        if field(g, "Glow Filter-Glow Enable", id, hour)? < 0.5 || sky.max_element() <= 0. {
            return None;
        }
        let levels = field(g, "Glow Filter - Glow NumLevels", id, hour)
            .unwrap_or(6.)
            .clamp(1., 8.) as u32;
        let smoothness = field(g, "Glow Filter - Blur Smoothness", id, hour)
            .unwrap_or(1.)
            .max(0.);
        let final_scale = field(g, "New Glow Filter-Glow - FinalColorScale", id, hour)
            .unwrap_or(10.)
            .max(0.);
        // The global fade at [[3f3b490]+0x12628]+0x80 is not tracked; it is 1 in play.
        let scale = composite_scale(final_scale, smoothness, levels, 1.);
        (scale > 0.).then(|| Self {
            params: Vec4::new(smoothness, scale, levels as f32, BLUR_SIGMA),
            source: sky.extend(0.),
        })
    }
}

pub struct SunGlowPlugin;
impl Plugin for SunGlowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sun_glow.wesl");
        app.add_plugins((
            ExtractComponentPlugin::<GameGlow>::default(),
            UniformComponentPlugin::<GameGlow>::default(),
        ))
        .add_systems(
            Update,
            (configure, keep_depth_sampling)
                .chain()
                .after(super::apply_lighting),
        )
        .add_observer(release_depth_sampling);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<GlowViews>()
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(
                    bevy::render::Render,
                    prune_views.in_set(bevy::render::RenderSystems::PrepareResources),
                );
        }
    }
}

type ConfigureKey = (i32, f32, String, u32, Vec3);

fn configure(
    mut commands: Commands,
    lighting: Res<super::MapLighting>,
    sky: Option<Res<crate::map::sky::SkyGlowScale>>,
    cameras: Query<Entity, With<Camera3d>>,
    added: Query<(), Added<Camera3d>>,
    mut last: Local<Option<ConfigureKey>>,
) {
    let sky = sky.map_or(Vec3::ZERO, |s| s.0);
    let key = (
        lighting.light_set,
        lighting.hour,
        lighting.source.clone(),
        lighting.revision,
        sky,
    );
    let unchanged = last.as_ref() == Some(&key);
    if unchanged && added.is_empty() {
        return;
    }
    let value = (!std::env::args().any(|a| a == "--no-sun-glow"))
        .then(|| GameGlow::from_gparam(&lighting.gparam, key.0, key.1, sky))
        .flatten();
    if !unchanged && lighting.fade.is_none() {
        info!("sun glow filter: {value:?}");
    }
    *last = Some(key);
    for entity in &cameras {
        if let Some(value) = &value {
            commands.entity(entity).insert(value.clone());
        } else {
            commands.entity(entity).remove::<GameGlow>();
        }
    }
}

/// Marks a camera whose depth sampling the glow switched on.
#[derive(Component)]
struct GlowDepthSampling;

fn keep_depth_sampling(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera3d), With<GameGlow>>,
) {
    for (entity, mut camera) in &mut cameras {
        let usages = TextureUsages::from(camera.depth_texture_usages);
        if !usages.contains(TextureUsages::TEXTURE_BINDING) {
            camera.depth_texture_usages = (usages | TextureUsages::TEXTURE_BINDING).into();
            commands.entity(entity).insert(GlowDepthSampling);
        }
    }
}

/// Retires the depth sampling the glow added, unless fog or the depth of field still read it
/// (a sampleable depth left behind lost FwLo's Vulkan device, #94).
#[allow(clippy::type_complexity)]
fn release_depth_sampling(
    event: On<Remove<GameGlow>>,
    mut commands: Commands,
    mut cameras: Query<
        &mut Camera3d,
        (
            With<GlowDepthSampling>,
            Without<super::fog::GameFog>,
            Without<super::lens_dof::GameDof>,
        ),
    >,
) {
    if let Ok(mut camera) = cameras.get_mut(event.entity) {
        let mut depth = TextureUsages::from(camera.depth_texture_usages);
        depth.remove(TextureUsages::TEXTURE_BINDING);
        camera.depth_texture_usages = depth.into();
        commands.entity(event.entity).remove::<GlowDepthSampling>();
    }
}

const PASSES: [&str; 6] = ["SOURCE", "DOWN", "BLUR_H", "BLUR_V", "UP", "COMPOSITE"];

struct GlowVariant {
    layout: BindGroupLayoutDescriptor,
    passes: [CachedRenderPipelineId; 6],
}

#[derive(Resource)]
pub(crate) struct GlowPipeline {
    variants: [GlowVariant; 2],
    sampler: Sampler,
}

fn init_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
    device: Res<bevy::render::renderer::RenderDevice>,
) {
    let variants = [false, true].map(|multisampled| {
        let layout = BindGroupLayoutDescriptor::new(
            "sun glow",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    if multisampled {
                        texture_depth_2d_multisampled()
                    } else {
                        texture_depth_2d()
                    },
                    uniform_buffer::<GameGlow>(true),
                ),
            ),
        );
        let passes = PASSES.map(|pass| {
            let mut defs = vec![pass.into()];
            if multisampled {
                defs.push("MULTISAMPLED_DEPTH".into());
            }
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(format!("Sekiro glow {pass}").into()),
                layout: vec![layout.clone()],
                vertex: fullscreen.to_vertex_state(),
                fragment: Some(FragmentState {
                    shader: load_embedded_asset!(assets.as_ref(), "sun_glow.wesl"),
                    shader_defs: defs,
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: (pass == "COMPOSITE").then_some(COMPOSITE_BLEND),
                        write_mask: ColorWrites::ALL,
                    })],
                    ..default()
                }),
                ..default()
            })
        });
        GlowVariant { layout, passes }
    });
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("sun glow"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        ..default()
    });
    commands.insert_resource(GlowPipeline { variants, sampler });
}

struct GlowLevels {
    size: UVec2,
    /// Per level: result and blur scratch.
    levels: Vec<(TextureView, TextureView)>,
}

#[derive(Resource, Default)]
pub(crate) struct GlowViews(HashMap<Entity, GlowLevels>);

fn prune_views(mut views: ResMut<GlowViews>, cameras: Query<(), With<GameGlow>>) {
    views.0.retain(|entity, _| cameras.contains(*entity));
}

fn level_views(
    size: UVec2,
    count: usize,
    device: &bevy::render::renderer::RenderDevice,
) -> GlowLevels {
    let make = |i: usize| {
        let divisor = 1 << (i + 1);
        device
            .create_texture(&TextureDescriptor {
                label: Some("sun glow level"),
                size: Extent3d {
                    width: size.x.div_ceil(divisor).max(1),
                    height: size.y.div_ceil(divisor).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor::default())
    };
    GlowLevels {
        size,
        levels: (0..count).map(|i| (make(i), make(i))).collect(),
    }
}

/// Everything the glow draw needs from the tone-map node's view.
pub(crate) struct GlowInputs<'a> {
    pub entity: Entity,
    pub settings: &'a GameGlow,
    pub uniform: BindingResource<'a>,
    pub index: u32,
    pub scene: &'a TextureView,
    pub depth: &'a TextureView,
    pub multisampled: bool,
    pub size: UVec2,
}

/// Runs the Glow Filter and adds it into `glare`, the half-resolution glare accumulation
/// the tone pass multiplies by the bloom's final colour and glare luminance.
pub(crate) fn draw(
    pipeline: &GlowPipeline,
    cache: &PipelineCache,
    views: &mut GlowViews,
    ctx: &mut RenderContext,
    input: GlowInputs,
    glare: &TextureView,
) {
    let variant = &pipeline.variants[usize::from(input.multisampled)];
    let passes: Vec<_> = variant
        .passes
        .iter()
        .filter_map(|id| cache.get_render_pipeline(*id))
        .collect();
    if passes.len() != PASSES.len() {
        return;
    }
    let count = input.settings.params.z as usize;
    let b = views
        .0
        .entry(input.entity)
        .or_insert_with(|| level_views(input.size, count, ctx.render_device()));
    if b.size != input.size || b.levels.len() != count {
        *b = level_views(input.size, count, ctx.render_device());
    }
    let layout = cache.get_bind_group_layout(&variant.layout);
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "sun_glow");
    let run = |ctx: &mut RenderContext,
               pass: usize,
               a: &TextureView,
               other: &TextureView,
               target: &TextureView| {
        let bindings = ctx.render_device().create_bind_group(
            "sun glow",
            &layout,
            &BindGroupEntries::sequential((
                a,
                other,
                &pipeline.sampler,
                input.depth,
                input.uniform.clone(),
            )),
        );
        let mut rp = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("sun glow stage"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            ..default()
        });
        rp.set_render_pipeline(passes[pass]);
        rp.set_bind_group(0, &bindings, &[input.index]);
        rp.draw(0..3, 0..1);
    };
    // EmissiveToGlowPS, then each level: downsample and the stand-in Gaussian.
    for i in 0..count {
        let (step, source) = if i == 0 {
            (0, input.scene)
        } else {
            (1, &b.levels[i - 1].0)
        };
        run(ctx, step, source, source, &b.levels[i].0);
        run(ctx, 2, &b.levels[i].0, &b.levels[i].0, &b.levels[i].1);
        run(ctx, 3, &b.levels[i].1, &b.levels[i].1, &b.levels[i].0);
    }
    // Fold back from the smallest with constant (Smoothness, 1): L_i + s * up(acc_{i+1}).
    for i in (1..count).rev() {
        run(
            ctx,
            4,
            &b.levels[i - 1].0,
            &b.levels[i].0,
            &b.levels[i - 1].1,
        );
        let level = &mut b.levels[i - 1];
        std::mem::swap(&mut level.0, &mut level.1);
    }
    // GlowCompositeSinglePS into the glare accumulation, additive (stand-in blend).
    run(ctx, 5, &b.levels[0].0, &b.levels[0].0, glare);
    span.end(ctx.command_encoder());
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::gparam::{Entry, Group, Param, Value};

    fn gparam(params: &[(&str, Value)]) -> Gparam {
        Gparam {
            groups: vec![Group {
                name: GROUP.into(),
                display_name: GROUP.into(),
                params: params
                    .iter()
                    .map(|(n, v)| Param {
                        name: (*n).into(),
                        display_name: (*n).into(),
                        entries: vec![Entry {
                            id: 0,
                            time: 0.,
                            value: v.clone(),
                        }],
                    })
                    .collect(),
            }],
        }
    }

    fn mapbase() -> Vec<(&'static str, Value)> {
        vec![
            ("Glow Filter-Glow Enable", Value::Int(1)),
            ("Glow Filter - Glow NumLevels", Value::Int(6)),
            ("Glow Filter - Blur Smoothness", Value::Float(1.)),
            ("New Glow Filter-Glow - FinalColorScale", Value::Float(1.)),
            ("Glow Filter-Glow - Blur RgbScale", Value::Float(1.5)),
        ]
    }

    #[test]
    fn composite_normalises_by_the_smoothness_series() {
        // Smoothness 1: the plain mean of the levels.
        assert!((composite_scale(1., 1., 6, 1.) - 1. / 6.).abs() < 1e-6);
        // Smoothness 0.5 over three levels: 1 + 0.5 + 0.25.
        assert!((composite_scale(10., 0.5, 3, 1.) - 10. / 1.75).abs() < 1e-5);
        assert_eq!(composite_scale(1., 0., 0, 1.), 0.);
    }

    #[test]
    fn mapbase_glow_takes_the_traced_fields_and_ignores_rgb_scale() {
        let sky = Vec3::splat(0.2);
        let glow = GameGlow::from_gparam(&gparam(&mapbase()), 0, 7., sky).unwrap();
        assert_eq!(glow.params.x, 1.);
        assert!((glow.params.y - 1. / 6.).abs() < 1e-6, "{glow:?}");
        assert_eq!(glow.params.z, 6.);
        assert_eq!(glow.source, Vec4::new(0.2, 0.2, 0.2, 0.));
        // Default.gparam's 0.1 scales it down tenfold.
        let mut base = mapbase();
        base[3].1 = Value::Float(0.1);
        let low = GameGlow::from_gparam(&gparam(&base), 0, 7., sky).unwrap();
        assert!((low.params.y - 0.1 / 6.).abs() < 1e-6);
        // Off, or a dome without glow, uploads nothing.
        base[0].1 = Value::Int(0);
        assert!(GameGlow::from_gparam(&gparam(&base), 0, 7., sky).is_none());
        assert!(GameGlow::from_gparam(&gparam(&mapbase()), 0, 7., Vec3::ZERO).is_none());
    }
}
