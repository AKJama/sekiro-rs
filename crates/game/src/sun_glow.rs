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
//! in-scatter colour leak in at the dome's weight, and so do alpha-blended draws that write no
//! depth over the sky (FXR sparks, fire, blended foliage; a known gap in docs/FIDELITY.md).
//! The global fade is taken as 1, and each level halves the previous one (the helper 1ca700's
//! sizes are not traced).
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
            configure
                .after(super::apply_lighting)
                .after(super::apply_post_effects)
                .after(crate::map::sky::SkyUpdate),
        );
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

/// Gives each post-processed scene camera the resolved Glow Filter, or takes it away.
/// The glow only reaches the scene through the glare accumulation, times the bloom's final
/// colour (255c90 in the tone-map job), so a camera whose bloom gain is 0 (`--no-bloom`, or an
/// authored zero) gets no glow and no depth sampling for it; `depth_sampling` owns that bit.
fn configure(
    mut commands: Commands,
    lighting: Res<super::MapLighting>,
    sky: Option<Res<crate::map::sky::SkyGlowScale>>,
    cameras: Query<(Entity, &super::tonemap::GamePost, Option<&GameGlow>)>,
    mut last: Local<Option<(ConfigureKey, Option<GameGlow>)>>,
) {
    let sky = sky.map_or(Vec3::ZERO, |s| s.0);
    let key = (
        lighting.light_set,
        lighting.hour,
        lighting.source.clone(),
        lighting.revision,
        sky,
    );
    if last.as_ref().is_none_or(|(last, _)| last != &key) {
        let value = (!std::env::args().any(|a| a == "--no-sun-glow"))
            .then(|| GameGlow::from_gparam(&lighting.gparam, key.0, key.1, sky))
            .flatten();
        if lighting.fade.is_none() {
            info!("sun glow filter: {value:?}");
        }
        *last = Some((key, value));
    }
    let value = last.as_ref().and_then(|(_, value)| value.as_ref());
    for (entity, post, current) in &cameras {
        let wanted = value.filter(|_| post.bloom.y > 0.);
        if wanted == current {
            continue;
        }
        if let Some(value) = wanted {
            commands.entity(entity).insert(value.clone());
        } else {
            commands.entity(entity).remove::<GameGlow>();
        }
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

/// Most bind groups a view keeps before the cache is dropped and refilled (a guard against
/// inputs that change identity every frame; a steady view needs about 4 per level).
const MAX_BIND_GROUPS: usize = 64;

struct GlowLevels {
    size: UVec2,
    /// Per level: result and blur scratch (the scratch then holds the folded result).
    levels: Vec<(TextureView, TextureView)>,
    /// Stage bind groups by their two input views, valid for [`Self::inputs`].
    bind_groups: HashMap<(TextureViewId, TextureViewId), BindGroup>,
    /// Depth view, uniform buffer and depth multisampling the bind groups were built with.
    inputs: Option<(TextureViewId, BufferId, bool)>,
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
        bind_groups: HashMap::new(),
        inputs: None,
    }
}

/// Everything the glow draw needs from the tone-map node's view.
pub(crate) struct GlowInputs<'a> {
    pub entity: Entity,
    pub settings: &'a GameGlow,
    pub uniform: BindingResource<'a>,
    pub uniform_buffer: BufferId,
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
    let inputs = (input.depth.id(), input.uniform_buffer, input.multisampled);
    if b.inputs != Some(inputs) || b.bind_groups.len() > MAX_BIND_GROUPS {
        b.bind_groups.clear();
        b.inputs = Some(inputs);
    }
    let GlowLevels {
        levels,
        bind_groups,
        ..
    } = b;
    // EmissiveToGlowPS, then each level: downsample and the stand-in Gaussian.
    let mut stages: Vec<(usize, &TextureView, &TextureView, &TextureView)> = Vec::new();
    for i in 0..count {
        let (step, source) = if i == 0 {
            (0, input.scene)
        } else {
            (1, &levels[i - 1].0)
        };
        stages.push((step, source, source, &levels[i].0));
        stages.push((2, &levels[i].0, &levels[i].0, &levels[i].1));
        stages.push((3, &levels[i].1, &levels[i].1, &levels[i].0));
    }
    // Fold back from the smallest with constant (Smoothness, 1): L_i + s * up(acc_{i+1}),
    // into each level's scratch, so the views a stage reads stay the same every frame.
    let folded = |i: usize| {
        if i + 1 == count {
            &levels[i].0
        } else {
            &levels[i].1
        }
    };
    for i in (1..count).rev() {
        stages.push((4, &levels[i - 1].0, folded(i), &levels[i - 1].1));
    }
    // GlowCompositeSinglePS into the glare accumulation, additive (stand-in blend).
    stages.push((5, folded(0), folded(0), glare));
    let layout = cache.get_bind_group_layout(&variant.layout);
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "sun_glow");
    for (pass, a, other, target) in stages {
        let bindings = bind_groups.entry((a.id(), other.id())).or_insert_with(|| {
            ctx.render_device().create_bind_group(
                "sun glow",
                &layout,
                &BindGroupEntries::sequential((
                    a,
                    other,
                    &pipeline.sampler,
                    input.depth,
                    input.uniform.clone(),
                )),
            )
        });
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
        rp.set_bind_group(0, &*bindings, &[input.index]);
        rp.draw(0..3, 0..1);
    }
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

    fn post(bloom_gain: f32) -> super::super::tonemap::GamePost {
        super::super::tonemap::GamePost {
            exposure: Vec4::ZERO,
            adaptation: Vec4::ZERO,
            bloom: Vec4::new(0., bloom_gain, 0., 1.),
            transfer: Vec4::ZERO,
            controls: Vec4::ZERO,
            curve: Vec4::ZERO,
            reinhard: Vec4::ZERO,
            colour: Mat4::IDENTITY,
        }
    }

    #[test]
    fn glow_goes_only_to_post_cameras_whose_bloom_reaches_the_scene() {
        let mut app = App::new();
        app.insert_resource(super::super::MapLighting {
            gparam: gparam(&mapbase()),
            hour: 7.,
            light_set: 0,
            applied: None,
            source: String::new(),
            fade: None,
            revision: 0,
        })
        .insert_resource(crate::map::sky::SkyGlowScale(Vec3::splat(0.2)))
        .add_systems(Update, configure);
        let scene = app.world_mut().spawn((Camera3d::default(), post(0.5))).id();
        let no_bloom = app.world_mut().spawn((Camera3d::default(), post(0.))).id();
        let hud = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        assert!(app.world().get::<GameGlow>(scene).is_some());
        assert!(
            app.world().get::<GameGlow>(no_bloom).is_none(),
            "--no-bloom"
        );
        assert!(
            app.world().get::<GameGlow>(hud).is_none(),
            "not a post camera"
        );
        // Bloom switched off later takes the glow away; back on brings it back.
        app.world_mut()
            .get_mut::<super::super::tonemap::GamePost>(scene)
            .unwrap()
            .bloom
            .y = 0.;
        app.update();
        assert!(app.world().get::<GameGlow>(scene).is_none());
        app.world_mut()
            .get_mut::<super::super::tonemap::GamePost>(scene)
            .unwrap()
            .bloom
            .y = 0.5;
        app.update();
        assert!(app.world().get::<GameGlow>(scene).is_some());
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
