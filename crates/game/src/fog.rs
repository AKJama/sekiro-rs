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

// Provisional CPU upload conversions; see docs/SHADERS.md, Fog open questions.
// GameScale is retained as authored metadata, not presumed to multiply density.
const HEIGHT_DENSITY_PER_UNIT: f32 = 0.01;
const FIXED_DENSITY_PER_METRE: f32 = 1.0;
const DEPTH_METRES_PER_UNIT: f32 = 1.0;
const HEIGHT_OFFSET_METRES: f32 = 1.0;
const HEIGHT_DISTRIBUTION_PER_METRE: f32 = 1.0;
const SOURCE_RADIANCE_PER_UNIT: f32 = 1.0;
const DIRECTIONAL_COSINE_FLOOR: f32 = 0.01;

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
    // RGB extinction; w retains authored GameScale for future verified packing.
    extinction: Vec4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::gparam::{Entry, Group, Param, Value};

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
        assert_eq!(f.extinction.xyz(), Vec3::new(2., 0., 0.5));
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
        assert_eq!(noon.depths.y, 400.);
        assert_eq!(noon.extinction.w, 2.5);
        assert!((noon.density.x - 0.0015).abs() < 0.000001);
        assert!((noon.density.z - 0.0025).abs() < 0.000001);
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
            rgb.max(Vec3::ZERO)
                * v.and_then(|v| v.get(3).copied()).unwrap_or(1.).max(0.)
                * SOURCE_RADIANCE_PER_UNIT
        };
        Some(Self {
            density: Vec4::new(
                get("FixedDensity", 0.0015).max(0.) * FIXED_DENSITY_PER_METRE,
                get("HeightFogDistribution", 0.1).max(0.) * HEIGHT_DISTRIBUTION_PER_METRE,
                get("HeightFogDensity Scale", 0.25).max(0.) * HEIGHT_DENSITY_PER_UNIT,
                get("HeightFogOffset", -150.) * HEIGHT_OFFSET_METRES,
            ),
            depths: Vec4::new(
                get("NearDepth", 0.01).max(0.001) * DEPTH_METRES_PER_UNIT,
                get("FarDepth", 400.).max(1.) * DEPTH_METRES_PER_UNIT,
                get("SkyLight StartDist", 0.).max(0.) * DEPTH_METRES_PER_UNIT,
                get("DirLight StartDist", 0.).max(0.) * DEPTH_METRES_PER_UNIT,
            ),
            sky: color("SkyColor", Vec3::new(0.49, 0.55, 0.65))
                .extend(get("SkyLight HeightAtten", 0.).max(0.)),
            direct: color("DirectColor", Vec3::splat(0.5))
                .extend(get("DirLight HeightAtten", 0.).max(0.)),
            opposite: color("OppositeColor", Vec3::splat(0.15)).extend(0.),
            direction: sun.extend(1. / sun.y.abs().max(DIRECTIONAL_COSINE_FLOOR)),
            extinction: g
                .param("VolumetricFog", "AbsorptionScale")
                .and_then(|p| p.at(id, hour))
                .filter(|v| v.len() >= 3)
                .map_or(Vec3::ONE, |v| Vec3::new(v[0], v[1], v[2]))
                .max(Vec3::ZERO)
                .extend(get("GameScale", 1.)),
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
    mut last: Local<Option<(i32, f32)>>,
    added: Query<(), Added<Camera3d>>,
    selection: Res<FogSelection>,
    mut last_fog: Local<Option<i32>>,
) {
    let key = (lighting.light_set, lighting.hour);
    if *last == Some(key) && *last_fog == Some(selection.0) && added.is_empty() {
        return;
    }
    *last = Some(key);
    *last_fog = Some(selection.0);
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
    let value = (!std::env::args().any(|a| a == "--no-fog"))
        .then(|| GameFog::from_gparam(&lighting.gparam, key.0, key.1, set.sun_toward))
        .flatten();
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
struct FogPipeline(Vec<FogVariant>);
struct FogVariant {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedRenderPipelineId,
}
fn init_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
) {
    let variants = [false, true]
        .into_iter()
        .map(|multisampled| {
            let layout = BindGroupLayoutDescriptor::new(
                "game fog",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        if multisampled {
                            texture_depth_2d_multisampled()
                        } else {
                            texture_depth_2d()
                        },
                        uniform_buffer::<ViewUniform>(true),
                        uniform_buffer::<GameFog>(true),
                    ),
                ),
            );
            let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("Sekiro global fog".into()),
                layout: vec![layout.clone()],
                vertex: fullscreen.to_vertex_state(),
                fragment: Some(FragmentState {
                    shader: load_embedded_asset!(assets.as_ref(), "fog.wgsl"),
                    shader_defs: if multisampled {
                        vec!["MULTISAMPLED_DEPTH".into()]
                    } else {
                        vec![]
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
            FogVariant { layout, pipeline }
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
    let pipeline = &pipeline.0[usize::from(msaa.samples() > 1)];
    let (Some(pso), Some(view_binding), Some(fog_binding)) = (
        cache.get_render_pipeline(pipeline.pipeline),
        views.uniforms.binding(),
        uniforms.uniforms().binding(),
    ) else {
        return;
    };
    if target.main_texture_format() != TextureFormat::Rgba16Float {
        return;
    }
    let post = target.post_process_write();
    let bindings = ctx.render_device().create_bind_group(
        "game fog",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, depth.view(), view_binding, fog_binding)),
    );
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "game_fog");
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
    pass.set_render_pipeline(pso);
    pass.set_bind_group(0, &bindings, &[offset.offset, index.index()]);
    pass.draw(0..3, 0..1);
    drop(pass);
    span.end(ctx.command_encoder());
}
