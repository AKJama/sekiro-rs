//! Playtest A/B switch (#358): flip variants live in one run instead of relaunching.
//!
//! `--ab <name>=<v1>,<v2>[,...]` (repeatable) lists the variants of one knob; [`KEY`] steps
//! through every combination of the listed knobs, and a small label (shown even with the F3 debug
//! text off) names the one showing. The first values are in force from the start.
//! A new knob is one entry in [`KNOBS`].

use bevy::prelude::*;

/// Steps to the next variant combination.
pub const KEY: KeyCode = KeyCode::F5;

/// A live-switchable setting: how one `--ab` value reads, and how it is put in force.
struct Knob {
    name: &'static str,
    parse: fn(&str) -> Option<f32>,
    apply: fn(&mut World, f32),
}

/// The knobs `--ab` accepts; adding a variant is one line here.
const KNOBS: &[Knob] = &[
    Knob {
        name: "hour",
        parse: number,
        apply: set_hour,
    },
    Knob {
        name: "fog-shadow",
        parse: on_off,
        apply: |world, v| {
            let on = v > 0.5;
            if world
                .get_resource::<crate::lighting::fog::FogShadows>()
                .map(|f| f.0)
                != Some(on)
            {
                world.insert_resource(crate::lighting::fog::FogShadows(on));
            }
        },
    },
];

fn number(v: &str) -> Option<f32> {
    v.parse().ok().filter(|v: &f32| v.is_finite())
}

/// `on`/`off` as 1/0.
fn on_off(v: &str) -> Option<f32> {
    match v {
        "on" => Some(1.),
        "off" => Some(0.),
        _ => None,
    }
}

/// Moves the clock as `--hour` would at load: the light set, sun, shadows, fog, post, sky and
/// every other user keyed on [`crate::lighting::MapLighting::hour`] rebuild, and the probes follow
/// the hour unless the map's environment is fixed.
fn set_hour(world: &mut World, hour: f32) {
    let environment = world
        .get_resource::<crate::lighting_selection::LightingSelection>()
        .map(|s| s.environment_for_hour(hour));
    if let Some(mut light) = world.get_resource_mut::<crate::lighting::MapLighting>() {
        light.set_hour(hour);
    }
    if let (Some(variant), Some(mut probes)) = (
        environment,
        world.get_resource_mut::<crate::lighting::MapProbes>(),
    ) {
        probes.set_variant(variant);
    }
}

/// One `--ab` knob and its variants, as written and as parsed.
#[derive(Debug, Clone, PartialEq)]
struct Axis {
    knob: usize,
    values: Vec<(String, f32)>,
}

/// The `--ab` variants of a run and the combination showing.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct AbSwitch {
    axes: Vec<Axis>,
    index: usize,
    /// Whether the combination showing is in force; false until the first frame applies it.
    applied: bool,
}

impl AbSwitch {
    pub fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut axes: Vec<Axis> = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            if arg != "--ab" {
                continue;
            }
            let spec = args
                .get(i + 1)
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| anyhow::anyhow!("--ab wants <name>=<v1>,<v2>[,...]"))?;
            let (name, values) = spec
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--ab {spec}: wants <name>=<v1>,<v2>[,...]"))?;
            let knob = KNOBS.iter().position(|k| k.name == name).ok_or_else(|| {
                let known: Vec<_> = KNOBS.iter().map(|k| k.name).collect();
                anyhow::anyhow!("--ab {name}: unknown, known: {}", known.join(", "))
            })?;
            anyhow::ensure!(
                axes.iter().all(|a| a.knob != knob),
                "--ab {name} given twice"
            );
            let values = values
                .split(',')
                .map(|v| {
                    (KNOBS[knob].parse)(v)
                        .map(|parsed| (v.to_string(), parsed))
                        .ok_or_else(|| anyhow::anyhow!("--ab {name}: bad value {v:?}"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            anyhow::ensure!(values.len() >= 2, "--ab {name} wants at least two values");
            axes.push(Axis { knob, values });
        }
        Ok(Self { axes, ..default() })
    }

    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// The number of combinations.
    fn len(&self) -> usize {
        self.axes.iter().map(|a| a.values.len()).product()
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.len();
        self.applied = false;
    }

    /// The value of every knob in the combination showing; the first knob changes fastest.
    fn picks(&self) -> impl Iterator<Item = (&'static Knob, &(String, f32))> {
        let mut rest = self.index;
        self.axes.iter().map(move |a| {
            let pick = &a.values[rest % a.values.len()];
            rest /= a.values.len();
            (&KNOBS[a.knob], pick)
        })
    }

    /// The first value of `name`, which is in force from the start.
    pub fn first(&self, name: &str) -> Option<f32> {
        self.axes
            .iter()
            .find(|a| KNOBS[a.knob].name == name)
            .map(|a| a.values[0].1)
    }

    pub fn label(&self) -> String {
        let picks: Vec<_> = self
            .picks()
            .map(|(knob, (text, _))| format!("{} {text}", knob.name))
            .collect();
        format!(
            "A/B: {} ({}/{})",
            picks.join(", "),
            self.index + 1,
            self.len()
        )
    }
}

/// Whether the run sets the clock itself (`--hour`, or `--ab hour=...`), so story events and
/// arena story hours leave it alone.
pub fn explicit_hour(args: &[String]) -> bool {
    args.iter().any(|a| a == "--hour")
        || args
            .windows(2)
            .any(|w| w[0] == "--ab" && w[1].starts_with("hour="))
}

#[derive(Component)]
struct Label;

/// Adds the switch when the command line lists `--ab` variants.
pub fn install(app: &mut App, args: &[String]) -> anyhow::Result<()> {
    let switch = AbSwitch::parse(args)?;
    if switch.is_empty() {
        return Ok(());
    }
    app.insert_resource(switch)
        .add_systems(Startup, spawn_label)
        .add_systems(Update, step.run_if(switching));
    Ok(())
}

/// The text is written by [`step`] on the first frame.
fn spawn_label(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                Label,
                Text::default(),
                TextFont {
                    font_size: bevy::text::FontSize::Px(14.),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.85, 0.55)),
                BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
                Node {
                    padding: UiRect::axes(px(8), px(3)),
                    ..default()
                },
            ));
        });
}

/// Whether [`step`] has work: the first frame, or a [`KEY`] press. Keeps the exclusive system off
/// every other frame.
fn switching(keys: Option<Res<ButtonInput<KeyCode>>>, switch: Res<AbSwitch>) -> bool {
    !switch.applied || keys.is_some_and(|keys| keys.just_pressed(KEY))
}

/// Puts the start combination in force on the first frame, and the next one on each [`KEY`].
fn step(world: &mut World) {
    let mut switch = world.resource_mut::<AbSwitch>();
    if switch.applied {
        switch.next();
    }
    switch.applied = true;
    let picks: Vec<_> = switch.picks().map(|(k, (_, v))| (k.apply, *v)).collect();
    let label = switch.label();
    for (apply, value) in picks {
        apply(world, value);
    }
    info!("{label}");
    let mut texts = world.query_filtered::<&mut Text, With<Label>>();
    for mut text in texts.iter_mut(world) {
        text.0.clone_from(&label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("sekiro-rs")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn parses_and_cycles_every_combination() {
        let mut ab = AbSwitch::parse(&args(&["--play", "--ab", "hour=6,6.25,18"])).unwrap();
        assert_eq!(ab.first("hour"), Some(6.));
        assert_eq!(ab.label(), "A/B: hour 6 (1/3)");
        ab.next();
        assert_eq!(ab.label(), "A/B: hour 6.25 (2/3)");
        ab.next();
        ab.next();
        assert_eq!(ab.label(), "A/B: hour 6 (1/3)", "wraps to the start");
        assert!(explicit_hour(&args(&["--ab", "hour=6,7"])));
        assert!(!explicit_hour(&args(&["--play"])));
        assert!(AbSwitch::parse(&args(&["--play"])).unwrap().is_empty());
        for bad in [
            &["--ab"][..],
            &["--ab", "hour"],
            &["--ab", "hour=6"],
            &["--ab", "hour=6,dusk"],
            &["--ab", "weather=1,2"],
            &["--ab", "hour=6,7", "--ab", "hour=8,9"],
        ] {
            assert!(AbSwitch::parse(&args(bad)).is_err(), "{bad:?}");
        }
        assert_eq!(on_off("on"), Some(1.));
        assert_eq!(on_off("maybe"), None);
    }

    /// The fog-shadow knob turns the per-frame fog shadow switch on and off.
    #[test]
    fn fog_shadow_knob_sets_the_switch() {
        use crate::lighting::fog::FogShadows;
        let knob = KNOBS.iter().find(|k| k.name == "fog-shadow").unwrap();
        let mut world = World::new();
        for (value, on) in [("off", false), ("on", true)] {
            (knob.apply)(&mut world, (knob.parse)(value).unwrap());
            assert_eq!(world.resource::<FogShadows>().0, on);
        }
    }

    /// Two knobs step as a counter, the first changing fastest.
    #[test]
    fn several_knobs_step_through_their_product() {
        let mut ab = AbSwitch {
            axes: vec![
                Axis {
                    knob: 0,
                    values: vec![("6".into(), 6.), ("7".into(), 7.)],
                },
                Axis {
                    knob: 0,
                    values: vec![("a".into(), 0.), ("b".into(), 1.), ("c".into(), 2.)],
                },
            ],
            ..default()
        };
        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(ab.picks().map(|(_, (t, _))| t.clone()).collect::<String>());
            ab.next();
        }
        assert_eq!(seen, ["6a", "7a", "6b", "7b", "6c", "7c"]);
        assert_eq!(ab.index, 0);
    }

    /// F5 moves the Outskirts sun from 6 h to 18 h in the running app: light set, sun direction
    /// and colour all follow, with no reload.
    #[test]
    fn switching_hour_moves_the_sun_live() {
        use crate::lighting::{FillLight, LightSet, MapLighting, Sun, apply_lighting};
        use crate::lighting_selection::{LightingSelection, select};
        let cache = sekiro_sim::cache::root();
        let map = "m11_00_00_00";
        let mut selected = select(&cache, map, None, None).unwrap();
        selected.override_hour(6., true);
        let library = LightingSelection::load(&cache, map, selected, false).unwrap();
        let gparam = library.initial_gparam();
        let mut app = App::new();
        app.insert_resource(MapLighting {
            gparam: gparam.clone(),
            hour: 6.,
            light_set: 100,
            source: library.initial.source.clone(),
            ..default()
        })
        .insert_resource(library)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<GlobalAmbientLight>()
        .init_resource::<ClearColor>()
        .insert_resource(AbSwitch::parse(&args(&["--ab", "hour=6,18"])).unwrap())
        .add_systems(Update, (step.run_if(switching), apply_lighting).chain());
        let sun = app
            .world_mut()
            .spawn((Sun, DirectionalLight::default(), Transform::default()))
            .id();
        app.world_mut()
            .spawn((FillLight, DirectionalLight::default(), Transform::default()));
        let shown = |app: &App| {
            let light = app.world().get::<DirectionalLight>(sun).unwrap().color;
            let toward = -app
                .world()
                .get::<Transform>(sun)
                .unwrap()
                .forward()
                .as_vec3();
            (light, toward)
        };
        let expect = |hour| {
            let set = LightSet::from_gparam(&gparam, 100, hour);
            (set.sun_color, set.sun_toward)
        };
        let close = |(c, t): (Color, Vec3), (ec, et): (Color, Vec3)| {
            c == ec && t.abs_diff_eq(et.normalize(), 1e-4)
        };
        app.update();
        let morning = shown(&app);
        assert!(close(morning, expect(6.)), "start variant");
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KEY);
        app.update();
        let evening = shown(&app);
        assert_eq!(app.world().resource::<MapLighting>().hour, 18.);
        assert!(close(evening, expect(18.)), "switched variant");
        assert_ne!(
            morning.1, evening.1,
            "the sun must move between 6 h and 18 h"
        );
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KEY);
        keys.clear();
        keys.press(KEY);
        app.update();
        assert!(close(shown(&app), expect(6.)), "back to the start");
    }
}
