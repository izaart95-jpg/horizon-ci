//! Placed banners (`BannerBlockEntity`): the pattern layers move from the
//! banner item to the block when placed and back to the dropped item when
//! broken (`copy_components` in the banner loot tables).
use super::{Game, maps::MapBanner};
use horizon_client::{
    banner::{self, BannerLayer},
    containers::BlockEntity,
};

impl Game<'_> {
    /// The pattern textures and tints of the banner at `pos`, bottom first.
    pub(super) fn banner_layers_at(&self, pos: [i32; 3]) -> Vec<(String, u32)> {
        let Some(BlockEntity::Banner(b)) = self.world.block_entities.get(&(pos[0], pos[1], pos[2]))
        else {
            return Vec::new();
        };
        layer_textures(&self.stations().banners, &b.layers)
    }
    /// The RGB tint of the banner's base layer at `pos`, when the tile entity
    /// carries it (1.8 banners are all white blocks with a `Base` colour).
    pub(super) fn banner_base_at(&self, pos: [i32; 3]) -> Option<u32> {
        let Some(BlockEntity::Banner(b)) = self.world.block_entities.get(&(pos[0], pos[1], pos[2]))
        else {
            return None;
        };
        base_color(b, "").map(|color| banner::dye_rgb(&color))
    }
    /// The banner at `pos` as a map marker source (`MapBanner.fromWorld`).
    pub(super) fn map_banner_at(&self, pos: [i32; 3]) -> Option<MapBanner> {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        map_banner(
            pos,
            self.world.block_entities.get(&(pos[0], pos[1], pos[2])),
            &self.res.facts().get(state).name,
        )
    }
}

/// `MapBanner.fromWorld`: the banner whose block entity is `entity` at `pos`,
/// with the base colour of its block (`BannerBlockEntity.getBaseColor` is
/// `AbstractBannerBlock.getColor`) and its custom name; none when the block
/// entity is not a banner.
pub(super) fn map_banner(
    pos: [i32; 3],
    entity: Option<&BlockEntity>,
    block: &str,
) -> Option<MapBanner> {
    let Some(BlockEntity::Banner(banner)) = entity else {
        return None;
    };
    Some(MapBanner {
        pos,
        color: base_color(banner, block)?,
        name: banner.name.clone(),
    })
}

/// The banner's base dye name. 1.8 keeps it in the tile entity's
/// `minecraft:base_color` component (its only banner blocks are white);
/// current versions take it from the block name.
fn base_color(banner: &horizon_client::containers::Banner, block: &str) -> Option<String> {
    if let Some(horizon_world::nbt::Tag::String(name)) = banner.components.get(
        &horizon_world::nbt::JavaString::from("minecraft:base_color"),
    ) {
        return Some(name.to_string_lossy());
    }
    let block = block.strip_prefix("minecraft:").unwrap_or(block);
    block
        .strip_suffix("_wall_banner")
        .or_else(|| block.strip_suffix("_banner"))
        .map(str::to_owned)
}

/// Texture names and tints of banner layers (`Sheets.getBannerSprite`).
pub(super) fn layer_textures(
    banners: &banner::Banners,
    layers: &[BannerLayer],
) -> Vec<(String, u32)> {
    layers
        .iter()
        .map(|l| {
            (
                banners.banner_texture(&l.pattern),
                banner::dye_rgb(&l.color),
            )
        })
        .collect()
}
