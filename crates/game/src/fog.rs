//! Shader-derived global fog, integrated in HDR before bloom and tone mapping.
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::{
    FullscreenShader,
    schedule::{Core3d, Core3dSystems},
};
use bevy::prelude::*;
use bevy::render::{
    RenderApp, RenderStartup,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_resource::{binding_types::*, *},
    renderer::{RenderContext, ViewQuery},
    view::{ViewDepthTexture, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use sekiro_formats::gparam::Gparam;

#[derive(Resource, Default)]
pub(crate) struct FogSelection(pub i32);

#[derive(Component, Clone, ExtractComponent, ShaderType, Debug)]
pub struct GameFog {
    // fixed density, height coefficient, height density, affine height offset.
    density: Vec4,
    // near depth, far depth, sky start, directional start.
    depths: Vec4,
    sky: Vec4,
    direct: Vec4,
    opposite: Vec4,
    direction: Vec4,
    // RGB extinction; w is the signed world-height axis.
    extinction: Vec4,
    // NearOffset, slice count, sample offset, analytical continuation enabled.
    slices: Vec4,
    // Authored sky uses only the native fog continuation near the horizon.
    sky_blend: Vec4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::gparam::{Entry, Group, Param, Value};

    #[test]
    fn signed_height_and_scaled_depths_use_executable_units() {
        let g = Gparam {
            groups: vec![Group {
                name: "VolumetricFog".into(),
                display_name: String::new(),
                params: [
                    ("GameScale", 2.),
                    ("NearDepth", 3.),
                    ("FarDepth", 1.),
                    ("NearOffset", 4.),
                    ("FixedDensity", 0.4),
                    ("HeightFogDistribution", -0.8),
                    ("HeightFogDensity Scale", 0.6),
                    ("HeightFogOffset", 10.),
                    ("SkyLight StartDist", 5.),
                    ("DirLight StartDist", 6.),
                    ("DirLight AngleClip", 0.25),
                ]
                .into_iter()
                .map(|(name, value)| Param {
                    name: name.into(),
                    display_name: String::new(),
                    entries: vec![Entry {
                        id: 0,
                        time: 12.,
                        value: Value::Float(value),
                    }],
                })
                .collect(),
            }],
        };
        let f = GameFog::from_gparam(&g, 0, 12., Vec3::Y).unwrap();
        assert_eq!(f.depths, Vec4::new(6., 6.2, 10., 12.));
        assert_eq!(f.density, Vec4::new(0.2, 0.4, 0.15, 10.));
        assert_eq!(f.extinction.w, -1.);
        assert_eq!(f.direction.w, 4.);
        assert_eq!(f.slices.x, 8.);
        // Raw world Y=14 gives flipped height -4, not a GS-scaled offset.
        assert_eq!(f.extinction.w * 14. + f.density.w, -4.);
    }

    #[test]
    fn configuring_new_cameras_preserves_msaa_and_enables_depth_sampling() {
        let mut app = App::new();
        app.insert_resource(super::super::MapLighting {
            gparam: Gparam::default(),
            hour: 12.,
            light_set: 0,
            applied: None,
            source: String::new(),
        })
        .init_resource::<FogSelection>()
        .add_systems(Update, configure);
        let first = app
            .world_mut()
            .spawn((Camera3d::default(), Msaa::Sample4))
            .id();
        app.update();
        let second = app.world_mut().spawn(Camera3d::default()).id();
        app.update();
        for entity in [first, second] {
            assert!(app.world().get::<GameFog>(entity).is_some());
            let usages: TextureUsages = app
                .world()
                .get::<Camera3d>(entity)
                .unwrap()
                .depth_texture_usages
                .into();
            assert!(usages.contains(TextureUsages::TEXTURE_BINDING));
        }
        assert_eq!(*app.world().get::<Msaa>(first).unwrap(), Msaa::Sample4);
    }

    #[test]
    fn authored_zero_density_and_channel_extinction_survive_upload() {
        let g = Gparam {
            groups: vec![Group {
                name: "VolumetricFog ParamEditor".into(),
                display_name: String::new(),
                params: [
                    ("Global FixedDensity", Value::Float(0.)),
                    ("Global HeightFogDensity Scale", Value::Float(0.)),
                    ("Global AbsorptionScale", Value::Vec3([2., 0., 0.5])),
                    ("SkyLight StartDist", Value::Float(30.)),
                    ("DirLight StartDist", Value::Float(70.)),
                ]
                .into_iter()
                .map(|(name, value)| Param {
                    name: name.into(),
                    display_name: String::new(),
                    entries: vec![Entry {
                        id: 0,
                        time: 12.,
                        value,
                    }],
                })
                .collect(),
            }],
        };
        let f = GameFog::from_gparam(&g, 100, 12., Vec3::Y).unwrap();
        assert_eq!(f.density.x, 0.);
        assert_eq!(f.density.z, 0.);
        assert_eq!(f.extinction.xyz(), Vec3::new(3., 1., 1.5));
        assert_eq!(f.depths.zw(), Vec2::new(30., 70.));
    }

    #[test]
    fn castle_upload_uses_real_hour_rows_and_separate_directional_colours() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../cache/raw/param/drawparam/m11_01_0000.gparam");
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let g = sekiro_formats::gparam::parse(&bytes).unwrap();
        let noon = GameFog::from_gparam(&g, 0, 12., Vec3::Y).unwrap();
        let dusk = GameFog::from_gparam(&g, 0, 18., Vec3::Y).unwrap();
        assert_eq!(noon.depths.y, 1000.);
        assert_eq!(noon.extinction.w, 1.);
        assert!((noon.density.x - 0.0006).abs() < 0.000001);
        assert!((noon.density.z - 0.05).abs() < 0.000001);
        assert_ne!(noon.direct.xyz(), noon.opposite.xyz());
        assert_ne!(noon.direct.xyz(), dusk.direct.xyz());
    }
}

impl GameFog {
    pub fn from_gparam(g: &Gparam, id: i32, hour: f32, sun: Vec3) -> Option<Self> {
        let get = |name: &str, fallback: f32| {
            g.param("VolumetricFog", name)
                .and_then(|p| p.at(id, hour))
                .and_then(|v| v.first().copied())
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        };
        if get("Enable", 1.) < 0.5 {
            return None;
        }
        let color = |name: &str, fallback: Vec3| {
            let v = g.param("VolumetricFog", name).and_then(|p| p.at(id, hour));
            let rgb = v
                .as_ref()
                .filter(|v| v.len() >= 3)
                .map_or(fallback, |v| Vec3::new(v[0], v[1], v[2]));
            // Verified byte quantization and gamma 2.2 upload (39e3d0).
            super::fog_colour(rgb, v.and_then(|v| v.get(3).copied()).unwrap_or(1.))
        };
        // Verified CPU snapshot transforms (39e7e0, 39fa70); raw offset is metres.
        let gs = get("GameScale", 1.).max(f32::MIN_POSITIVE);
        let near = get("NearDepth", 0.01);
        let distribution = get("HeightFogDistribution", 0.1) / gs;
        let sign = if distribution < 0. { -1. } else { 1. };
        Some(Self {
            density: Vec4::new(
                get("FixedDensity", 0.0015) / gs,
                distribution.abs().max(f32::MIN_POSITIVE),
                0.5 * get("HeightFogDensity Scale", 0.25) / gs,
                -sign * get("HeightFogOffset", -150.),
            ),
            depths: Vec4::new(
                gs * near,
                gs * get("FarDepth", 400.).max(near + 0.1),
                gs * get("SkyLight StartDist", 0.),
                gs * get("DirLight StartDist", 0.),
            ),
            sky: color("SkyColor", Vec3::new(0.49, 0.55, 0.65))
                .extend(get("SkyLight HeightAtten", 0.).max(0.)),
            direct: color("DirectColor", Vec3::splat(0.5))
                .extend(get("DirLight HeightAtten", 0.).max(0.)),
            opposite: color("OppositeColor", Vec3::splat(0.15)).extend(0.),
            // Verified signed slant factor; bank/id resolution still inferred as sun.
            direction: sun.extend(1. / (sign * sun.y).max(get("AngleClip", 0.01))),
            extinction: (Vec3::ONE
                + g.param("VolumetricFog", "AbsorptionScale")
                    .and_then(|p| p.at(id, hour))
                    .filter(|v| v.len() >= 3)
                    .map_or(Vec3::ZERO, |v| Vec3::new(v[0], v[1], v[2])))
            .extend(sign),
            slices: Vec4::new(
                gs * get("NearOffset", 0.),
                get("VolumeDepth", 48.).clamp(4., 128.),
                get("VolumeSliceSampleOffset", 0.5),
                get("EnableAnalyticalFog", 1.),
            ),
            sky_blend: Vec4::ZERO,
        })
    }
}

pub struct FogPlugin;
impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fog.wgsl");
        app.add_plugins((
            ExtractComponentPlugin::<GameFog>::default(),
            UniformComponentPlugin::<GameFog>::default(),
        ))
        .add_systems(Update, configure.after(super::apply_lighting));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(
                    Core3d,
                    fog.before(Core3dSystems::EarlyPostProcess)
                        .after(Core3dSystems::MainPass),
                );
        }
    }
}

fn configure(
    mut commands: Commands,
    lighting: Res<super::MapLighting>,
    cameras: Query<(Entity, &Camera3d)>,
    mut last: Local<Option<(i32, f32, i32)>>,
    added: Query<(), Added<Camera3d>>,
    selection: Res<FogSelection>,
    dome: Option<Res<crate::map::sky::SkyEnabled>>,
) {
    let key = (lighting.light_set, lighting.hour);
    let cache_key = (key.0, key.1, selection.0);
    if *last == Some(cache_key) && added.is_empty() {
        return;
    }
    *last = Some(cache_key);
    // Preserve the independent MSB FogParamID. No separate Fog formula was recovered.
    // Inspect selected authored rows rather than silently aliasing this ID to LightSetID.
    let legacy: Vec<_> = lighting
        .gparam
        .groups
        .iter()
        .filter(|g| g.name.contains("Fog") && !g.name.contains("VolumetricFog"))
        .flat_map(|g| g.params.iter().map(|p| (&p.name, p.at(selection.0, key.1))))
        .collect();
    info!(
        "MSB FogParamID {}: separate Fog rows {legacy:?}",
        selection.0
    );
    let set = super::LightSet::from_gparam(&lighting.gparam, key.0, key.1);
    let mut value = (!std::env::args().any(|a| a == "--no-fog"))
        .then(|| GameFog::from_gparam(&lighting.gparam, key.0, key.1, set.sun_toward))
        .flatten();
    if dome.is_some()
        && let Some(value) = &mut value
    {
        value.sky_blend = Vec4::new(1., 0., 0.22, 0.);
    }
    info!("shader fog: {value:?}");
    for (entity, camera) in &cameras {
        let mut camera = camera.clone();
        camera.depth_texture_usages =
            (TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING).into();
        commands.entity(entity).insert(camera);
        if let Some(value) = &value {
            commands.entity(entity).insert(value.clone());
        } else {
            commands.entity(entity).remove::<GameFog>();
        }
    }
}

#[derive(Resource)]
struct FogPipeline(Vec<Option<FogVariant>>);
struct FogVariant {
    layout: BindGroupLayoutDescriptor,
    pipelines: Vec<CachedRenderPipelineId>,
}
fn init_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
    adapter: Res<bevy::render::renderer::RenderAdapter>,
) {
    let supported = adapter
        .get_texture_format_features(TextureFormat::Rgba16Float)
        .flags;
    let variants = [1, 2, 4, 8]
        .into_iter()
        .map(|samples| {
            if !supported.sample_count_supported(samples) {
                return None;
            }
            let multisampled = samples > 1;
            let layout = BindGroupLayoutDescriptor::new(
                "game fog",
                &if multisampled {
                    BindGroupLayoutEntries::sequential(
                        ShaderStages::FRAGMENT,
                        (
                            texture_depth_2d_multisampled(),
                            uniform_buffer::<ViewUniform>(true),
                            uniform_buffer::<GameFog>(true),
                        ),
                    )
                    .to_vec()
                } else {
                    BindGroupLayoutEntries::sequential(
                        ShaderStages::FRAGMENT,
                        (
                            texture_2d(TextureSampleType::Float { filterable: false }),
                            texture_depth_2d(),
                            uniform_buffer::<ViewUniform>(true),
                            uniform_buffer::<GameFog>(true),
                        ),
                    )
                    .to_vec()
                },
            );
            let pipelines = (0..if multisampled { 2 } else { 1 })
                .map(|step| {
                    let blend = multisampled.then_some(BlendState {
                        color: BlendComponent {
                            src_factor: if step == 0 {
                                BlendFactor::Zero
                            } else {
                                BlendFactor::One
                            },
                            dst_factor: if step == 0 {
                                BlendFactor::Src
                            } else {
                                BlendFactor::One
                            },
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    });
                    let mut shader_defs = Vec::new();
                    if multisampled {
                        shader_defs.push("MULTISAMPLED_DEPTH".into());
                        if step == 0 {
                            shader_defs.push("FOG_ATTENUATION".into());
                        }
                    }
                    cache.queue_render_pipeline(RenderPipelineDescriptor {
                        label: Some("Sekiro global fog".into()),
                        layout: vec![layout.clone()],
                        vertex: fullscreen.to_vertex_state(),
                        fragment: Some(FragmentState {
                            shader: load_embedded_asset!(assets.as_ref(), "fog.wgsl"),
                            shader_defs,
                            targets: vec![Some(ColorTargetState {
                                format: TextureFormat::Rgba16Float,
                                blend,
                                write_mask: ColorWrites::ALL,
                            })],
                            ..default()
                        }),
                        multisample: MultisampleState {
                            count: samples,
                            ..default()
                        },
                        ..default()
                    })
                })
                .collect();
            Some(FogVariant { layout, pipelines })
        })
        .collect();
    commands.insert_resource(FogPipeline(variants));
}
fn fog(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthTexture,
        &Msaa,
        &ViewUniformOffset,
        &DynamicUniformIndex<GameFog>,
    )>,
    pipeline: Res<FogPipeline>,
    cache: Res<PipelineCache>,
    views: Res<ViewUniforms>,
    uniforms: Res<ComponentUniforms<GameFog>>,
    mut ctx: RenderContext,
) {
    let (target, depth, msaa, offset, index) = view.into_inner();
    let multisampled = msaa.samples() > 1;
    let Some(pipeline) = &pipeline.0[msaa.samples().ilog2() as usize] else {
        return;
    };
    let (Some(view_binding), Some(fog_binding)) =
        (views.uniforms.binding(), uniforms.uniforms().binding())
    else {
        return;
    };
    if target.main_texture_format() != TextureFormat::Rgba16Float {
        return;
    }
    let psos: Vec<_> = pipeline
        .pipelines
        .iter()
        .filter_map(|id| cache.get_render_pipeline(*id))
        .collect();
    if psos.len() != pipeline.pipelines.len() {
        return;
    }
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "game_fog");
    if multisampled {
        // Preserve per-sample colour/depth correspondence through attenuation and
        // inscattering, then let the existing main attachment resolve the result.
        let bindings = ctx.render_device().create_bind_group(
            "game fog samples",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((depth.view(), view_binding, fog_binding)),
        );
        for pso in psos {
            let mut attachment = target.get_color_attachment();
            attachment.ops.load = LoadOp::Load;
            let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("game fog samples"),
                color_attachments: &[Some(attachment)],
                ..default()
            });
            pass.set_render_pipeline(pso);
            pass.set_bind_group(0, &bindings, &[offset.offset, index.index()]);
            pass.draw(0..3, 0..1);
        }
        span.end(ctx.command_encoder());
        return;
    }
    let post = target.post_process_write();
    let bindings = ctx.render_device().create_bind_group(
        "game fog",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, depth.view(), view_binding, fog_binding)),
    );
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("game fog"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })],
        ..default()
    });
    pass.set_render_pipeline(psos[0]);
    pass.set_bind_group(0, &bindings, &[offset.offset, index.index()]);
    pass.draw(0..3, 0..1);
    drop(pass);
    span.end(ctx.command_encoder());
}
