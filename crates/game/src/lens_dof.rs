//! The game's distance depth of field (`[GS]Lens Simulation`), as an HDR post pass after fog.
//!
//! `mapbase.gparam` enables the lens simulation with a linear far circle of confusion (CoC) from
//! 100 m to 500 m at `LinearCoCFactorFar` 0.1, which the Ashina base files keep.
//! Some block files override it (`m11_00_0200` and `m11_01_0200`: 50 m to 500 m at 0.5).
//! The far CoC follows `LensSimu2_ComputeCoC`:
//! `saturate(saturate((z - FarStart) / (FarEnd - FarStart)) * LinearCoCFactorFar)` in view depth,
//! and `LensSimu2_MakeBlurBuffer` scales the sky's far CoC by `1 - SkyMaskRate`.
//! The blur gathers that shader's two tap rings (16 at the radius, 8 at half of it).
//! The CPU scale from CoC to pixels (`g_vSampleScale`) is not traced, see [`MAX_RADIUS`].
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::{
    FullscreenShader,
    schedule::{Core3d, Core3dSystems},
};
use bevy::prelude::*;
use bevy::render::renderer::RenderContext;
use bevy::render::view::ViewDepthStencilTexture;
use bevy::render::{
    RenderApp, RenderStartup,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_resource::{binding_types::*, *},
    renderer::ViewQuery,
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use sekiro_formats::gparam::Gparam;

/// Far blur radius at CoC 1, as a fraction of the view height.
/// Fitted to retail reference shot 34 (Outskirts cliff ledge, spot 009, 2560x1440), not traced:
/// the native scale lives in the untraced CPU setup of `g_vSampleScale`.
/// 30 px at 1440 lines brings the far hill's high-frequency energy down to retail's; see
/// docs/LIGHTING.md.
pub const MAX_RADIUS: f32 = 30. / 1440.;

const GROUP: &str = "[GS]Lens Simulation";
const PREFIX: &str = "[GS]Lens Simulation Param";

#[derive(Component, Clone, ExtractComponent, ShaderType, Debug, PartialEq)]
#[extract_app(bevy::render::RenderApp)]
pub struct GameDof {
    /// FarStart, FarEnd, LinearCoCFactorFar, sky CoC weight (1 - SkyMaskRate).
    far: Vec4,
    /// Radius at CoC 1 as a fraction of the view height; yzw unused.
    blur: Vec4,
}

/// The exact `[GS]Lens Simulation Param <field>`; names are not matched by substring because
/// the separate `Lens Simulation` and Yebis groups carry overlapping words.
fn lens(g: &Gparam, field: &str, id: i32, hour: f32) -> Option<f32> {
    g.groups
        .iter()
        .filter(|group| group.name == GROUP)
        .flat_map(|group| &group.params)
        .find(|p| p.name.strip_prefix(PREFIX).map(str::trim) == Some(field))
        .and_then(|p| p.at(id, hour))
        .and_then(|v| v.first().copied())
        .filter(|v| v.is_finite())
}

impl GameDof {
    /// The far linear depth of field of the resolved draw params, or `None` when the lens
    /// simulation is off or its far CoC is in the (untraced) physical lens mode.
    pub fn from_gparam(g: &Gparam, id: i32, hour: f32) -> Option<Self> {
        if lens(g, "Enable", id, hour)? < 0.5 || lens(g, "FarCoCMode Linear", id, hour)? < 0.5 {
            return None;
        }
        let start = lens(g, "FarStart", id, hour)?;
        let end = lens(g, "FarEnd", id, hour)?.max(start + 0.01);
        let factor = lens(g, "LinearCoCFactorFar", id, hour)?.clamp(0., 1.);
        let sky = 1. - lens(g, "SkyMaskRate", id, hour).unwrap_or(0.).clamp(0., 1.);
        (factor > 0.).then(|| Self {
            far: Vec4::new(start, end, factor, sky),
            blur: Vec4::new(MAX_RADIUS, 0., 0., 0.),
        })
    }

    /// The far CoC at view depth `z` metres, as `LensSimu2_ComputeCoC` computes it.
    #[cfg(test)]
    pub fn far_coc(&self, z: f32) -> f32 {
        (((z - self.far.x) / (self.far.y - self.far.x)).clamp(0., 1.) * self.far.z).clamp(0., 1.)
    }
}

pub struct LensDofPlugin;
impl Plugin for LensDofPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "lens_dof.wesl");
        app.add_plugins((
            ExtractComponentPlugin::<GameDof>::default(),
            UniformComponentPlugin::<GameDof>::default(),
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
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(
                    Core3d,
                    dof.after(super::fog::fog)
                        .before(Core3dSystems::EarlyPostProcess)
                        .after(Core3dSystems::MainPass),
                );
        }
    }
}

/// Light set, hour, gparam source and lighting revision of the last configure.
type ConfigureKey = (i32, f32, String, u32);

fn configure(
    mut commands: Commands,
    lighting: Res<super::MapLighting>,
    cameras: Query<Entity, With<Camera3d>>,
    added: Query<(), Added<Camera3d>>,
    mut last: Local<Option<ConfigureKey>>,
) {
    let key = (
        lighting.light_set,
        lighting.hour,
        lighting.source.clone(),
        lighting.revision,
    );
    let unchanged = last.as_ref() == Some(&key);
    if unchanged && added.is_empty() {
        return;
    }
    let value = (!std::env::args().any(|a| a == "--no-dof"))
        .then(|| GameDof::from_gparam(&lighting.gparam, key.0, key.1))
        .flatten();
    if !unchanged && lighting.fade.is_none() {
        info!("lens depth of field: {value:?}");
    }
    *last = Some(key);
    for entity in &cameras {
        if let Some(value) = &value {
            commands.entity(entity).insert(value.clone());
        } else {
            commands.entity(entity).remove::<GameDof>();
        }
    }
}

/// Marks a camera whose depth sampling the depth of field switched on.
#[derive(Component)]
struct DofDepthSampling;

/// The pass reads depth; fog may add and later retire that usage bit on its own.
fn keep_depth_sampling(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera3d), With<GameDof>>,
) {
    for (entity, mut camera) in &mut cameras {
        let usages = TextureUsages::from(camera.depth_texture_usages);
        if !usages.contains(TextureUsages::TEXTURE_BINDING) {
            camera.depth_texture_usages = (usages | TextureUsages::TEXTURE_BINDING).into();
            commands.entity(entity).insert(DofDepthSampling);
        }
    }
}

/// Turns depth sampling off again when the depth of field that switched it on goes, unless fog
/// still reads depth: a sampleable depth left behind is what lost FwLo's Vulkan device (#94).
fn release_depth_sampling(
    event: On<Remove<GameDof>>,
    mut commands: Commands,
    mut cameras: Query<&mut Camera3d, (With<DofDepthSampling>, Without<super::fog::GameFog>)>,
) {
    if let Ok(mut camera) = cameras.get_mut(event.entity) {
        let mut depth = TextureUsages::from(camera.depth_texture_usages);
        depth.remove(TextureUsages::TEXTURE_BINDING);
        camera.depth_texture_usages = depth.into();
        commands.entity(event.entity).remove::<DofDepthSampling>();
    }
}

struct DofVariant {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedRenderPipelineId,
}

#[derive(Resource)]
pub(crate) struct DofPipeline {
    variants: [DofVariant; 2],
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
            "lens dof",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    if multisampled {
                        texture_depth_2d_multisampled()
                    } else {
                        texture_depth_2d()
                    },
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GameDof>(true),
                ),
            ),
        );
        let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Sekiro lens depth of field".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: load_embedded_asset!(assets.as_ref(), "lens_dof.wesl"),
                shader_defs: if multisampled {
                    vec!["MULTISAMPLED_DEPTH".into()]
                } else {
                    Vec::new()
                },
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        DofVariant { layout, pipeline }
    });
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("lens dof"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    commands.insert_resource(DofPipeline { variants, sampler });
}

pub(crate) fn dof(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthStencilTexture,
        &Msaa,
        &ViewUniformOffset,
        &DynamicUniformIndex<GameDof>,
    )>,
    pipeline: Res<DofPipeline>,
    cache: Res<PipelineCache>,
    views: Res<ViewUniforms>,
    uniforms: Res<ComponentUniforms<GameDof>>,
    mut ctx: RenderContext,
) {
    let (target, depth, msaa, offset, index) = view.into_inner();
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    // A newly extracted component can precede its rebuilt depth attachment.
    if !depth
        .texture()
        .usage()
        .contains(TextureUsages::TEXTURE_BINDING)
        || target.main_texture_format() != TextureFormat::Rgba16Float
    {
        return;
    }
    let variant = &pipeline.variants[usize::from(msaa.samples() > 1)];
    let (Some(pso), Some(view_binding), Some(dof_binding)) = (
        cache.get_render_pipeline(variant.pipeline),
        views.uniforms.binding(),
        uniforms.uniforms().binding(),
    ) else {
        return;
    };
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "lens_dof");
    let post = target.post_process_write();
    let bindings = ctx.render_device().create_bind_group(
        "lens dof",
        &cache.get_bind_group_layout(&variant.layout),
        &BindGroupEntries::sequential((
            post.source,
            &pipeline.sampler,
            depth_view,
            view_binding,
            dof_binding,
        )),
    );
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("lens dof"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Default::default()),
                store: StoreOp::Store,
            },
        })],
        ..default()
    });
    pass.set_render_pipeline(pso);
    pass.set_bind_group(0, &bindings, &[offset.offset, index.index()]);
    pass.draw(0..3, 0..1);
    drop(pass);
    span.end(ctx.command_encoder());
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::gparam::{Entry, Group, Param, Value};

    fn group(name: &str, params: &[(&str, Value)]) -> Group {
        Group {
            name: name.into(),
            display_name: name.into(),
            params: params
                .iter()
                .map(|(n, v)| Param {
                    name: format!("{PREFIX} {n}"),
                    display_name: (*n).into(),
                    entries: vec![Entry {
                        id: 0,
                        time: 0.,
                        value: v.clone(),
                    }],
                })
                .collect(),
        }
    }

    #[test]
    fn depth_sampling_goes_with_the_depth_of_field_unless_fog_reads_depth() {
        let mut app = App::new();
        app.add_systems(Update, keep_depth_sampling)
            .add_observer(release_depth_sampling);
        let dof = GameDof {
            far: Vec4::new(100., 500., 0.1, 0.75),
            blur: Vec4::ZERO,
        };
        let plain = app
            .world_mut()
            .spawn((Camera3d::default(), dof.clone()))
            .id();
        let fogged = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                dof,
                super::super::fog::GameFog::default(),
            ))
            .id();
        app.update();
        let sampled = |app: &App, entity| {
            TextureUsages::from(
                app.world()
                    .get::<Camera3d>(entity)
                    .unwrap()
                    .depth_texture_usages,
            )
            .contains(TextureUsages::TEXTURE_BINDING)
        };
        assert!(sampled(&app, plain) && sampled(&app, fogged));
        for entity in [plain, fogged] {
            app.world_mut().entity_mut(entity).remove::<GameDof>();
        }
        app.update();
        assert!(
            !sampled(&app, plain),
            "left sampleable after the depth of field went"
        );
        assert!(sampled(&app, fogged), "fog still reads depth");
    }

    #[test]
    fn linear_far_coc_ramps_from_far_start_to_far_end() {
        let g = Gparam {
            groups: vec![group(
                GROUP,
                &[
                    (" Enable", Value::Bool(true)),
                    ("FarCoCMode Linear", Value::Bool(true)),
                    ("FarStart", Value::Float(50.)),
                    ("FarEnd", Value::Float(500.)),
                    ("LinearCoCFactorFar", Value::Float(0.5)),
                    ("SkyMaskRate", Value::Float(0.25)),
                ],
            )],
        };
        let dof = GameDof::from_gparam(&g, 100, 12.).expect("enabled linear far DoF");
        assert_eq!(dof.far, Vec4::new(50., 500., 0.5, 0.75));
        assert_eq!(dof.far_coc(10.), 0.);
        assert_eq!(dof.far_coc(50.), 0.);
        assert!((dof.far_coc(275.) - 0.25).abs() < 1e-6);
        assert_eq!(dof.far_coc(5000.), 0.5);
    }

    #[test]
    fn disabled_or_physical_far_mode_uploads_nothing() {
        for (enable, linear) in [(false, true), (true, false)] {
            let g = Gparam {
                groups: vec![group(
                    GROUP,
                    &[
                        (" Enable", Value::Bool(enable)),
                        ("FarCoCMode Linear", Value::Bool(linear)),
                        ("FarStart", Value::Float(50.)),
                        ("FarEnd", Value::Float(500.)),
                        ("LinearCoCFactorFar", Value::Float(0.5)),
                    ],
                )],
            };
            assert_eq!(GameDof::from_gparam(&g, 0, 12.), None);
        }
    }

    /// Fails without the lens pass: the Outskirts stack the game selects (`mapbase` under the
    /// block's `m11_00_0000`) enables a far blur from 100 to 500 m at 0.1.
    #[test]
    fn installed_outskirts_stack_enables_far_depth_of_field() {
        let cache = sekiro_formats::cache::root();
        let read = |name: &str| {
            std::fs::read(cache.join(format!("raw/param/drawparam/{name}.gparam")))
                .ok()
                .map(|b| sekiro_formats::gparam::parse(&b).expect(name))
        };
        let (Some(defaults), Some(mapbase), Some(base), Some(overlay)) = (
            read("default"),
            read("mapbase"),
            read("m11_00_0000"),
            read("m11_00_0200"),
        ) else {
            return;
        };
        use sekiro_formats::gparam::{layer, priority};
        let stack = |files: &[&Gparam]| {
            let mut layers = vec![(priority::MAP_BASE, &mapbase)];
            layers.extend(
                files
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (priority::BLOCK_LAYER + i as u8, *g)),
            );
            layer(&layers, Some(&defaults))
        };
        let dof = GameDof::from_gparam(&stack(&[&base]), 100, 12.).expect("Outskirts far DoF");
        assert!(
            (dof.far - Vec4::new(100., 500., 0.1, 0.75))
                .abs()
                .max_element()
                < 1e-6
        );
        // `default.gparam` alone leaves the lens simulation off.
        assert_eq!(
            GameDof::from_gparam(&layer(&[], Some(&defaults)), 0, 12.),
            None
        );
        // Block file 2, where a layer selects it, moves the ramp to 50-500 m at 0.5.
        let dof = GameDof::from_gparam(&stack(&[&base, &overlay]), 0, 12.).expect("file 2");
        assert!(
            (dof.far - Vec4::new(50., 500., 0.5, 0.75))
                .abs()
                .max_element()
                < 1e-6
        );
    }
}
