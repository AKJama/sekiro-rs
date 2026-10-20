//! The game's own look for every screen: its fonts, texts and menu art, read from the unpacked
//! install in `cache/raw` when the app starts (nothing is extracted ahead and nothing of the
//! game is committed).
//! - Font: `font/engus_std/font.gfx` (FOT-Matisse ProN, the menus, HUD and prompts) converted
//!   to TrueType (`sekiro_formats::gfx_font`), else a Windows font, else Bevy's.
//! - Texts: the menu FMG banks (`msg/engus/menu.msgbnd.d/*.fmg`).
//! - Menu art: a sprite of the menu atlases (`menu/hi/01_common.tpf`, cut by the regions in
//!   `01_common.sblytbnd.d/<atlas>.layout`), drawn from the atlas's own DDS with the sprite's
//!   rectangle, so nothing is decoded on the CPU.
//!
//! After FwLo's RustSekiro `src/fonts.rs` and `tools/gfx_font.py` (the font and its fallbacks)
//! and its `export_hud` sprite cuts, here done at load time.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::prelude::*;
use bevy::text::FontSource;

/// The game's fonts, converted once when the app is built.
#[derive(Resource, Default, Clone)]
pub struct GameFonts {
    pub ui: Option<Handle<Font>>,
}

impl GameFonts {
    /// Converts the fonts and inserts them, unless an earlier plugin already did; call after
    /// the text plugin is added and before `Startup`.
    pub fn insert(app: &mut App) {
        if app.world().contains_resource::<Self>() {
            return;
        }
        let bytes = font_bytes(
            &sekiro_sim::cache::root(),
            "engus_std",
            &["georgia.ttf", "segoeui.ttf"],
        );
        let ui = bytes.map(|b| {
            app.world_mut()
                .resource_mut::<Assets<Font>>()
                .add(Font::from_bytes(b))
        });
        app.insert_resource(Self { ui });
    }

    pub fn ui(&self, px: f32) -> TextFont {
        TextFont {
            font: self.ui.clone().map(FontSource::Handle).unwrap_or_default(),
            font_size: bevy::text::FontSize::Px(px),
            ..default()
        }
    }
}

/// The game's font `name` (`cache/raw/font/<name>/font.gfx`) as TrueType, else the first of
/// the Windows `fallbacks` found.
fn font_bytes(cache: &Path, name: &str, fallbacks: &[&str]) -> Option<Vec<u8>> {
    let game = std::fs::read(cache.join(format!("raw/font/{name}/font.gfx")))
        .ok()
        .and_then(|gfx| {
            sekiro_formats::gfx_font::to_sfnt(&gfx)
                .inspect_err(|e| warn!("game font {name}: {e}"))
                .ok()
        });
    game.or_else(|| {
        fallbacks
            .iter()
            .find_map(|f| std::fs::read(Path::new("C:/Windows/Fonts").join(f)).ok())
    })
}

/// A menu text bank (`イベントテキスト` and kin, `cache/raw/msg/engus/menu.msgbnd.d`), empty
/// when it is missing.
pub fn menu_texts(cache: &Path, bank: &str) -> BTreeMap<i32, String> {
    let path = cache.join(format!("raw/msg/engus/menu.msgbnd.d/{bank}.fmg"));
    std::fs::read(&path)
        .ok()
        .and_then(|b| {
            sekiro_formats::fmg::parse(&b)
                .inspect_err(|e| warn!("menu texts {bank}: {e}"))
                .ok()
        })
        .unwrap_or_default()
}

/// Sprites of one menu atlas (`SB_TextEffect`, `SB_Title`, ...): its texture and regions.
pub struct MenuAtlas {
    image: Handle<Image>,
    regions: HashMap<String, sekiro_formats::menu_layout::Region>,
}

impl MenuAtlas {
    /// The atlas `name` from `cache/raw/menu/hi/01_common`, or `None` with a warning.
    pub fn load(cache: &Path, name: &str, images: &mut Assets<Image>) -> Option<Self> {
        let dir = cache.join("raw/menu/hi");
        let mut load = || -> Result<Self, String> {
            let layout = std::fs::read(dir.join(format!("01_common.sblytbnd.d/{name}.layout")))
                .map_err(|e| e.to_string())?;
            let atlas = sekiro_formats::menu_layout::parse(&layout).map_err(|e| e.to_string())?;
            let tpf = std::fs::read(dir.join("01_common.tpf")).map_err(|e| e.to_string())?;
            let textures = sekiro_formats::tpf::parse(&tpf).map_err(|e| e.to_string())?;
            let texture = textures
                .iter()
                .find(|t| format!("{}.png", t.name).eq_ignore_ascii_case(&atlas.image))
                .ok_or_else(|| format!("no texture {}", atlas.image))?;
            let image = Image::from_buffer(
                texture.dds,
                bevy::image::ImageType::Extension("dds"),
                bevy::image::CompressedImageFormats::all(),
                true,
                bevy::image::ImageSampler::linear(),
                bevy::asset::RenderAssetUsages::RENDER_WORLD,
            )
            .map_err(|e| e.to_string())?;
            Ok(Self {
                image: images.add(image),
                regions: atlas.regions,
            })
        };
        load().inspect_err(|e| warn!("menu atlas {name}: {e}")).ok()
    }

    /// The sprite `name` (`MENU_Death`) as a UI image, with its width over height.
    pub fn sprite(&self, name: &str) -> Option<(ImageNode, f32)> {
        let r = self.regions.get(&format!("{name}.png"))?;
        let (x, y, w, h) = (r.x as f32, r.y as f32, r.width as f32, r.height as f32);
        let mut node = ImageNode::new(self.image.clone());
        node.rect = Some(Rect::new(x, y, x + w, y + h));
        Some((node, w / h))
    }
}
