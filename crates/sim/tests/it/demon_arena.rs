use super::gyoubu_arena::stage;
use sekiro_sim::events::EventCommand;

#[test]
fn demon_arena_starts_at_night_and_runs_mark_and_defeat_events() {
    let Some((arena, mut scripts, mut host)) = stage("demon") else {
        return;
    };
    let cache = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cache");
    let warp = sekiro_sim::events::warp_point(&cache, 11, 0, 1102957).unwrap();
    assert!(
        arena
            .layout
            .wolf
            .position
            .distance(glam::Vec3::from_array(warp.position))
            < 0.001
    );
    assert_eq!(
        arena.spec.story_time(&arena.spec.enemy_spec()),
        sekiro_sim::story::StoryTime::Night
    );
    let commands = scripts.step(1.0 / 60.0, &host);
    assert!(commands.contains(&EventCommand::Immortality {
        entity: 1100900,
        on: true
    }));
    assert!(!commands.contains(&EventCommand::Immortality {
        entity: 1100900,
        on: false
    }));
    assert!(commands.iter().any(|c| matches!(
        c,
        EventCommand::BossBar {
            shown: true,
            entity: 1100900,
            name: 907021,
            ..
        }
    )));
    host.in_throw = true;
    assert!(
        scripts
            .step(1.0 / 60.0, &host)
            .contains(&EventCommand::Immortality {
                entity: 1100900,
                on: false
            })
    );
    host.in_throw = false;
    host.bars = 2;
    scripts.step(1.0 / 60.0, &host);
    assert!(scripts.flag(11105902));
    host.bars = 1;
    let commands = scripts.step(1.0 / 60.0, &host);
    assert!(commands.contains(&EventCommand::SetSpEffect {
        entity: 1100900,
        id: 3702005
    }));
    host.message = 30;
    scripts.step(1.0 / 60.0, &host);
    assert!(scripts.flag(9313));
    assert!(
        arena
            .fog_walls
            .iter()
            .all(|p| !p.wall.active(|f| scripts.flag(f)))
    );
}
