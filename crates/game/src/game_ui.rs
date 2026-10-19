//! The game's own look for every screen: its fonts and its menu art, read from the unpacked
//! install in `cache/raw` when the app starts (nothing is extracted ahead and nothing of the
//! game is committed).
//! - Fonts: `font/<lang>_<use>/font.gfx` converted to TrueType (`sekiro_formats::gfx_font`):
//!   `ui` is FOT-Matisse ProN (engus_std, the menus, HUD and prompts), `kanji` the Japanese
//!   Matisse (jpnjp_std) for marks such as 死. Each falls back to a Windows font, then Bevy's.
//! - Menu art: a sprite of the menu atlases (`menu/hi/01_common.tpf`, cut by the regions in
//!   `01_common.sblytbnd.d/<atlas>.layout`), drawn from the atlas's own DDS with the sprite's
//!   rectangle, so nothing is decoded on the CPU.
//!
//! After FwLo's RustSekiro `src/fonts.rs` and `tools/gfx_font.py` (the font roles and fallbacks)
//! and its `export_hud` sprite cuts, here done at load time.
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use bevy::text::FontSource;

/// The game's fonts, loaded once at startup.
#[derive(Resource, Default, Clone)]
pub struct GameFonts {
    pub ui: Option<Handle<Font>>,
    pub kanji: Option<Handle<Font>>,
}

impl GameFonts {
    pub fn load(cache: &Path, fonts: &mut Assets<Font>) -> Self {
        let mut add = |bytes: Option<Vec<u8>>| bytes.map(|b| fonts.add(Font::from_bytes(b)));
        Self {
            ui: add(font_bytes(
                cache,
                "engus_std",
                &["georgia.ttf", "segoeui.ttf"],
            )),
            kanji: add(font_bytes(
                cache,
                "jpnjp_std",
                &["YuGothB.ttc", "msgothic.ttc", "meiryo.ttc"],
            )),
        }
    }

    pub fn ui(&self, px: f32) -> TextFont {
        text_font(&self.ui, px)
    }

    pub fn kanji(&self, px: f32) -> TextFont {
        text_font(&self.kanji, px)
    }
}

fn text_font(font: &Option<Handle<Font>>, px: f32) -> TextFont {
    TextFont {
        font: font.clone().map(FontSource::Handle).unwrap_or_default(),
        font_size: bevy::text::FontSize::Px(px),
        ..default()
    }
}

/// The game's font `name` (`cache/raw/font/<name>/font.gfx`) as TrueType, else the first of
/// the Windows `fallbacks` found.
pub fn font_bytes(cache: &Path, name: &str, fallbacks: &[&str]) -> Option<Vec<u8>> {
    let game = std::fs::read(cache.join(format!("raw/font/{name}/font.gfx")))
        .ok()
        .and_then(|gfx| match sekiro_formats::gfx_font::to_sfnt(&gfx) {
            Ok(ttf) => Some(ttf),
            Err(e) => {
                warn!("game font {name}: {e}");
                None
            }
        });
    game.or_else(|| {
        fallbacks
            .iter()
            .find_map(|f| std::fs::read(Path::new("C:/Windows/Fonts").join(f)).ok())
    })
}

/// Sprites of one menu atlas (`SB_TextEffect`, `SB_Title`, ...): its texture and regions.
pub struct MenuAtlas {
    image: Handle<Image>,
    regions: HashMap<String, sekiro_formats::menu_layout::Region>,
}

impl MenuAtlas {
    /// Loads the atlases named (`SB_TextEffect` and kin) from `cache/raw/menu/hi/01_common`;
    /// the ones missing are left out with a warning.
    pub fn load(cache: &Path, names: &[&str], images: &mut Assets<Image>) -> HashMap<String, Self> {
        let dir = cache.join("raw/menu/hi");
        let layouts: Vec<_> = names
            .iter()
            .filter_map(|name| {
                let path = dir.join(format!("01_common.sblytbnd.d/{name}.layout"));
                let atlas = std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| {
                        sekiro_formats::menu_layout::parse(&b).map_err(|e| e.to_string())
                    });
                match atlas {
                    Ok(atlas) => Some((name.to_string(), atlas)),
                    Err(e) => {
                        warn!("menu atlas {name}: {e}");
                        None
                    }
                }
            })
            .collect();
        if layouts.is_empty() {
            return HashMap::new();
        }
        let Ok(tpf) = std::fs::read(dir.join("01_common.tpf")) else {
            warn!("menu art: no cache/raw/menu/hi/01_common.tpf");
            return HashMap::new();
        };
        let Ok(textures) = sekiro_formats::tpf::parse(&tpf) else {
            warn!("menu art: 01_common.tpf does not parse");
            return HashMap::new();
        };
        layouts
            .into_iter()
            .filter_map(|(name, atlas)| {
                let texture = textures
                    .iter()
                    .find(|t| format!("{}.png", t.name).eq_ignore_ascii_case(&atlas.image))?;
                let image = Image::from_buffer(
                    texture.dds,
                    bevy::image::ImageType::Extension("dds"),
                    bevy::image::CompressedImageFormats::all(),
                    true,
                    bevy::image::ImageSampler::linear(),
                    bevy::asset::RenderAssetUsages::RENDER_WORLD,
                )
                .inspect_err(|e| warn!("menu atlas {name}: {e}"))
                .ok()?;
                Some((
                    name,
                    Self {
                        image: images.add(image),
                        regions: atlas.regions,
                    },
                ))
            })
            .collect()
    }

    /// The sprite `name` (`MENU_Death`) as a UI image of its own aspect.
    pub fn sprite(&self, name: &str) -> Option<(ImageNode, f32)> {
        let r = self.regions.get(&format!("{name}.png"))?;
        let (x, y, w, h) = (r.x as f32, r.y as f32, r.width as f32, r.height as f32);
        let mut node = ImageNode::new(self.image.clone());
        node.rect = Some(Rect::new(x, y, x + w, y + h));
        Some((node, w / h))
    }
}
