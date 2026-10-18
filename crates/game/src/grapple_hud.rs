//! Wire HUD sprites loaded read-only from the player's menu binder and TPF.
use anyhow::{Context as ContextExt, Result};
use bevy::prelude::*;
use sekiro_formats::{bnd4, dvd::Vfs, menu_layout, tpf};
use sekiro_sim::wire::MarkerState;
use std::path::Path;

#[derive(Component)]
pub(super) struct Marker;
#[derive(Component)]
pub(super) struct Fill;
#[derive(Component)]
pub(super) struct Cursor;

#[derive(Resource)]
pub(super) struct Sprites {
    ready: Handle<Image>,
    ready_ring: Handle<Image>,
    distant_ring: Handle<Image>,
    cursor: Handle<Image>,
    distant: Handle<Image>,
    distance_pixels: Vec<u8>,
    size: UVec2,
    last_fill: Option<f32>,
}

fn image(size: UVec2, pixels: Vec<u8>) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}

impl Sprites {
    fn load(cache: &Path, images: &mut Assets<Image>) -> Result<Self> {
        let keys = std::fs::read_to_string(cache.join("refs/ArchiveKeys.cs"))?;
        let vfs = Vfs::open(
            &sekiro_fmod::game_dir(),
            &sekiro_formats::dvd::parse_uxm_keys(&keys),
            &[],
        )?;
        let binder = vfs.read("/menu/hi/01_common.sblytbnd.dcx")?;
        let entries = bnd4::parse(&binder)?;
        let layout = entries
            .iter()
            .find(|f| f.file_name() == "SB_FE.layout")
            .with_context(|| "SB_FE.layout missing")?;
        let atlas = menu_layout::parse(&layout.contents(vfs.oodle())?)?;
        let bytes = vfs.read("/menu/hi/01_common.tpf.dcx")?;
        let textures = tpf::parse(&bytes)?;
        let texture = textures
            .iter()
            .find(|t| format!("{}.png", t.name) == atlas.image)
            .with_context(|| "SB_FE texture missing")?;
        let header = texture.header()?;
        anyhow::ensure!(
            header.format == tpf::DdsFormat::Bc7,
            "unsupported SB_FE atlas format {:?}",
            header.format
        );
        let mut pixels = vec![0; (header.width * header.height) as usize];
        texture2ddecoder::decode_bc7(
            header.top_mip(texture.dds)?,
            header.width as usize,
            header.height as usize,
            &mut pixels,
        )
        .map_err(anyhow::Error::msg)?;
        let crop = |name: &str| -> Result<(UVec2, Vec<u8>)> {
            let r = atlas
                .regions
                .get(name)
                .with_context(|| "wire sprite missing")?;
            anyhow::ensure!(
                r.x.checked_add(r.width).is_some_and(|x| x <= header.width)
                    && r.y
                        .checked_add(r.height)
                        .is_some_and(|y| y <= header.height),
                "wire sprite outside atlas"
            );
            let mut rgba = Vec::with_capacity((r.width * r.height * 4) as usize);
            for y in r.y..r.y + r.height {
                for x in r.x..r.x + r.width {
                    let [b, g, r, a] = pixels[(y * header.width + x) as usize].to_le_bytes();
                    rgba.extend_from_slice(&[r, g, b, a]);
                }
            }
            Ok((UVec2::new(r.width, r.height), rgba))
        };
        let (size, distance_pixels) = crop("MENU_Wire_Distance.png")?;
        let (ready_size, ready) = crop("MENU_Wire_Marker.png")?;
        let (ring_size, ring) = crop("MENU_Wire_01.png")?;
        let (distant_ring_size, distant_ring) = crop("MENU_Wire_02.png")?;
        let (cursor_size, cursor) = crop("MENU_Wire_Cursor.png")?;
        anyhow::ensure!(size == ready_size, "wire sprite sizes differ");
        // NG is a diagonal strike, Cursor an arrow. Neither is the distant circular mark.
        info!(
            "wire HUD: SB_FE MENU_Wire_Marker + MENU_Wire_Distance, {}x{}",
            size.x, size.y
        );
        Ok(Self {
            ready: images.add(image(size, ready)),
            ready_ring: images.add(image(ring_size, ring)),
            distant_ring: images.add(image(distant_ring_size, distant_ring)),
            cursor: images.add(image(cursor_size, cursor)),
            distant: images.add(image(size, distance_pixels.clone())),
            distance_pixels,
            size,
            last_fill: None,
        })
    }

    pub(super) fn ring(&self, state: MarkerState) -> Handle<Image> {
        if state == MarkerState::Ready {
            self.ready_ring.clone()
        } else {
            self.distant_ring.clone()
        }
    }

    pub(super) fn draw(&mut self, state: MarkerState, images: &mut Assets<Image>) -> Handle<Image> {
        let MarkerState::Distant { fill } = state else {
            return self.ready.clone();
        };
        if self.last_fill != Some(fill) {
            self.last_fill = Some(fill);
            let mut pixels = self.distance_pixels.clone();
            // The atlas contains no animation instructions. Expand from the central dot
            // toward the authored disc edge; Wire_02 supplies the fixed outer rim.
            for (i, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let p = Vec2::new(
                    i as f32 % self.size.x as f32 + 0.5,
                    (i / self.size.x as usize) as f32 + 0.5,
                ) / self.size.as_vec2()
                    - Vec2::splat(0.5);
                let radius = p.length();
                let texels = self.size.x as f32;
                let reveal = if fill >= 1.0 {
                    1.0
                } else {
                    ((0.11 + 0.39 * fill - radius) * texels + 0.5).clamp(0.0, 1.0)
                };
                pixel[3] = (pixel[3] as f32 * reveal).round() as u8;
            }
            if let Some(mut img) = images.get_mut(&self.distant) {
                img.data = Some(pixels);
            }
        }
        self.distant.clone()
    }
}

pub(super) fn spawn(commands: &mut Commands, cache: &Path, images: &mut Assets<Image>) {
    match Sprites::load(cache, images) {
        Ok(sprites) => {
            commands
                .spawn((
                    Marker,
                    Node {
                        position_type: PositionType::Absolute,
                        display: Display::None,
                        ..default()
                    },
                    ImageNode::new(sprites.distant_ring.clone()),
                    GlobalZIndex(20),
                ))
                .with_children(|p| {
                    p.spawn((
                        Fill,
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(24.615),
                            top: percent(24.615),
                            width: percent(50.77),
                            height: percent(50.77),
                            ..default()
                        },
                        ImageNode::new(sprites.distant.clone()),
                    ));
                    p.spawn((
                        Cursor,
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(16.92),
                            top: percent(90),
                            width: percent(66.15),
                            height: percent(29.23),
                            display: Display::None,
                            ..default()
                        },
                        ImageNode::new(sprites.cursor.clone()),
                    ));
                });
            commands.insert_resource(sprites);
        }
        Err(e) => warn!("wire HUD unavailable: {e:#}"),
    }
}
