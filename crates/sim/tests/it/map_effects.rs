use sekiro_sim::{map_effects, world_events};

#[test]
fn installed_map_effects_resolve_model_parameters_and_story_commands() {
    let cache = sekiro_sim::cache::root();
    if !cache.join("maps/m11_00_00_00/layout.json").exists() {
        return;
    }
    for map in world_events::SUPPORTED_MAPS {
        let state = world_events::first_visit(&cache, map).unwrap().unwrap();
        let catalog = map_effects::load(&cache, map, &state).unwrap();
        let braziers: Vec<_> = catalog
            .placements
            .iter()
            .filter(|p| p.param == Some(116000000))
            .collect();
        assert!(!braziers.is_empty());
        assert!(
            braziers
                .iter()
                .all(|p| p.effect == 839530 && p.dummy == Some(200))
        );
        assert!(
            catalog
                .placements
                .iter()
                .all(|p| p.position.iter().all(|x| x.is_finite()))
        );
        for placement in &catalog.placements {
            if let Some(write) = &placement.source {
                assert_eq!(write.entity, placement.entity);
                if write.command == 200903 {
                    assert!(state.bonfires.contains_key(&placement.entity));
                    continue;
                }
                assert!(state.effect_controls.iter().any(|c| c.event == write.event
                    && c.slot == write.slot
                    && c.instruction == write.instruction));
            }
        }
        assert!(catalog.placements.iter().any(|p| p.effect == 839500));
    }
}
