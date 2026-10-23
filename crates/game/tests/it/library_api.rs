//! What another crate (RustSekiro) loads a map with, each item named through the library, so
//! making one private again fails this build (#193).

#[test]
fn the_map_library_items_stay_public() {
    use sekiro_rs::map;
    let _: fn(&mut bevy::prelude::App) = map::ready::install;
    let _ = map::upload_budget;
    let _ = map::layered_gltf_plugin;
    let _ = map::world::enabled;
    let _ = map::world::connects;
    let _ = map::world::play_blocks;
    let _ = map::world::collision_world;
    let _ = map::visibility::supported;
    let _: Option<&map::story::MapStory> = None;
    let _: Option<map::AppHook> = map::APP_HOOKS.get().map(|h| h.0);
}
