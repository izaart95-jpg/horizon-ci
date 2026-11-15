//! Banner pattern layers (`BannerPatternLayers`) and their NBT form.
use horizon_world::nbt::Tag;
use std::sync::Arc;

/// `BannerPatternLayers.Layer`: a pattern id and the dye name it is painted in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerLayer {
    pub pattern: Arc<str>,
    pub color: Arc<str>,
}
impl BannerLayer {
    pub fn new(pattern: &str, color: &str) -> Self {
        Self {
            pattern: Arc::from(pattern),
            color: Arc::from(color),
        }
    }
}

/// `DyeColor` names in `getId()` order (white first, black last). 1.8 stores
/// banner colours as `DyeColor.getSwappedId()`, the complement of this index.
pub const DYE_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

/// The 1.8 `DyeColor.getSwappedId()` of a dye name: the metadata 1.8 writes
/// in a banner's `Base` and `Patterns[].Color` (0 black, 15 white).
pub fn dye_meta(name: &str) -> Option<u8> {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    DYE_COLORS
        .iter()
        .position(|color| *color == name)
        .map(|id| 15 - id as u8)
}

/// The dye name of a 1.8 `DyeColor.getSwappedId()` metadata.
pub fn dye_name(meta: i32) -> &'static str {
    DYE_COLORS[15 - meta.clamp(0, 15) as usize]
}

/// The `banner_patterns` NBT list of layers.
pub fn layers_to_nbt(layers: &[BannerLayer]) -> Tag {
    Tag::List(
        layers
            .iter()
            .map(|l| {
                Tag::compound([
                    ("pattern", Tag::String((&*l.pattern).into())),
                    ("color", Tag::String((&*l.color).into())),
                ])
            })
            .collect(),
    )
}
pub fn layers_from_nbt(tag: Option<&Tag>) -> Vec<BannerLayer> {
    let Some(Tag::List(list)) = tag else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|e| {
            let text = |k: &str| match e.get(k) {
                Some(Tag::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            };
            let pattern = text("pattern")?;
            let color = text("color")?;
            Some(BannerLayer::new(
                &if pattern.contains(':') {
                    pattern
                } else {
                    format!("minecraft:{pattern}")
                },
                &color,
            ))
        })
        .collect()
}
