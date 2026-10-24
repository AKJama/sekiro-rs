//! Shader-derived global fog, integrated in HDR before bloom and tone mapping.
use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::camera::{
    Exposure,
    visibility::{NoFrustumCulling, RenderLayers},
};
use bevy::core_pipeline::{
    FullscreenShader,
    schedule::{Core3d, Core3dSystems},
};
use bevy::diagnostic::FrameCount;
use bevy::ecs::entity::EntityHashMap;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::{
    ExtractedDirectionalLight, GpuLights, LightMeta, MAX_DIRECTIONAL_LIGHTS, ShadowSamplers,
    ViewLightsUniformOffset, ViewShadowBindings,
};
use bevy::prelude::*;
use bevy::render::renderer::RenderContext;
use bevy::render::view::ViewDepthStencilTexture;
use bevy::render::{
    Render, RenderApp, RenderStartup, RenderSystems,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::{binding_types::*, *},
    renderer::{RenderQueue, ViewQuery},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use sekiro_formats::gparam::Gparam;

#[derive(Resource, Default)]
pub(crate) struct FogSelection(pub i32);

#[derive(Component, Clone, Default, ExtractComponent, ShaderType, Debug, PartialEq)]
#[extract_app(bevy::render::RenderApp)]
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
    // Ring count, polygon sector angle, first vertex azimuth, unused.
    background_control: Vec4,
    // Native camera-centred sky outline: polygon apothem and world Y.
    background: [Vec4; 16],
    // Native scene radiance to the camera's HDR colour space.
    radiance_scale: Vec4,
    // DirLight ShadowSampleCount, DirLight ShadowDither, VolumeWidth (the shadow volume's
    // vertical cell count), and the render world's GpuLights index of the shadow-casting sun
    // (-1 when there is none; see `shadow_light`).
    shadow: Vec4,
    // DirLight JitterCount, JitterXYSize, FeedbackWeight and EnableReproj (the PreShadowed
    // reprojection, read on the CPU by `fog`).
    reproj: Vec4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sekiro_formats::gparam::{Entry, Group, Param, Value};

    fn fixture(params: Vec<(&str, i32, f32, Value)>) -> Gparam {
        let mut group = Group {
            name: "VolumetricFog ParamEditor".into(),
            display_name: String::new(),
            params: Vec::new(),
        };
        for (name, id, time, value) in params {
            if let Some(param) = group.params.iter_mut().find(|p| p.name == name) {
                param.entries.push(Entry { id, time, value });
            } else {
                group.params.push(Param {
                    name: name.into(),
                    display_name: String::new(),
                    entries: vec![Entry { id, time, value }],
                });
            }
        }
        Gparam {
            groups: vec![group],
        }
    }

    #[test]
    fn global_enable_is_independent_of_analytical_continuation_and_discrete() {
        for (enable, analytical, hour, expected) in [
            (None, false, 0., Some(0.)),
            (Some(false), true, 0., None),
            (Some(false), false, 5., None),
            (Some(false), false, 8., Some(0.)),
            (Some(true), false, 0., Some(0.)),
            (Some(true), true, 0., Some(1.)),
        ] {
            let mut values = vec![(
                "VolumetricFog EnableAnalyticalFog",
                0,
                0.,
                Value::Bool(analytical),
            )];
            if let Some(enable) = enable {
                values.extend([
                    ("VolumetricFog Enable", 0, 0., Value::Bool(enable)),
                    ("VolumetricFog Enable", 0, 12., Value::Bool(true)),
                ]);
            }
            let fog = GameFog::from_gparam(&fixture(values), 100, hour, Vec3::Y);
            assert_eq!(fog.map(|fog| fog.slices.w), expected);
        }
    }

    #[test]
    fn sun_shadow_taps_follow_authored_count_and_dither() {
        // Omitted rows keep constructor 1402a0740's 8 taps and 0.4 dither.
        let fog = GameFog::from_gparam(&fixture(vec![]), 0, 6., Vec3::Y).unwrap();
        assert_eq!(fog.shadow, Vec4::new(8., 0.4, 160., 0.));
        let g = fixture(vec![
            (
                "VolumetricFog DirLight ShadowSampleCount",
                0,
                0.,
                Value::Int(3),
            ),
            (
                "VolumetricFog DirLight ShadowDither",
                0,
                0.,
                Value::Float(1.),
            ),
        ]);
        let fog = GameFog::from_gparam(&g, 0, 6., Vec3::Y).unwrap();
        assert_eq!(fog.shadow.truncate().truncate(), Vec2::new(3., 1.));
    }

    #[test]
    fn shadow_reprojection_follows_the_authored_switch_jitter_and_feedback() {
        // Omitted rows: EnableReproj on (constructor 1402a0740), the Ashina jitter and feedback.
        let fog = GameFog::from_gparam(&fixture(vec![]), 0, 6., Vec3::Y).unwrap();
        assert_eq!(fog.reproj, Vec4::new(4., 1., 0.8, 1.));
        let g = fixture(vec![
            ("VolumetricFog DirLight JitterCount", 0, 0., Value::Int(2)),
            (
                "VolumetricFog DirLight JitterXYSize",
                0,
                0.,
                Value::Float(0.5),
            ),
            (
                "VolumetricFog DirLight FeedbackWeight",
                0,
                0.,
                Value::Float(0.9),
            ),
            (
                "VolumetricFog DirLight EnableReproj",
                0,
                0.,
                Value::Bool(false),
            ),
        ]);
        let fog = GameFog::from_gparam(&g, 0, 6., Vec3::Y).unwrap();
        assert_eq!(fog.reproj, Vec4::new(2., 0.5, 0.9, 0.));
    }

    #[test]
    fn shadow_jitter_cycles_distinct_cell_positions_and_dither_offsets() {
        // The user's blocky fog shadows (#345): one fixed ray and dither per cell showed the cell
        // grid and the 4x4 dither tile. Each of the JitterCount frames must move both.
        let frames: Vec<Vec3> = (0..4).map(|i| shadow_jitter(i, 4, 1.)).collect();
        for (i, a) in frames.iter().enumerate() {
            assert!(a.cmpge(Vec3::ZERO).all() && a.cmplt(Vec3::ONE).all(), "{a}");
            assert_eq!(a.z, i as f32 / 4.);
            for b in &frames[i + 1..] {
                assert!(a.truncate().distance(b.truncate()) > 0.1, "{a} {b}");
            }
        }
        assert_eq!(shadow_jitter(5, 4, 1.), frames[1]);
        // The sequence averages near the cell centre, and JitterXYSize scales the spread.
        let mean = frames.iter().map(|f| f.truncate()).sum::<Vec2>() / 4.;
        assert!(mean.distance(Vec2::splat(0.5)) < 1e-6, "{mean}");
        assert_eq!(shadow_jitter(1, 4, 0.).truncate(), Vec2::splat(0.5));
        assert_eq!(shadow_jitter(3, 1, 1.), Vec3::new(0.5, 0.5, 0.));
    }

    #[test]
    fn shadow_history_is_used_only_right_after_a_same_size_write() {
        let reproj = Vec4::new(4., 1., 0.8, 1.);
        let size = Extent3d {
            width: 144,
            height: 80,
            depth_or_array_layers: 4,
        };
        assert_eq!(history_weight(reproj, Some((9, size)), 10, size), 0.8);
        // First frame, a skipped frame, a resize, or reprojection switched off.
        assert_eq!(history_weight(reproj, None, 10, size), 0.);
        assert_eq!(history_weight(reproj, Some((8, size)), 10, size), 0.);
        let resized = Extent3d { width: 152, ..size };
        assert_eq!(history_weight(reproj, Some((9, size)), 10, resized), 0.);
        assert_eq!(
            history_weight(reproj.with_w(0.), Some((9, size)), 10, size),
            0.
        );
        assert_eq!(history_weight(reproj, Some((u32::MAX, size)), 0, size), 0.8);
    }

    #[test]
    fn shadow_volume_follows_the_native_froxel_grid() {
        // 1080p, VolumeWidth 160: Y = 160, X = 160 * 16/9 = 284.4 rounded up to 288; 32 slices
        // pack four to a texel.
        let size = shadow_volume_size(UVec2::new(1920, 1080), 160., 32.);
        assert_eq!(
            (size.width, size.height, size.depth_or_array_layers),
            (288, 160, 8)
        );
        // Both axes cap at a quarter of the viewport before rounding up to a multiple of 8, and a
        // partial group of slices still gets its texel.
        let size = shadow_volume_size(UVec2::new(640, 360), 160., 33.);
        assert_eq!(
            (size.width, size.height, size.depth_or_array_layers),
            (160, 96, 9)
        );
    }

    #[test]
    fn shadow_light_index_follows_the_gpu_lights_order() {
        let e = |i| Entity::from_raw_u32(i).unwrap();
        // Sorted (volumetric, shadow maps, entity): the shadowless light sorts first, so the sun
        // is GpuLights index 1 although it was listed first.
        let sun = ((false, true, e(1)), true, true);
        let fill = ((false, false, e(2)), true, false);
        assert_eq!(shadow_light_index(vec![sun, fill]), Some(1));
        // Lights outside the camera's layers take no GpuLights slot.
        let hidden = ((false, false, e(0)), false, false);
        assert_eq!(shadow_light_index(vec![sun, hidden]), Some(0));
        // No caster, or a caster past MAX_DIRECTIONAL_LIGHTS: the plain form runs.
        assert_eq!(shadow_light_index(vec![fill]), None);
        let mut many: Vec<_> = (10..10 + MAX_DIRECTIONAL_LIGHTS as u32)
            .map(|i| ((false, false, e(i)), true, false))
            .collect();
        many.push(sun);
        assert_eq!(shadow_light_index(many), None);
    }

    #[test]
    fn gates_and_the_switch_run_plain_fog() {
        let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(FogShadows::from_args(&args(&["sekiro-rs", "--play"])).0);
        for off in [
            "--no-fog-shadow",
            "--sweep",
            "--render-gate",
            "--reference-gate",
        ] {
            assert!(
                !FogShadows::from_args(&args(&["sekiro-rs", off])).0,
                "{off}"
            );
        }
    }

    #[test]
    fn omitted_analytical_switch_preserves_the_native_finite_volume() {
        let g = fixture(vec![(
            "VolumetricFog Global FixedDensity",
            0,
            0.,
            Value::Float(0.003),
        )]);
        let fog = GameFog::from_gparam(&g, 0, 22., Vec3::Y).unwrap();
        assert_eq!(fog.slices.w, 0.);
        assert_eq!(fog.density.x, 0.003);
        assert_eq!(fog.slices, Vec4::new(1., 32., 0., 0.));
        for map in ["m11_00", "m11_01"] {
            let path =
                sekiro_sim::cache::root().join(format!("raw/param/drawparam/{map}_0000.gparam"));
            if !path.is_file() {
                continue;
            }
            let g = sekiro_formats::gparam::parse(&std::fs::read(path).unwrap()).unwrap();
            assert!(fog_param_exact(&g, "EnableAnalyticalFog").is_none());
            let fog = GameFog::from_gparam(&g, 0, 22., Vec3::Y).unwrap();
            assert_eq!(fog.slices.w, 0.);
        }
    }

    fn light_fixture(angles: &[(i32, [f32; 2])]) -> Gparam {
        Gparam {
            groups: vec![Group {
                name: "LightSet ParamEditor".into(),
                display_name: String::new(),
                params: vec![Param {
                    name: "Directional Light Angle0".into(),
                    display_name: String::new(),
                    entries: angles
                        .iter()
                        .map(|&(id, angle)| Entry {
                            id,
                            time: 0.,
                            value: Value::Vec2(angle),
                        })
                        .collect(),
                }],
            }],
        }
    }

    #[test]
    fn fog_resolves_integer_bank_and_light_without_aliasing_the_collision_sun() {
        let mut g = fixture(vec![
            (
                "VolumetricFog DirLight LightBankId",
                0,
                0.,
                Value::Int(111_010_000),
            ),
            (
                "VolumetricFog DirLight LightBankId",
                0,
                12.,
                Value::Int(111_020_000),
            ),
        ]);
        g.groups
            .extend(light_fixture(&[(0, [-0.4, 0.2]), (100, [-1., 2.])]).groups);
        let mut banks = FogBanks::default();
        banks.0.insert(
            "m11_02_0000".into(),
            Some(light_fixture(&[(0, [-0.8, -0.2]), (42, [-0.7, 1.])])),
        );
        assert_eq!(
            fog_integer(&g, "DirLight LightBankId", 100, 5.),
            Some(111_010_000)
        );
        let local = banks.direction(&g, "m11_01_0750", 100, 0.);
        assert_eq!(local, super::super::toward(&[-0.4, 0.2]));
        assert_ne!(
            local,
            super::super::LightSet::from_gparam(&g, 100, 5.).sun_toward
        );
        let external = banks.direction(&g, "m11_01_0750", 100, 12.);
        assert_eq!(external, super::super::toward(&[-0.8, -0.2]));
        let midpoint = banks.direction(&g, "m11_01_0750", 100, 6.);
        assert!(midpoint.distance((local + external).normalize()) < 0.000001);
        g.groups[0].params.push(Param {
            name: "VolumetricFog DirLight LightId".into(),
            display_name: String::new(),
            entries: vec![Entry {
                id: 0,
                time: 0.,
                value: Value::Int(42),
            }],
        });
        assert_eq!(
            banks.direction(&g, "m11_01_0750", 100, 12.),
            super::super::toward(&[-0.7, 1.])
        );
        let exact = fixture(vec![(
            "DirLight LightBankId",
            0,
            0.,
            Value::Int(111_010_001),
        )]);
        assert_eq!(
            fog_integer(&exact, "DirLight LightBankId", 0, 0.),
            Some(111_010_001)
        );
        assert!(
            FogBanks::light(&light_fixture(&[(43, [0.4, 0.2])]), 43, 0.)
                .unwrap()
                .y
                < 0.
        );
    }

    #[test]
    fn sky_swaps_update_fog_in_the_same_frame_without_reinserting_diagnostic_fog() {
        #[derive(Component)]
        struct TestDome;
        #[derive(Component)]
        struct Diagnostic;
        #[derive(Resource)]
        struct NextDome(Handle<Mesh>);
        fn swap(next: Res<NextDome>, mut domes: Query<&mut Mesh3d, With<TestDome>>) {
            for mut mesh in &mut domes {
                mesh.0 = next.0.clone();
            }
        }
        fn diagnostics(mut commands: Commands, cameras: Query<Entity, With<Diagnostic>>) {
            for entity in &cameras {
                commands.entity(entity).remove::<GameFog>();
            }
        }
        let mesh = |height: f32| {
            let positions: Vec<_> = [-1000., height]
                .into_iter()
                .flat_map(|y| {
                    (0..16).map(move |i| {
                        let angle = i as f32 * std::f32::consts::TAU / 16.;
                        [1000. * angle.cos(), y, 1000. * angle.sin()]
                    })
                })
                .collect();
            let mut mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0., 0.]; positions.len()]);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh
        };
        let mut app = App::new();
        app.insert_resource(super::super::MapLighting {
            gparam: Gparam::default(),
            hour: 12.,
            light_set: 0,
            applied: None,
            source: String::new(),
            fade: None,
            revision: 0,
        })
        .init_resource::<FogSelection>()
        .init_resource::<Assets<Mesh>>()
        .add_systems(Update, (configure, swap, diagnostics).chain())
        .add_systems(PostUpdate, update_background);
        let first = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(mesh(1000.));
        let second = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(mesh(1300.));
        app.insert_resource(NextDome(first.clone()));
        app.world_mut().spawn((
            TestDome,
            Mesh3d(first),
            Visibility::Inherited,
            NoFrustumCulling,
            NotShadowCaster,
            NotShadowReceiver,
        ));
        let camera = app.world_mut().spawn(Camera3d::default()).id();
        let diagnostic = app
            .world_mut()
            .spawn((Camera3d::default(), Diagnostic))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<GameFog>(camera).unwrap().background[1].y,
            1000.
        );
        app.insert_resource(NextDome(second));
        app.update();
        assert_eq!(
            app.world().get::<GameFog>(camera).unwrap().background[1].y,
            1300.
        );
        assert!(app.world().get::<GameFog>(diagnostic).is_none());
        let scale = app.world().get::<GameFog>(camera).unwrap().radiance_scale.x;
        app.world_mut().entity_mut(camera).insert(Exposure {
            ev100: Exposure::default().ev100 + 1.,
        });
        app.update();
        assert!(
            (app.world().get::<GameFog>(camera).unwrap().radiance_scale.x - scale * 0.5).abs()
                < 0.00001
        );
    }

    #[test]
    fn installed_fog_banks_and_enabled_authored_densities() {
        let cache = sekiro_sim::cache::root();
        if !cache
            .join("raw/param/drawparam/m15_00_0000.gparam")
            .is_file()
        {
            return;
        }
        let mut banks = FogBanks::default();
        for map in [
            "m10_00", "m11_00", "m11_01", "m11_02", "m13_00", "m15_00", "m17_00", "m20_00",
            "m25_00",
        ] {
            let selection =
                crate::lighting_selection::select(&cache, &format!("{map}_00_00"), None, None)
                    .unwrap();
            let bytes = std::fs::read(
                cache.join(format!("raw/param/drawparam/{}.gparam", selection.source)),
            )
            .unwrap();
            let g = sekiro_formats::gparam::parse(&bytes).unwrap();
            let direction = banks.direction(&g, &selection.source, 100, selection.hour);
            let fog = GameFog::from_gparam(&g, 100, selection.hour, direction).unwrap();
            println!(
                "FOG_NATIVE {map}: source={}, hour={}, bank={:?}, density={:?}, depths={:?}, sky={:?}, direct={:?}, opposite={:?}, direction={:?}, analytical={}",
                selection.source,
                selection.hour,
                fog_integer(&g, "DirLight LightBankId", 100, selection.hour),
                fog.density,
                fog.depths,
                fog.sky,
                fog.direct,
                fog.opposite,
                fog.direction,
                fog.slices.w
            );
            if map == "m15_00" {
                assert!((fog.density.x - 0.007).abs() < 0.000001);
                assert_eq!(fog.slices.w, 0.);
            }
            if map == "m10_00" {
                assert!((fog.density.x - 0.0032).abs() < 0.000001);
                assert_eq!(fog.slices.w, 0.);
            }
        }
    }

    #[test]
    fn native_sky_profile_intersections_match_the_authored_triangles() {
        let dir = sekiro_sim::cache::root().join("maps/m89_00_00_99");
        let Ok(bytes) = std::fs::read(dir.join("sky.json")) else {
            return;
        };
        let library: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for (name, row) in library["models"].as_object().unwrap() {
            if !row["shader"]
                .as_str()
                .unwrap_or_default()
                .contains("SkyDome")
            {
                continue;
            }
            let bytes = std::fs::read(dir.join(format!("models/{name}.glb"))).unwrap();
            let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
            let binary = &bytes[28 + length..];
            let primitive = json["meshes"][0]["primitives"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["material"] == row["material_index"])
                .unwrap();
            let accessor = |index: usize| {
                let accessor = &json["accessors"][index];
                let view = &json["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
                let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
                    + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
                (accessor, view, offset)
            };
            let (a, v, offset) =
                accessor(primitive["attributes"]["POSITION"].as_u64().unwrap() as usize);
            let positions: Vec<[f32; 3]> = (0..a["count"].as_u64().unwrap() as usize)
                .map(|i| {
                    std::array::from_fn(|axis| {
                        let offset =
                            offset + i * v["byteStride"].as_u64().unwrap_or(12) as usize + axis * 4;
                        f32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap())
                    })
                })
                .collect();
            let background = Background::from_positions(&positions)
                .unwrap_or_else(|| panic!("unsupported native sky outline {name}"));
            let (a, _, offset) = accessor(primitive["indices"].as_u64().unwrap() as usize);
            let size = if a["componentType"] == 5123 { 2 } else { 4 };
            let indices: Vec<usize> = (0..a["count"].as_u64().unwrap() as usize)
                .map(|i| {
                    let offset = offset + i * size;
                    if size == 2 {
                        u16::from_le_bytes(binary[offset..offset + 2].try_into().unwrap()) as usize
                    } else {
                        u32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap()) as usize
                    }
                })
                .collect();
            for ray in [
                Vec3::X,
                Vec3::Z,
                Vec3::Y,
                Vec3::new(-0.4, 0.7, 1.).normalize(),
                Vec3::new(0.8, -0.1, -0.2).normalize(),
            ] {
                let mesh_distance = indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .filter_map(|triangle| {
                        let a = Vec3::from_array(positions[triangle[0]]);
                        let b = Vec3::from_array(positions[triangle[1]]);
                        let c = Vec3::from_array(positions[triangle[2]]);
                        let edge = b - a;
                        let edge2 = c - a;
                        let cross = ray.cross(edge2);
                        let determinant = edge.dot(cross);
                        if determinant.abs() < 0.000001 {
                            return None;
                        }
                        let u = (-a).dot(cross) / determinant;
                        let q = (-a).cross(edge);
                        let v = ray.dot(q) / determinant;
                        let t = edge2.dot(q) / determinant;
                        (u >= -0.00001 && v >= -0.00001 && u + v <= 1.00001 && t > 0.).then_some(t)
                    })
                    .min_by(f32::total_cmp)
                    .unwrap();
                let angle = ray.z.atan2(ray.x);
                let step = background.control.y;
                let normal = background.control.z
                    + (((angle - background.control.z) / step).floor() + 0.5) * step;
                let radial = ray.xz().length() * (angle - normal).cos();
                let profile_distance = background.rings[..background.control.x as usize]
                    .windows(2)
                    .filter_map(|ring| {
                        let a = ring[0].xy();
                        let delta = ring[1].xy() - a;
                        let denominator = radial * delta.y - ray.y * delta.x;
                        if denominator.abs() < 0.000001 {
                            return None;
                        }
                        let t = (a.x * delta.y - a.y * delta.x) / denominator;
                        let u =
                            (Vec2::new(radial, ray.y) * t - a).dot(delta) / delta.length_squared();
                        (t > 0. && (-0.00001..=1.00001).contains(&u)).then_some(t)
                    })
                    .min_by(f32::total_cmp)
                    .unwrap();
                assert!(
                    (mesh_distance - profile_distance).abs() < 0.05 + 0.00002 * mesh_distance,
                    "{name} {ray:?}: mesh={mesh_distance}, profile={profile_distance}"
                );
                println!(
                    "FOG_BACKGROUND {name} {ray:?}: mesh={mesh_distance:.4}, profile={profile_distance:.4}"
                );
            }
        }
    }

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
            fade: None,
            revision: 0,
        })
        .init_resource::<FogSelection>()
        .add_systems(Update, configure)
        .add_systems(
            PostUpdate,
            super::super::depth_sampling::sync_depth_sampling,
        );
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
        app.world_mut().entity_mut(first).remove::<GameFog>();
        app.update();
        assert!(
            !TextureUsages::from(
                app.world()
                    .get::<Camera3d>(first)
                    .unwrap()
                    .depth_texture_usages
            )
            .contains(TextureUsages::TEXTURE_BINDING)
        );
        // Enter a bank with fog disabled: retire only the depth usage owned by fog.
        let third = app
            .world_mut()
            .spawn(Camera3d {
                depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_SRC)
                    .into(),
                ..default()
            })
            .id();
        app.update();
        app.world_mut()
            .resource_mut::<super::super::MapLighting>()
            .gparam = fixture(vec![("VolumetricFog Enable", 0, 0., Value::Bool(false))]);
        app.world_mut()
            .resource_mut::<super::super::MapLighting>()
            .source = "fog disabled".into();
        app.update();
        for entity in [first, second] {
            assert!(app.world().get::<GameFog>(entity).is_none());
            let usages = TextureUsages::from(
                app.world()
                    .get::<Camera3d>(entity)
                    .unwrap()
                    .depth_texture_usages,
            );
            assert!(!usages.contains(TextureUsages::TEXTURE_BINDING));
        }
        let usages = TextureUsages::from(
            app.world()
                .get::<Camera3d>(third)
                .unwrap()
                .depth_texture_usages,
        );
        assert!(usages.contains(TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC));
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
        let path = sekiro_sim::cache::root().join("raw/param/drawparam/m11_01_0000.gparam");
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
            g.f32_at("VolumetricFog", name, id, hour)
                .unwrap_or(fallback)
        };
        // Renderer enable and analytical continuation are independent discrete switches.
        if fog_param_exact(g, "Enable")
            .and_then(|p| p.nearest(id, hour))
            .and_then(|v| v.first().copied())
            .unwrap_or(1.)
            < 0.5
        {
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
            // The caller supplies the resolved native bank/light direction.
            direction: sun.extend(1. / (sign * sun.y).max(get("AngleClip", 0.01))),
            extinction: (Vec3::ONE
                + g.param("VolumetricFog", "AbsorptionScale")
                    .and_then(|p| p.at(id, hour))
                    .filter(|v| v.len() >= 3)
                    .map_or(Vec3::ZERO, |v| Vec3::new(v[0], v[1], v[2])))
            .extend(sign),
            slices: Vec4::new(
                // Constructor 1402a0740: +0x14 = 1, +0x08 = 32, +0x24 = 0.
                // The base Ashina GPARAMs omit these sampling controls.
                gs * get("NearOffset", 1.),
                get("VolumeDepth", 32.).clamp(4., 128.),
                get("VolumeSliceSampleOffset", 0.),
                fog_param_exact(g, "EnableAnalyticalFog")
                    .and_then(|p| p.nearest(id, hour))
                    .and_then(|v| v.first().copied())
                    // 1402a0740 initializes blended-record byte +0x20 to false.
                    .unwrap_or(0.),
            ),
            background_control: Vec4::ZERO,
            background: [Vec4::ZERO; 16],
            radiance_scale: Vec4::X,
            // Constructor 1402a0740 defaults: ShadowSampleCount 8 (+0xc8), ShadowDither 0.4
            // (+0xcc); the debug menu (1403a5820) bounds the count to 1..32.
            shadow: Vec4::new(
                get("DirLight ShadowSampleCount", 8.).round().clamp(1., 32.),
                get("DirLight ShadowDither", 0.4).clamp(0., 1.),
                // Authored 80..320; the 160 fallback is not traced in the constructor.
                get("VolumeWidth", 160.).clamp(8., 640.),
                0.,
            ),
            // Constructor 1402a0740 defaults EnableReproj to true; the other three defaults were
            // not traced, so the fallbacks are the Ashina GPARAM values (4, 1.0, 0.8).
            reproj: Vec4::new(
                get("DirLight JitterCount", 4.).round().clamp(1., 64.),
                get("DirLight JitterXYSize", 1.).clamp(0., 1.),
                get("DirLight FeedbackWeight", 0.8).clamp(0., 1.),
                fog_param_exact(g, "DirLight EnableReproj")
                    .and_then(|p| p.nearest(id, hour))
                    .and_then(|v| v.first().copied())
                    .unwrap_or(1.),
            ),
        })
    }
}

fn fog_param_exact<'a>(g: &'a Gparam, name: &str) -> Option<&'a sekiro_formats::gparam::Param> {
    g.groups
        .iter()
        .filter(|group| group.name.starts_with("VolumetricFog"))
        .flat_map(|group| &group.params)
        .find(|param| {
            let canonical = param
                .name
                .strip_prefix("VolumetricFog ")
                .unwrap_or(&param.name);
            canonical.strip_prefix("Global ").unwrap_or(canonical) == name
        })
}

// Read identifiers directly from their integer entries, without interpolation or f32 rounding.
fn fog_integer(g: &Gparam, name: &str, id: i32, hour: f32) -> Option<i32> {
    let param = fog_param_exact(g, name)?;
    let row = if param.entries.iter().any(|entry| entry.id == id) {
        id
    } else {
        0
    };
    let distance = |time: f32| {
        let d = (hour.rem_euclid(24.) - time.rem_euclid(24.)).abs();
        d.min(24. - d)
    };
    match &param
        .entries
        .iter()
        .filter(|entry| entry.id == row)
        .min_by(|a, b| distance(a.time).total_cmp(&distance(b.time)))?
        .value
    {
        sekiro_formats::gparam::Value::Int(value) => Some(*value),
        _ => None,
    }
}

fn bank_map(bank: i32) -> Option<String> {
    // Native map-bank producer f47270/f472a0: ((area + 100) * 100 + block) * 10000.
    let area = bank / 1_000_000 - 100;
    let block = bank / 10_000 % 100;
    ((0..100).contains(&area) && bank % 10_000 == 0).then(|| format!("m{area:02}_{block:02}"))
}

fn fog_bounds(
    param: &sekiro_formats::gparam::Param,
    id: i32,
    hour: f32,
) -> Option<(
    &sekiro_formats::gparam::Entry,
    &sekiro_formats::gparam::Entry,
    f32,
)> {
    let mut entries: Vec<_> = param
        .entries
        .iter()
        .filter(|entry| entry.id == id)
        .collect();
    if entries.is_empty() && id != 0 {
        return fog_bounds(param, 0, hour);
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort_by(|a, b| a.time.total_cmp(&b.time));
    let hour = hour.rem_euclid(24.);
    let (a, b) = match entries.iter().position(|entry| entry.time > hour) {
        Some(0) | None => (entries[entries.len() - 1], entries[0]),
        Some(i) => (entries[i - 1], entries[i]),
    };
    let span = (b.time - a.time).rem_euclid(24.);
    let blend = if span > 0. {
        (hour - a.time).rem_euclid(24.) / span
    } else {
        0.
    };
    Some((a, b, blend))
}

#[derive(Default)]
struct FogBanks(std::collections::BTreeMap<String, Option<Gparam>>);

impl FogBanks {
    fn direction(&mut self, g: &Gparam, source: &str, id: i32, hour: f32) -> Vec3 {
        let pair = fog_param_exact(g, "DirLight LightBankId")
            .and_then(|param| fog_bounds(param, id, hour));
        let Some((a, b, blend)) = pair else {
            return self.resolve(g, source, 1, 0, hour);
        };
        let integer = |entry: &sekiro_formats::gparam::Entry| match &entry.value {
            sekiro_formats::gparam::Value::Int(value) => *value,
            _ => 1,
        };
        let bank_a = integer(a);
        let bank_b = integer(b);
        let light_a = fog_integer(g, "DirLight LightId", id, a.time).unwrap_or(0);
        let light_b = fog_integer(g, "DirLight LightId", id, b.time).unwrap_or(0);
        let direction = self.resolve(g, source, bank_a, light_a, hour);
        if blend == 0. || (bank_a, light_a) == (bank_b, light_b) {
            return direction;
        }
        let other = self.resolve(g, source, bank_b, light_b, hour);
        // 169080 -> 30d730/30d3e0 blends resolved directions by a spherical rotation.
        // Identifiers themselves are never interpolated.
        let rotation = Quat::from_rotation_arc(direction.normalize(), other.normalize());
        let result = Quat::IDENTITY.slerp(rotation, blend) * direction;
        info!(
            "fog light blend: ({bank_a},{light_a}) -> ({bank_b},{light_b}), factor={blend}, direction={result:?}"
        );
        result
    }

    fn resolve(&mut self, g: &Gparam, source: &str, bank: i32, light: i32, hour: f32) -> Vec3 {
        let stem = bank_map(bank).map(|map| format!("{map}_0000"));
        let current = bank_map(bank).is_some_and(|map| source.starts_with(&map));
        let bank_source = if current {
            source
        } else {
            stem.as_deref().unwrap_or("default")
        };
        let direction = if current {
            Self::light(g, light, hour)
        } else {
            self.load(bank_source)
                .and_then(|bank| Self::light(bank, light, hour))
        };
        // 168e10 falls back to bank 1 / light 0, never the selected collision sun.
        let direction =
            direction.or_else(|| self.load("default").and_then(|g| Self::light(g, 0, hour)));
        if direction.is_none() {
            warn!(
                "fog bank {bank}, light {light}: no authored direction in {bank_source} or default bank; using native dummy light"
            );
        }
        // 30e440's dummy vector, mirrored into our world coordinates.
        let axis = f32::from_bits(0x3f350481);
        let direction = direction.unwrap_or(Vec3::new(-axis, axis, 0.));
        info!(
            "fog light: bank={bank}, light={light}, source={bank_source}, direction={direction:?}"
        );
        direction
    }

    fn light(g: &Gparam, id: i32, hour: f32) -> Option<Vec3> {
        let param = g.param("LightSet", "Directional Light Angle0")?;
        if !param.entries.iter().any(|entry| entry.id == id) {
            return None;
        }
        let (a, b, blend) = fog_bounds(param, id, hour)?;
        let toward = |entry: &sekiro_formats::gparam::Entry| {
            let angle = entry.value.floats();
            if angle.len() < 2 || !angle[..2].iter().all(|v| v.is_finite()) {
                return None;
            }
            let direction =
                Quat::from_rotation_y(angle[1]) * Quat::from_rotation_x(angle[0]) * Vec3::Z;
            Some(Vec3::new(-direction.x, direction.y, direction.z))
        };
        // 310630 uploads the linear blend of the two derived LightSet directions.
        Some(toward(a)?.lerp(toward(b)?, blend))
    }

    fn load(&mut self, source: &str) -> Option<&Gparam> {
        self.0
            .entry(source.into())
            .or_insert_with(|| {
                let bytes = std::fs::read(
                    sekiro_sim::cache::root().join(format!("raw/param/drawparam/{source}.gparam")),
                )
                .ok()?;
                sekiro_formats::gparam::parse(&bytes).ok()
            })
            .as_ref()
    }
}

#[derive(Clone)]
struct Background {
    control: Vec4,
    rings: [Vec4; 16],
}

impl Background {
    fn from_positions(positions: &[[f32; 3]]) -> Option<Self> {
        // Native sky meshes are regular polygon rings, including a planar cap in night skies.
        let mut vertices: Vec<_> = positions.iter().map(|&p| Vec3::from_array(p)).collect();
        if vertices.len() < 16 || vertices.iter().any(|p| !p.is_finite()) {
            return None;
        }
        vertices.sort_by(|a, b| a.y.total_cmp(&b.y));
        let mut rings: Vec<(f32, f32)> = Vec::new();
        for vertex in &vertices {
            let radius = vertex.xz().length();
            if let Some(last) = rings.last_mut()
                && (last.1 - vertex.y).abs() < 0.01
            {
                last.0 = last.0.max(radius);
            } else {
                rings.push((radius, vertex.y));
            }
        }
        let top = *rings.last()?;
        if top.0 > 0.02
            && vertices
                .iter()
                .any(|v| (v.y - top.1).abs() < 0.01 && v.xz().length() < 0.02)
        {
            rings.push((0., top.1));
        }
        if rings.len() < 2 || rings.len() > 16 {
            return None;
        }
        let widest = rings.iter().max_by(|a, b| a.0.total_cmp(&b.0))?;
        let mut angles: Vec<_> = vertices
            .iter()
            .filter(|p| (p.y - widest.1).abs() < 0.01 && (p.xz().length() - widest.0).abs() < 0.02)
            .map(|p| p.z.atan2(p.x))
            .collect();
        angles.sort_by(f32::total_cmp);
        angles.dedup_by(|a, b| (*a - *b).abs() < 0.00001);
        if angles.last()? - angles[0] > std::f32::consts::TAU - 0.00001 {
            angles.pop();
        }
        if angles.len() < 3 {
            return None;
        }
        let step = std::f32::consts::TAU / angles.len() as f32;
        let phase = angles[0];
        if angles
            .iter()
            .enumerate()
            .any(|(i, angle)| (angle - phase - i as f32 * step).abs() > 0.00001)
        {
            return None;
        }
        let mut result = Self {
            control: Vec4::new(rings.len() as f32, step, phase, 0.),
            rings: [Vec4::ZERO; 16],
        };
        for (target, (radius, height)) in result.rings.iter_mut().zip(&rings) {
            *target = Vec4::new(radius * (0.5 * step).cos(), *height, 0., 0.);
        }
        // Validate all rings against the same polygon grid; only cap interiors may have smaller radii.
        for vertex in &vertices {
            let radius = vertex.xz().length();
            let ring = rings
                .iter()
                .find(|(_, height)| (vertex.y - height).abs() < 0.01)?;
            if radius < 0.02 {
                continue;
            }
            let angular_error = ((vertex.z.atan2(vertex.x) - phase) / step).rem_euclid(1.);
            if angular_error.min(1. - angular_error) * step > 0.00001
                || ((radius - ring.0).abs() > 0.02 && (ring.1 - rings.last()?.1).abs() > 0.01)
            {
                return None;
            }
        }
        Some(result)
    }
}

type BackgroundMeshes<'w, 's> = Query<
    'w,
    's,
    (&'static Mesh3d, &'static Visibility),
    (
        With<NoFrustumCulling>,
        With<NotShadowCaster>,
        With<NotShadowReceiver>,
    ),
>;

/// Whether fog sun in-scatter may follow the sun shadows (#345), fixed at launch.
#[derive(Resource, Clone, Copy, ExtractResource)]
#[extract_app(bevy::render::RenderApp)]
pub(crate) struct FogShadows(pub bool);
impl FogShadows {
    fn from_args(args: &[String]) -> Self {
        // `--no-fog-shadow` runs the game's plain WriteInScattering, for A/B checks.
        // The gates run plain fog too: the render gate's pop-in metric compares a normal pass with
        // an all-visible pass, where hidden draw groups cast extra sun shadows into the fog over
        // sky and haze and read as pop-in. Pop-in is a geometry check.
        Self(!args.iter().any(|a| a == "--no-fog-shadow") && !crate::sweep_mode(args))
    }
}

pub struct FogPlugin;
impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fog.wesl");
        let args: Vec<String> = std::env::args().collect();
        app.insert_resource(FogShadows::from_args(&args))
            .add_plugins((
                ExtractComponentPlugin::<GameFog>::default(),
                UniformComponentPlugin::<GameFog>::default(),
                ExtractResourcePlugin::<FogShadows>::default(),
            ))
            .add_systems(Update, configure.after(super::apply_lighting))
            .add_systems(PostUpdate, update_background);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(Render, shadow_light.in_set(RenderSystems::Queue))
                .add_systems(
                    Core3d,
                    fog.before(Core3dSystems::EarlyPostProcess)
                        .after(Core3dSystems::MainPass),
                );
        }
    }
}

/// Light set, hour, fog id, gparam source and lighting revision of the last configure.
type ConfigureKey = (i32, f32, i32, String, u32);

fn configure(
    mut commands: Commands,
    lighting: Res<super::MapLighting>,
    cameras: Query<Entity, With<Camera3d>>,
    mut last: Local<Option<ConfigureKey>>,
    added: Query<(), Added<Camera3d>>,
    selection: Res<FogSelection>,
    mut banks: Local<FogBanks>,
) {
    let key = (lighting.light_set, lighting.hour);
    let unchanged = last
        .as_ref()
        .is_some_and(|(set, hour, fog, source, revision)| {
            (*set, *hour, *fog, *revision) == (key.0, key.1, selection.0, lighting.revision)
                && source == &lighting.source
        });
    if unchanged && added.is_empty() {
        return;
    }
    if !unchanged {
        *last = Some((
            key.0,
            key.1,
            selection.0,
            lighting.source.clone(),
            lighting.revision,
        ));
    }
    // Preserve the independent MSB FogParamID. No separate Fog formula was recovered.
    // Inspect selected authored rows rather than silently aliasing this ID to LightSetID.
    let legacy: Vec<_> = lighting
        .gparam
        .groups
        .iter()
        .filter(|g| g.name.contains("Fog") && !g.name.contains("VolumetricFog"))
        .flat_map(|g| g.params.iter().map(|p| (&p.name, p.at(selection.0, key.1))))
        .collect();
    // Logged once a lighting fade settles, not on every blended frame.
    let settled = lighting.fade.is_none();
    if settled {
        info!(
            "MSB FogParamID {}: separate Fog rows {legacy:?}",
            selection.0
        );
    }
    let direction = banks.direction(&lighting.gparam, &lighting.source, key.0, key.1);
    let value = (!std::env::args().any(|a| a == "--no-fog"))
        .then(|| GameFog::from_gparam(&lighting.gparam, key.0, key.1, direction))
        .flatten();
    if settled {
        info!("shader fog: {value:?}");
    }
    // The depth the pass samples is switched on and off by `depth_sampling` for all readers.
    for entity in &cameras {
        if let Some(value) = &value {
            commands.entity(entity).insert(value.clone());
        } else {
            commands.entity(entity).remove::<GameFog>();
        }
    }
}

fn update_background(
    meshes: Option<Res<Assets<Mesh>>>,
    background_meshes: BackgroundMeshes,
    mut backgrounds: Local<
        std::collections::HashMap<bevy::asset::AssetId<Mesh>, Option<Background>>,
    >,
    mut cameras: Query<(&mut GameFog, Option<&Exposure>)>,
) {
    // Sky's positions/UV-only mesh layout and shadow/visibility markers identify its live dome.
    // Follow the selected mesh handle instead of reproducing the sky/event selector here.
    let background = meshes.as_ref().and_then(|meshes| {
        background_meshes
            .iter()
            .filter(|(_, visibility)| **visibility != Visibility::Hidden)
            .filter_map(|(handle, _)| {
                let mesh = meshes.get(&handle.0)?;
                if mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL)
                    || !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1)
                {
                    return None;
                }
                let profile = backgrounds
                    .entry(handle.0.id())
                    .or_insert_with(|| {
                        let VertexAttributeValues::Float32x3(positions) =
                            mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
                        else {
                            return None;
                        };
                        Background::from_positions(positions)
                    })
                    .as_ref()?;
                Some((handle.0.id(), profile.clone()))
            })
            .max_by(|a, b| a.1.control.x.total_cmp(&b.1.control.x))
    });
    // Run after Update's sky swaps, without reinserting fog on sweep diagnostic cameras.
    for (mut value, exposure) in &mut cameras {
        // Same radiance bridge as the sky and diffuse accumulator, applied once.
        let radiance_scale = super::ENV_PER_UNIT * exposure.copied().unwrap_or_default().exposure();
        let control = background.as_ref().map_or(Vec4::ZERO, |(_, b)| b.control);
        let rings = background
            .as_ref()
            .map_or([Vec4::ZERO; 16], |(_, b)| b.rings);
        if value.background_control != control
            || value.background != rings
            || value.radiance_scale.x != radiance_scale
        {
            value.background_control = control;
            value.background = rings;
            value.radiance_scale.x = radiance_scale;
            if let Some((id, background)) = &background {
                info!(
                    "fog background: mesh={id:?}, rings={}, sectors={}, outline={:?}",
                    background.control.x,
                    std::f32::consts::TAU / background.control.y,
                    &background.rings[..background.control.x as usize]
                );
            }
            info!("shader fog: {value:?}");
        }
    }
}

#[derive(Resource)]
pub(crate) struct FogPipeline {
    /// Per MSAA sample count: the plain WriteInScattering form and the FOG_SHADOW form.
    variants: Vec<Option<[FogVariant; 2]>>,
    // DirLightShadow: the per-cell sun shadow volume the FOG_SHADOW variants read (#345).
    shadow_layout: BindGroupLayoutDescriptor,
    shadow: CachedComputePipelineId,
    volume_sampler: Sampler,
}
struct FogVariant {
    layout: BindGroupLayoutDescriptor,
    pipelines: Vec<CachedRenderPipelineId>,
}
/// The compiled pipelines one fog frame uses.
struct ReadyFog<'a> {
    variant: &'a FogVariant,
    psos: Vec<&'a RenderPipeline>,
    /// The DirLightShadow pass; `None` runs the plain form.
    shadow: Option<&'a ComputePipeline>,
}
impl FogPipeline {
    /// The shadowed set when it is wanted and fully compiled, otherwise the plain set, so fog
    /// never waits on the shadow pipelines.
    fn ready<'a>(
        &'a self,
        cache: &'a PipelineCache,
        samples: u32,
        shadowed: bool,
    ) -> Option<ReadyFog<'a>> {
        let [plain, shadow_variant] = self.variants.get(samples.ilog2() as usize)?.as_ref()?;
        let psos = |variant: &'a FogVariant| {
            variant
                .pipelines
                .iter()
                .map(|id| cache.get_render_pipeline(*id))
                .collect::<Option<Vec<_>>>()
        };
        if shadowed
            && let Some(shadow) = cache.get_compute_pipeline(self.shadow)
            && let Some(psos) = psos(shadow_variant)
        {
            return Some(ReadyFog {
                variant: shadow_variant,
                psos,
                shadow: Some(shadow),
            });
        }
        Some(ReadyFog {
            variant: plain,
            psos: psos(plain)?,
            shadow: None,
        })
    }
}
fn init_pipeline(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
    cache: Res<PipelineCache>,
    adapter: Res<bevy::render::renderer::RenderAdapter>,
    device: Res<bevy::render::renderer::RenderDevice>,
) {
    let supported = adapter
        .get_texture_format_features(TextureFormat::Rgba16Float)
        .flags;
    let variant = |samples: u32, shadowed: bool| {
        let multisampled = samples > 1;
        let mut entries = if multisampled {
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
        };
        if shadowed {
            let next = entries.len() as u32;
            entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (
                        next,
                        texture_3d(TextureSampleType::Float { filterable: true }),
                    ),
                    (next + 1, sampler(SamplerBindingType::Filtering)),
                ),
            ));
        }
        let layout = BindGroupLayoutDescriptor::new("game fog", &entries);
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
                if shadowed {
                    shader_defs.push("FOG_SHADOW".into());
                }
                cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("Sekiro global fog".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen.to_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: load_embedded_asset!(assets.as_ref(), "fog.wesl"),
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
        FogVariant { layout, pipelines }
    };
    let variants = [1, 2, 4, 8]
        .into_iter()
        .map(|samples| {
            supported
                .sample_count_supported(samples)
                .then(|| [variant(samples, false), variant(samples, true)])
        })
        .collect();
    let shadow_layout = BindGroupLayoutDescriptor::new(
        "game fog shadow volume",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<GameFog>(true),
                uniform_buffer::<GpuLights>(true),
                texture_2d_array(TextureSampleType::Depth),
                sampler(SamplerBindingType::Comparison),
                texture_storage_3d(SHADOW_VOLUME_FORMAT, StorageTextureAccess::WriteOnly),
                uniform_buffer::<ShadowFrame>(false),
                texture_3d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let shadow = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("Sekiro fog DirLightShadow".into()),
        layout: vec![shadow_layout.clone()],
        shader: load_embedded_asset!(assets.as_ref(), "fog.wesl"),
        shader_defs: vec!["SHADOW_VOLUME".into()],
        entry_point: Some("shadow_volume_cells".into()),
        ..default()
    });
    let volume_sampler = device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    commands.insert_resource(FogPipeline {
        variants,
        shadow_layout,
        shadow,
        volume_sampler,
    });
}
/// Rgba16Float is the smallest filterable format WebGPU also allows as a storage texture; each
/// texel packs four consecutive slices' shadow means into rgba.
const SHADOW_VOLUME_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// Froxel grid size after `3a2be0` (inferred, docs/SHADER-INPUTS.md): Y is VolumeWidth, X is
/// Y*aspect, both capped at a quarter of the viewport and at 640 and rounded up to a multiple of 8.
/// Depth is one texel per four slices.
fn shadow_volume_size(target: UVec2, volume_width: f32, slices: f32) -> Extent3d {
    let target = target.max(UVec2::ONE);
    let axis = |n: f32, viewport: u32| {
        let cap = (viewport / 4).clamp(1, 640) as f32;
        (n.min(cap).ceil() as u32).next_multiple_of(8)
    };
    let height = axis(volume_width, target.y);
    Extent3d {
        width: axis(height as f32 * target.x as f32 / target.y as f32, target.x),
        height,
        depth_or_array_layers: (slices as u32).div_ceil(4).max(1),
    }
}
/// One frame of `GXVolumetricFog_DirLightShadowReproj`: last frame's view for the reprojection
/// (native mtxNowToPrevView and PrevLogDepthRangeInfo), this frame's ShadowSubpixelOffset and the
/// history weight (ReprojectionFeedback, zero when there is no valid history).
#[derive(ShaderType, Clone, Copy, Default)]
struct ShadowFrame {
    prev_clip_from_world: Mat4,
    prev_view_from_world: Mat4,
    // Cell-space ray position xy (0..1 across the cell), dither offset, history weight.
    jitter: Vec4,
}
/// A fog camera's two shadow volumes: the DirLightShadowReproj pass reads last frame's and writes
/// the other, which the fog then reads.
pub(crate) struct ShadowHistory {
    size: Extent3d,
    volumes: [TextureView; 2],
    frame: UniformBuffer<ShadowFrame>,
    /// The volume written last.
    last: usize,
    /// FrameCount and views of the last write.
    written: (u32, Mat4, Mat4),
    /// Frames written so far, which steps the jitter sequence.
    writes: u32,
}
/// ShadowSubpixelOffset for the `index`th shadow frame: the cell's ray position in xy and the
/// dither offset, which cycle over JitterCount frames so the history averages them.
/// 0x14039fa70 adds per-frame offsets to a constant; neither was traced, so this is inferred: a
/// rank-1 lattice of JitterCount points (one per row and column stratum, centred on the cell
/// centre and scaled by JitterXYSize), and dither offsets k/JitterCount.
fn shadow_jitter(index: u32, count: u32, xy_size: f32) -> Vec3 {
    let n = count.max(1);
    let k = index % n;
    // The generator nearest the golden ratio of n that is coprime with n spreads the points.
    let gcd = |mut a: u32, mut b: u32| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let g = ((n as f32 * 0.618).round() as u32..)
        .find(|&g| gcd(g, n) == 1)
        .unwrap_or(1);
    let stratum = |i: u32| (i as f32 + 0.5) / n as f32 - 0.5;
    Vec3::new(
        0.5 + stratum(k) * xy_size,
        0.5 + stratum(k * g % n) * xy_size,
        k as f32 / n as f32,
    )
}
/// The history weight: FeedbackWeight when reprojection is on and last frame wrote a volume of
/// this size, else zero (the first frame, after a resize or after frames of plain fog).
fn history_weight(
    reproj: Vec4,
    history: Option<(u32, Extent3d)>,
    frame: u32,
    size: Extent3d,
) -> f32 {
    let valid = history.is_some_and(|(written, s)| written.wrapping_add(1) == frame && s == size);
    if reproj.w > 0.5 && valid {
        reproj.z
    } else {
        0.
    }
}
/// The GpuLights index of the first shadow-casting sun in the order bevy_pbr's `prepare_lights`
/// fills GpuLights: sorted by (volumetric, shadow maps, entity), filtered to the camera's render
/// layers and capped at MAX_DIRECTIONAL_LIGHTS. Each light is its sort key, whether it shares the
/// camera's layers and whether it casts cascaded shadows.
fn shadow_light_index(mut lights: Vec<((bool, bool, Entity), bool, bool)>) -> Option<usize> {
    lights.sort_by_key(|(key, ..)| *key);
    lights
        .iter()
        .filter(|(_, visible, _)| *visible)
        .take(MAX_DIRECTIONAL_LIGHTS)
        .position(|(_, _, casts)| *casts)
}
/// Stores each fog camera's shadow-casting sun index in `shadow.w` (-1 for none) before the fog
/// uniforms upload, so the DirLightShadow pass does not search the lights per thread.
fn shadow_light(
    lights: Query<(Entity, &ExtractedDirectionalLight, &RenderLayers)>,
    mut views: Query<(&mut GameFog, Option<&RenderLayers>)>,
) {
    for (mut fog, view_layers) in &mut views {
        let view_layers = view_layers.cloned().unwrap_or_default();
        let index = shadow_light_index(
            lights
                .iter()
                .map(|(entity, light, layers)| {
                    (
                        (light.volumetric, light.shadow_maps_enabled, entity),
                        layers.intersects(&view_layers),
                        light.shadow_maps_enabled && !light.cascade_shadow_config.bounds.is_empty(),
                    )
                })
                .collect(),
        );
        fog.shadow.w = index.map_or(-1., |i| i as f32);
    }
}
/// The fragment bindings, plus the shadow volume and its sampler for the FOG_SHADOW variants.
fn with_volume<'a>(
    base: &[BindGroupEntry<'a>],
    volume: Option<(&'a TextureView, &'a Sampler)>,
) -> Vec<BindGroupEntry<'a>> {
    let mut entries = base.to_vec();
    if let Some((view, sampler)) = volume {
        let next = entries.len() as u32;
        entries.extend_from_slice(&BindGroupEntries::with_indices((
            (next, view),
            (next + 1, sampler),
        )));
    }
    entries
}
/// The camera's targets and uniform offsets, and the sun shadow maps when it has them.
type FogView = (
    &'static GameFog,
    &'static ViewTarget,
    &'static ViewDepthStencilTexture,
    &'static Msaa,
    &'static ViewUniformOffset,
    &'static DynamicUniformIndex<GameFog>,
    Option<&'static ViewShadowBindings>,
    Option<&'static ViewLightsUniformOffset>,
    &'static ExtractedView,
);
/// The sun shadow and per-frame resources the fog node reads.
type FogFrame<'w> = (
    Res<'w, LightMeta>,
    Res<'w, ShadowSamplers>,
    Res<'w, FogShadows>,
    Res<'w, RenderQueue>,
    Res<'w, FrameCount>,
);
pub(crate) fn fog(
    view: ViewQuery<FogView>,
    pipeline: Res<FogPipeline>,
    cache: Res<PipelineCache>,
    (views, uniforms): (Res<ViewUniforms>, Res<ComponentUniforms<GameFog>>),
    (light_meta, shadow_samplers, fog_shadows, queue, frame_count): FogFrame,
    mut ctx: RenderContext,
    (mut warned_depth, mut histories): (Local<bool>, Local<EntityHashMap<ShadowHistory>>),
) {
    let view_entity = view.entity();
    let (settings, target, depth, msaa, offset, index, shadows, lights_offset, extracted) =
        view.into_inner();
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        if !*warned_depth {
            warn!("global fog skipped: depth-only view is unavailable");
            *warned_depth = true;
        }
        return;
    };
    // A newly extracted fog component can precede its rebuilt depth attachment.
    if !depth
        .texture()
        .usage()
        .contains(TextureUsages::TEXTURE_BINDING)
    {
        return;
    }
    let (Some(view_binding), Some(fog_binding)) =
        (views.uniforms.binding(), uniforms.uniforms().binding())
    else {
        return;
    };
    if target.main_texture_format() != TextureFormat::Rgba16Float {
        return;
    }
    // Variant pick 0x1403a1d10: the shadowed form only when the frame has a sun shadow map.
    let sun = (fog_shadows.0 && settings.shadow.x > 0. && settings.shadow.w >= 0.)
        .then_some(shadows.zip(lights_offset))
        .flatten()
        .zip(light_meta.view_gpu_lights.binding());
    let Some(ready) = pipeline.ready(&cache, msaa.samples(), sun.is_some()) else {
        return;
    };
    let offsets = [
        offset.offset,
        index.index(),
        lights_offset.map_or(0, |l| l.offset),
    ];
    use bevy::render::diagnostic::RecordDiagnostics;
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let span = diagnostics.time_span(ctx.command_encoder(), "game_fog");
    // DirLightShadowReproj: the sun cascades the map shading uses, tapped once per froxel cell at
    // a per-frame jittered position and blended with last frame's volume (#345).
    let frame = frame_count.0;
    let mut volume = None;
    if let (Some(shadow_pso), Some(((shadows, _), lights_binding))) = (ready.shadow, sun) {
        let device = ctx.render_device().clone();
        let size = shadow_volume_size(
            UVec2::new(
                target.main_texture().width(),
                target.main_texture().height(),
            ),
            settings.shadow.z,
            settings.slices.y,
        );
        let world_from_view = extracted.world_from_view.to_matrix();
        let view_from_world = world_from_view.inverse();
        let clip_from_world = extracted
            .clip_from_world
            .unwrap_or(extracted.clip_from_view * view_from_world);
        // Forget cameras that stopped drawing shadowed fog.
        histories.retain(|_, h| frame.wrapping_sub(h.written.0) < 600);
        let previous = histories.get(&view_entity).map(|h| (h.written.0, h.size));
        let weight = history_weight(settings.reproj, previous, frame, size);
        if previous.is_none_or(|(_, s)| s != size) {
            let new_volume = || {
                device
                    .create_texture(&TextureDescriptor {
                        label: Some("game fog shadow volume"),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D3,
                        format: SHADOW_VOLUME_FORMAT,
                        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&TextureViewDescriptor::default())
            };
            histories.insert(
                view_entity,
                ShadowHistory {
                    size,
                    volumes: [new_volume(), new_volume()],
                    frame: UniformBuffer::default(),
                    last: 0,
                    written: (frame, Mat4::IDENTITY, Mat4::IDENTITY),
                    writes: 0,
                },
            );
        }
        let history = histories.get_mut(&view_entity).expect("inserted above");
        let reproj = settings.reproj.w > 0.5;
        let jitter = if reproj {
            shadow_jitter(history.writes, settings.reproj.x as u32, settings.reproj.y)
        } else {
            Vec3::new(0.5, 0.5, 0.)
        };
        history.frame.set(ShadowFrame {
            prev_clip_from_world: history.written.1,
            prev_view_from_world: history.written.2,
            jitter: jitter.extend(weight),
        });
        history.frame.write_buffer(&device, &queue);
        let (read, write) = (history.last, 1 - history.last);
        let bindings = device.create_bind_group(
            "game fog shadow volume",
            &cache.get_bind_group_layout(&pipeline.shadow_layout),
            &BindGroupEntries::sequential((
                view_binding.clone(),
                fog_binding.clone(),
                lights_binding,
                &shadows.directional_light_depth_texture_view,
                &shadow_samplers.directional_light_comparison_sampler,
                &history.volumes[write],
                history.frame.binding().expect("written above"),
                &history.volumes[read],
                &pipeline.volume_sampler,
            )),
        );
        let mut pass = ctx
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("game fog shadow volume"),
                ..default()
            });
        pass.set_pipeline(shadow_pso);
        pass.set_bind_group(0, &bindings, &offsets);
        pass.dispatch_workgroups(
            size.width.div_ceil(8),
            size.height.div_ceil(8),
            size.depth_or_array_layers,
        );
        drop(pass);
        history.last = write;
        history.written = (frame, clip_from_world, view_from_world);
        history.writes = history.writes.wrapping_add(1);
        volume = Some(write);
    }
    let volume = volume.map(|write| {
        (
            &histories[&view_entity].volumes[write],
            &pipeline.volume_sampler,
        )
    });
    let layout = cache.get_bind_group_layout(&ready.variant.layout);
    if msaa.samples() > 1 {
        // Preserve per-sample colour/depth correspondence through attenuation and
        // inscattering, then let the existing main attachment resolve the result.
        let bindings = ctx.render_device().create_bind_group(
            "game fog samples",
            &layout,
            &with_volume(
                &BindGroupEntries::sequential((depth_view, view_binding, fog_binding)),
                volume,
            ),
        );
        for pso in ready.psos {
            let mut attachment = target.get_color_attachment();
            attachment.ops.load = LoadOp::Load;
            let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("game fog samples"),
                color_attachments: &[Some(attachment)],
                ..default()
            });
            pass.set_render_pipeline(pso);
            pass.set_bind_group(0, &bindings, &offsets[..2]);
            pass.draw(0..3, 0..1);
        }
        span.end(ctx.command_encoder());
        return;
    }
    let post = target.post_process_write();
    let bindings = ctx.render_device().create_bind_group(
        "game fog",
        &layout,
        &with_volume(
            &BindGroupEntries::sequential((post.source, depth_view, view_binding, fog_binding)),
            volume,
        ),
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
    pass.set_render_pipeline(ready.psos[0]);
    pass.set_bind_group(0, &bindings, &offsets[..2]);
    pass.draw(0..3, 0..1);
    drop(pass);
    span.end(ctx.command_encoder());
}
