//! One rule for the scene camera's sampleable depth, shared by every post pass that reads it.
//!
//! Fog, the depth of field and the Glow Filter each sample the camera's depth texture, which
//! needs `TEXTURE_BINDING` on it.
//! A sampleable depth left behind with no reader is what lost FwLo's Vulkan device (#94), and
//! per-effect release markers missed each other (glow removed while fog read depth, then fog
//! removed, left the bit on), so this one system owns the bit for all of them:
//! it switches the bit on while any reader is present, and strips it once none is left,
//! but only when it was the one that switched it on.
use super::{fog::GameFog, lens_dof::GameDof, sun_glow::GameGlow};
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;

/// Marks a camera whose depth sampling this rule switched on.
#[derive(Component)]
pub(crate) struct OwnedDepthSampling;

#[allow(clippy::type_complexity)]
pub(crate) fn sync_depth_sampling(
    mut commands: Commands,
    mut cameras: Query<(
        Entity,
        &mut Camera3d,
        Has<OwnedDepthSampling>,
        Has<GameFog>,
        Has<GameDof>,
        Has<GameGlow>,
    )>,
) {
    for (entity, mut camera, owned, fog, dof, glow) in &mut cameras {
        let read = fog || dof || glow;
        let usages = TextureUsages::from(camera.depth_texture_usages);
        let sampled = usages.contains(TextureUsages::TEXTURE_BINDING);
        if read && !sampled {
            camera.depth_texture_usages = (usages | TextureUsages::TEXTURE_BINDING).into();
            commands.entity(entity).insert(OwnedDepthSampling);
        } else if !read && owned {
            if sampled {
                camera.depth_texture_usages = (usages - TextureUsages::TEXTURE_BINDING).into();
            }
            commands.entity(entity).remove::<OwnedDepthSampling>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_systems(PostUpdate, sync_depth_sampling);
        app
    }

    fn sampled(app: &App, entity: Entity) -> bool {
        TextureUsages::from(
            app.world()
                .get::<Camera3d>(entity)
                .unwrap()
                .depth_texture_usages,
        )
        .contains(TextureUsages::TEXTURE_BINDING)
    }

    fn glow() -> GameGlow {
        GameGlow {
            params: Vec4::ONE,
            source: Vec4::ONE,
        }
    }

    #[test]
    fn depth_sampling_stays_while_any_reader_is_left_and_goes_with_the_last_in_either_order() {
        let mut app = app();
        let glow_first = app
            .world_mut()
            .spawn((Camera3d::default(), GameFog::default(), glow()))
            .id();
        let fog_first = app
            .world_mut()
            .spawn((Camera3d::default(), GameFog::default(), glow()))
            .id();
        app.update();
        assert!(sampled(&app, glow_first) && sampled(&app, fog_first));
        app.world_mut().entity_mut(glow_first).remove::<GameGlow>();
        app.world_mut().entity_mut(fog_first).remove::<GameFog>();
        app.update();
        assert!(sampled(&app, glow_first), "fog still reads depth");
        assert!(sampled(&app, fog_first), "the glow still reads depth");
        app.world_mut().entity_mut(glow_first).remove::<GameFog>();
        app.world_mut().entity_mut(fog_first).remove::<GameGlow>();
        app.update();
        assert!(!sampled(&app, glow_first), "left sampleable with no reader");
        assert!(!sampled(&app, fog_first), "left sampleable with no reader");
        // A reader coming back switches it on again.
        app.world_mut().entity_mut(fog_first).insert(glow());
        app.update();
        assert!(sampled(&app, fog_first));
    }

    #[test]
    fn depth_sampling_the_camera_already_had_is_left_alone() {
        let mut app = app();
        let camera = app
            .world_mut()
            .spawn((
                Camera3d {
                    depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC)
                        .into(),
                    ..default()
                },
                GameFog::default(),
            ))
            .id();
        app.update();
        app.world_mut().entity_mut(camera).remove::<GameFog>();
        app.update();
        let usages = TextureUsages::from(
            app.world()
                .get::<Camera3d>(camera)
                .unwrap()
                .depth_texture_usages,
        );
        assert!(usages.contains(TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC));
    }
}
