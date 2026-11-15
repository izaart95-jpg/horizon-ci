//! 1.8.9's 39 banner patterns.
//!
//! 1.8 has no `banner_pattern` registry: `BannerBlockEntity.BannerPattern` is
//! an enum of display name, two-letter save code and, for six of them, the
//! ingredient stack. `tools/BannerPatternProbe18.java` dumps the enum from the
//! real 1.8.9 jar and `examples/prepare_1_8_9_banners.rs` checks the dump
//! against [`PATTERNS`], then writes the current
//! `data/minecraft/banner_pattern/*.json` files and their tags.
//!
//! The enum's display name is already the current pattern id for all 39
//! (`DIAGONAL_RIGHT` saves as `rd` and is today's `diagonal_up_right`), so the
//! data fixer's code table and this list agree. 1.8's language keys are
//! `item.banner.<id>.<colour>`; the generated `translation_key` is the part
//! before the colour.
//!
//! 1.8 pattern textures are masks of opaque white (the pattern) and black
//! (the banner under it) on a transparent surround. The current renderer
//! wants the 1.9+ form, white pixels whose alpha is the mask, so the
//! generator converts them; without that every pattern draws as a black
//! flag with a coloured shape.
use crate::Result;

/// One 1.8 `BannerPattern`, in enum order (which is also the order
/// `CopyingRecipeType` tries its patterns in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pattern {
    /// The current `banner_pattern` id, without the namespace. This is also
    /// the 1.8 enum's display name and its texture file name.
    pub id: &'static str,
    /// The two-letter code a 1.8 banner or save writes in `Pattern`.
    pub save_code: &'static str,
    /// The 1.8 ingredient stack for the six patterns that take an item
    /// (`ItemBanner`'s `getIngredient`); `None` for the 3x3 dye layouts.
    pub ingredient: Option<Ingredient>,
}

/// A 1.8 ingredient stack: the item id and damage 1.8 matches, and the current
/// flattened item it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ingredient {
    /// The 1.8 item id (`minecraft:skull`, `minecraft:red_flower`).
    pub legacy_item: &'static str,
    /// The 1.8 damage value (`-1` for any).
    pub legacy_damage: i32,
    /// The current item id (`minecraft:creeper_head`,
    /// `minecraft:enchanted_golden_apple`).
    pub item: &'static str,
}

const fn item(
    legacy_item: &'static str,
    legacy_damage: i32,
    item: &'static str,
) -> Option<Ingredient> {
    Some(Ingredient {
        legacy_item,
        legacy_damage,
        item,
    })
}

/// The 39 patterns 1.8.9 registers, in `BannerPattern.values()` order.
pub const PATTERNS: [Pattern; 39] = [
    Pattern {
        id: "base",
        save_code: "b",
        ingredient: None,
    },
    Pattern {
        id: "square_bottom_left",
        save_code: "bl",
        ingredient: None,
    },
    Pattern {
        id: "square_bottom_right",
        save_code: "br",
        ingredient: None,
    },
    Pattern {
        id: "square_top_left",
        save_code: "tl",
        ingredient: None,
    },
    Pattern {
        id: "square_top_right",
        save_code: "tr",
        ingredient: None,
    },
    Pattern {
        id: "stripe_bottom",
        save_code: "bs",
        ingredient: None,
    },
    Pattern {
        id: "stripe_top",
        save_code: "ts",
        ingredient: None,
    },
    Pattern {
        id: "stripe_left",
        save_code: "ls",
        ingredient: None,
    },
    Pattern {
        id: "stripe_right",
        save_code: "rs",
        ingredient: None,
    },
    Pattern {
        id: "stripe_center",
        save_code: "cs",
        ingredient: None,
    },
    Pattern {
        id: "stripe_middle",
        save_code: "ms",
        ingredient: None,
    },
    Pattern {
        id: "stripe_downright",
        save_code: "drs",
        ingredient: None,
    },
    Pattern {
        id: "stripe_downleft",
        save_code: "dls",
        ingredient: None,
    },
    Pattern {
        id: "small_stripes",
        save_code: "ss",
        ingredient: None,
    },
    Pattern {
        id: "cross",
        save_code: "cr",
        ingredient: None,
    },
    Pattern {
        id: "straight_cross",
        save_code: "sc",
        ingredient: None,
    },
    Pattern {
        id: "triangle_bottom",
        save_code: "bt",
        ingredient: None,
    },
    Pattern {
        id: "triangle_top",
        save_code: "tt",
        ingredient: None,
    },
    Pattern {
        id: "triangles_bottom",
        save_code: "bts",
        ingredient: None,
    },
    Pattern {
        id: "triangles_top",
        save_code: "tts",
        ingredient: None,
    },
    Pattern {
        id: "diagonal_left",
        save_code: "ld",
        ingredient: None,
    },
    // 1.8's constant DIAGONAL_RIGHT is the current `diagonal_up_right`; the
    // data fixer's `rd` entry says the same.
    Pattern {
        id: "diagonal_up_right",
        save_code: "rd",
        ingredient: None,
    },
    Pattern {
        id: "diagonal_up_left",
        save_code: "lud",
        ingredient: None,
    },
    Pattern {
        id: "diagonal_right",
        save_code: "rud",
        ingredient: None,
    },
    Pattern {
        id: "circle",
        save_code: "mc",
        ingredient: None,
    },
    Pattern {
        id: "rhombus",
        save_code: "mr",
        ingredient: None,
    },
    Pattern {
        id: "half_vertical",
        save_code: "vh",
        ingredient: None,
    },
    Pattern {
        id: "half_horizontal",
        save_code: "hh",
        ingredient: None,
    },
    Pattern {
        id: "half_vertical_right",
        save_code: "vhr",
        ingredient: None,
    },
    Pattern {
        id: "half_horizontal_bottom",
        save_code: "hhb",
        ingredient: None,
    },
    Pattern {
        id: "border",
        save_code: "bo",
        ingredient: None,
    },
    Pattern {
        id: "curly_border",
        save_code: "cbo",
        ingredient: item("minecraft:vine", 0, "minecraft:vine"),
    },
    Pattern {
        id: "creeper",
        save_code: "cre",
        ingredient: item("minecraft:skull", 4, "minecraft:creeper_head"),
    },
    Pattern {
        id: "gradient",
        save_code: "gra",
        ingredient: None,
    },
    Pattern {
        id: "gradient_up",
        save_code: "gru",
        ingredient: None,
    },
    Pattern {
        id: "bricks",
        save_code: "bri",
        ingredient: item("minecraft:brick_block", 0, "minecraft:bricks"),
    },
    Pattern {
        id: "skull",
        save_code: "sku",
        ingredient: item("minecraft:skull", 1, "minecraft:wither_skeleton_skull"),
    },
    Pattern {
        id: "flower",
        save_code: "flo",
        ingredient: item("minecraft:red_flower", 8, "minecraft:oxeye_daisy"),
    },
    Pattern {
        id: "mojang",
        save_code: "moj",
        ingredient: item(
            "minecraft:golden_apple",
            1,
            "minecraft:enchanted_golden_apple",
        ),
    },
];

/// A `BannerPatternProbe18` line: enum name, save code, display name, whether
/// the enum has an ingredient, and the ingredient stack (`item@damage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpPattern {
    pub save_code: String,
    pub id: String,
    pub ingredient: Option<(String, i32)>,
}

/// Parse `tools/BannerPatternProbe18.java`'s dump.
pub fn parse_dump(text: &str) -> Result<Vec<DumpPattern>> {
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(format!("line {}: {} fields, want 5", number + 1, fields.len()).into());
        }
        if fields[1].is_empty() || fields[2].is_empty() {
            return Err(format!("line {}: empty code or name", number + 1).into());
        }
        let has_ingredient: bool = fields[3]
            .parse()
            .map_err(|_| format!("line {}: hasIngredient {:?}", number + 1, fields[3]))?;
        let ingredient = if fields[4] == "-" {
            None
        } else {
            let (item, damage) = fields[4]
                .rsplit_once('@')
                .ok_or_else(|| format!("line {}: ingredient {:?}", number + 1, fields[4]))?;
            let damage: i32 = damage
                .parse()
                .map_err(|_| format!("line {}: damage {:?}", number + 1, damage))?;
            Some((item.to_owned(), damage))
        };
        if has_ingredient != ingredient.is_some() {
            return Err(format!(
                "line {}: hasIngredient {has_ingredient} but {:?}",
                number + 1,
                fields[4]
            )
            .into());
        }
        out.push(DumpPattern {
            save_code: fields[1].to_owned(),
            id: fields[2].to_owned(),
            ingredient,
        });
    }
    Ok(out)
}

/// The current `banner_pattern` JSON for one pattern.
///
/// The translation key is 1.8's `item.banner.<id>`; `Banners::layer_key`
/// appends the 1.8 dye name (`item.banner.creeper.lightBlue`).
pub fn pattern_json(pattern: &Pattern) -> serde_json::Value {
    serde_json::json!({
        "asset_id": format!("minecraft:{}", pattern.id),
        "translation_key": format!("item.banner.{}", pattern.id),
    })
}

/// The current `#minecraft:no_item_required` tag: every pattern a blank banner
/// plus dyes can make (1.8's 32 layout patterns; `base` is not a layer and
/// the six ingredient patterns are not here).
pub fn no_item_required_tag() -> serde_json::Value {
    serde_json::json!({
        "values": PATTERNS
            .iter()
            .filter(|p| p.id != "base" && p.ingredient.is_none())
            .map(|p| format!("minecraft:{}", p.id))
            .collect::<Vec<_>>(),
    })
}

/// An RGBA image: `width * height * 4` bytes, row-major from the top left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Turn a 1.8 pattern's white-on-black mask into the current alpha mask.
///
/// 1.8 draws the pattern where the texture is opaque white and keeps the
/// banner underneath where it is opaque black (the transparent surround is
/// outside the flag). The 1.9+ renderer multiplies white pixels by the dye,
/// so the mask is the white channel as alpha and the colour is plain white.
pub fn alpha_mask(image: &Image) -> Result<Image> {
    if image.pixels.len() != (image.width as usize) * (image.height as usize) * 4 {
        return Err("image pixel buffer does not match its size".into());
    }
    let mut pixels = image.pixels.clone();
    for pixel in pixels.chunks_exact_mut(4) {
        if pixel[3] == 0 {
            pixel.copy_from_slice(&[255, 255, 255, 0]);
        } else {
            let mask = (u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2])) / 3;
            pixel.copy_from_slice(&[255, 255, 255, mask as u8]);
        }
    }
    Ok(Image {
        width: image.width,
        height: image.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The enum is 1.8's 39 patterns: the six ingredient stacks, the four
    /// diagonal code/name swaps and no post-1.8 pattern.
    #[test]
    fn the_list_is_1_8s_39_patterns() {
        assert_eq!(PATTERNS.len(), 39);
        let find = |id: &str| PATTERNS.iter().find(|p| p.id == id).unwrap();
        assert_eq!(find("base").save_code, "b");
        assert_eq!(find("diagonal_up_right").save_code, "rd");
        assert_eq!(find("diagonal_right").save_code, "rud");
        assert_eq!(find("diagonal_left").save_code, "ld");
        assert_eq!(find("diagonal_up_left").save_code, "lud");
        let ingredients: Vec<_> = PATTERNS.iter().filter(|p| p.ingredient.is_some()).collect();
        assert_eq!(ingredients.len(), 6);
        assert_eq!(
            find("creeper").ingredient.unwrap(),
            Ingredient {
                legacy_item: "minecraft:skull",
                legacy_damage: 4,
                item: "minecraft:creeper_head",
            }
        );
        assert_eq!(
            find("skull").ingredient.unwrap().item,
            "minecraft:wither_skeleton_skull"
        );
        assert_eq!(
            find("flower").ingredient.unwrap(),
            Ingredient {
                legacy_item: "minecraft:red_flower",
                legacy_damage: 8,
                item: "minecraft:oxeye_daisy",
            }
        );
        assert_eq!(
            find("mojang").ingredient.unwrap().item,
            "minecraft:enchanted_golden_apple"
        );
        for post_1_8 in ["flow", "guster", "globe", "piglin"] {
            assert!(!PATTERNS.iter().any(|p| p.id == post_1_8));
        }
    }

    /// The probe's five fields parse, its `item@damage` keeps the 1.8 damage,
    /// and a disagreement about the ingredient is an error.
    #[test]
    fn the_dump_parses_codes_names_and_ingredients() {
        let dump = concat!(
            "BASE\tb\tbase\tfalse\t-\n",
            "CREEPER\tcre\tcreeper\ttrue\tminecraft:skull@4\n",
            "FLOWER\tflo\tflower\ttrue\tminecraft:red_flower@8\n",
        );
        let parsed = parse_dump(dump).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].save_code, "b");
        assert_eq!(parsed[0].id, "base");
        assert_eq!(parsed[0].ingredient, None);
        assert_eq!(
            parsed[1].ingredient,
            Some(("minecraft:skull".to_owned(), 4))
        );
        assert_eq!(
            parsed[2].ingredient,
            Some(("minecraft:red_flower".to_owned(), 8))
        );
        assert_eq!(
            parse_dump("CREEPER\tcre\tcreeper\ttrue\n")
                .unwrap_err()
                .to_string(),
            "line 1: 4 fields, want 5"
        );
        assert!(parse_dump("CREEPER\tcre\tcreeper\ttrue\t-\n").is_err());
        assert!(parse_dump("CREEPER\tcre\tcreeper\ttrue\tminecraft:skull\n").is_err());
    }

    /// The conversion keeps the shape where 1.8 painted white and drops the
    /// black parts and the surround.
    #[test]
    fn the_alpha_mask_keeps_the_white_pixels() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![
                255, 255, 255, 255, // white: the pattern
                0, 0, 0, 255, // black: the banner under it
                0, 0, 0, 0, // transparent: outside the flag
                128, 128, 128, 255, // an edge
            ],
        };
        let out = alpha_mask(&image).unwrap();
        assert_eq!((out.width, out.height), (2, 2));
        assert_eq!(
            out.pixels,
            [
                255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 128
            ]
        );
        let broken = Image {
            width: 2,
            height: 2,
            pixels: vec![0; 4],
        };
        assert!(alpha_mask(&broken).is_err());
    }

    /// The JSON is the 1.8 language key, and the tag is the 32 patterns a
    /// blank banner and dyes make (no `base`, no ingredient pattern).
    #[test]
    fn the_generated_json_and_tag_are_1_8_banner_patterns() {
        let creeper = PATTERNS.iter().find(|p| p.id == "creeper").unwrap();
        let json = pattern_json(creeper);
        assert_eq!(json["asset_id"], "minecraft:creeper");
        assert_eq!(json["translation_key"], "item.banner.creeper");
        let tag = no_item_required_tag();
        let values = tag["values"].as_array().unwrap();
        assert_eq!(values.len(), 32);
        assert!(values.contains(&"minecraft:cross".into()));
        assert!(!values.contains(&"minecraft:creeper".into()));
        assert!(!values.contains(&"minecraft:base".into()));
    }
}
