//! 1.8.9's 26 paintings.
//!
//! 1.8 has no `painting_variant` registry: `PaintingEntity.PaintingMotive` is
//! an enum of name, pixel size and texture offset, and a saved painting's
//! `Motive` is the enum constant's name (`Kebab`, `SkullAndRoses`). The data
//! fixer already maps those names to the current ids; this module supplies the
//! current variant files so the 1.8 target cannot pick one of 26.3's larger
//! painting set. `tools/PaintingProbe18.java` dumps the enum from the real
//! 1.8.9 jar and `examples/prepare_1_8_9_paintings.rs` writes the files.
//!
//! 1.8 also has no per-motive textures or title/author tooltips. Its 26 spans
//! live in one 256x256 atlas, and the renderer tiles the 16x16 back patch at
//! the atlas' (192, 0); the example crops both into the current
//! `textures/painting/<id>.png` files.
use crate::Result;

/// The 1.8 painting atlas (`textures/painting/paintings_kristoffer_zetterstrand.png`).
pub const ATLAS: &str = "paintings_kristoffer_zetterstrand.png";
/// The atlas edge in pixels, which every motive's texture offset indexes.
pub const ATLAS_SIZE: u32 = 256;
/// The back-of-the-painting square in the atlas: `PaintingEntityRenderer`
/// draws the back face with u `0.75..0.8125` and v `0..0.0625` (one 16x16
/// tile repeated per block) and the wooden side frames from a 1-pixel inset.
pub const BACK_REGION: (u32, u32, u32, u32) = (192, 0, 16, 16);

/// One 1.8 `PaintingMotive`, in enum order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The current `painting_variant` id, without the namespace.
    pub id: &'static str,
    /// The motive string a 1.8 save carries.
    pub save_name: &'static str,
    /// Block size (`PaintingMotive.width / 16`).
    pub width: i32,
    pub height: i32,
    /// `PaintingMotive.textureX` and `textureY`: the motive's atlas corner in
    /// pixels.
    pub atlas: (u32, u32),
}

/// The 26 motives 1.8.9 registers, in `PaintingMotive.values()` order.
pub const VARIANTS: [Variant; 26] = [
    Variant {
        id: "kebab",
        save_name: "Kebab",
        width: 1,
        height: 1,
        atlas: (0, 0),
    },
    Variant {
        id: "aztec",
        save_name: "Aztec",
        width: 1,
        height: 1,
        atlas: (16, 0),
    },
    Variant {
        id: "alban",
        save_name: "Alban",
        width: 1,
        height: 1,
        atlas: (32, 0),
    },
    Variant {
        id: "aztec2",
        save_name: "Aztec2",
        width: 1,
        height: 1,
        atlas: (48, 0),
    },
    Variant {
        id: "bomb",
        save_name: "Bomb",
        width: 1,
        height: 1,
        atlas: (64, 0),
    },
    Variant {
        id: "plant",
        save_name: "Plant",
        width: 1,
        height: 1,
        atlas: (80, 0),
    },
    Variant {
        id: "wasteland",
        save_name: "Wasteland",
        width: 1,
        height: 1,
        atlas: (96, 0),
    },
    Variant {
        id: "pool",
        save_name: "Pool",
        width: 2,
        height: 1,
        atlas: (0, 32),
    },
    Variant {
        id: "courbet",
        save_name: "Courbet",
        width: 2,
        height: 1,
        atlas: (32, 32),
    },
    Variant {
        id: "sea",
        save_name: "Sea",
        width: 2,
        height: 1,
        atlas: (64, 32),
    },
    Variant {
        id: "sunset",
        save_name: "Sunset",
        width: 2,
        height: 1,
        atlas: (96, 32),
    },
    Variant {
        id: "creebet",
        save_name: "Creebet",
        width: 2,
        height: 1,
        atlas: (128, 32),
    },
    Variant {
        id: "wanderer",
        save_name: "Wanderer",
        width: 1,
        height: 2,
        atlas: (0, 64),
    },
    Variant {
        id: "graham",
        save_name: "Graham",
        width: 1,
        height: 2,
        atlas: (16, 64),
    },
    Variant {
        id: "match",
        save_name: "Match",
        width: 2,
        height: 2,
        atlas: (0, 128),
    },
    Variant {
        id: "bust",
        save_name: "Bust",
        width: 2,
        height: 2,
        atlas: (32, 128),
    },
    Variant {
        id: "stage",
        save_name: "Stage",
        width: 2,
        height: 2,
        atlas: (64, 128),
    },
    Variant {
        id: "void",
        save_name: "Void",
        width: 2,
        height: 2,
        atlas: (96, 128),
    },
    Variant {
        id: "skull_and_roses",
        save_name: "SkullAndRoses",
        width: 2,
        height: 2,
        atlas: (128, 128),
    },
    Variant {
        id: "wither",
        save_name: "Wither",
        width: 2,
        height: 2,
        atlas: (160, 128),
    },
    Variant {
        id: "fighters",
        save_name: "Fighters",
        width: 4,
        height: 2,
        atlas: (0, 96),
    },
    Variant {
        id: "pointer",
        save_name: "Pointer",
        width: 4,
        height: 4,
        atlas: (0, 192),
    },
    Variant {
        id: "pigscene",
        save_name: "Pigscene",
        width: 4,
        height: 4,
        atlas: (64, 192),
    },
    Variant {
        id: "burning_skull",
        save_name: "BurningSkull",
        width: 4,
        height: 4,
        atlas: (128, 192),
    },
    Variant {
        id: "skeleton",
        save_name: "Skeleton",
        width: 4,
        height: 3,
        atlas: (192, 64),
    },
    Variant {
        id: "donkey_kong",
        save_name: "DonkeyKong",
        width: 4,
        height: 3,
        atlas: (192, 112),
    },
];

/// A `PaintingProbe18` line: the enum id, its saved name, pixel size and atlas
/// corner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpVariant {
    pub id: String,
    pub save_name: String,
    pub width: i32,
    pub height: i32,
    pub atlas: (u32, u32),
}

/// Parse `tools/PaintingProbe18.java`'s dump.
///
/// Each line is `id <tab> name <tab> pixel width <tab> pixel height <tab>
/// texture x <tab> texture y`. The pixel size is converted to blocks. The
/// probe's enum constant name for Aztec2 is `AZTEC_2`; the current id is
/// `aztec2`, so that one is normalized here.
pub fn parse_dump(text: &str) -> Result<Vec<DumpVariant>> {
    let mut out = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 6 {
            return Err(format!("line {}: {} fields, want 6", number + 1, fields.len()).into());
        }
        let number_field = |field: usize, what: &str| -> Result<i32> {
            fields[field]
                .parse()
                .map_err(|_| format!("line {}: {what} {:?}", number + 1, fields[field]).into())
        };
        let width = number_field(2, "width")?;
        let height = number_field(3, "height")?;
        let x = number_field(4, "texture x")?;
        let y = number_field(5, "texture y")?;
        if !(16..=64).contains(&width) || !(16..=64).contains(&height) {
            return Err(format!("line {}: {}x{} pixels", number + 1, width, height).into());
        }
        if width % 16 != 0 || height % 16 != 0 {
            return Err(format!(
                "line {}: {}x{} is not whole blocks",
                number + 1,
                width,
                height
            )
            .into());
        }
        if x < 0 || y < 0 || x + width > ATLAS_SIZE as i32 || y + height > ATLAS_SIZE as i32 {
            return Err(format!(
                "line {}: {width}x{height} at ({x}, {y}) leaves the {ATLAS_SIZE}x{ATLAS_SIZE} atlas",
                number + 1
            )
            .into());
        }
        let id = if fields[0] == "aztec_2" {
            "aztec2".to_owned()
        } else {
            fields[0].to_owned()
        };
        out.push(DumpVariant {
            id,
            save_name: fields[1].to_owned(),
            width: width / 16,
            height: height / 16,
            atlas: (x as u32, y as u32),
        });
    }
    Ok(out)
}

/// The current `painting_variant` JSON for one motive.
pub fn variant_json(variant: &Variant) -> serde_json::Value {
    serde_json::json!({
        "asset_id": format!("minecraft:{}", variant.id),
        "width": variant.width,
        "height": variant.height,
    })
}

/// The `#minecraft:placeable` tag: every 1.8 motive, in enum order.
pub fn placeable_tag() -> serde_json::Value {
    serde_json::json!({
        "values": VARIANTS.iter().map(|v| format!("minecraft:{}", v.id)).collect::<Vec<_>>(),
    })
}

/// An RGBA image: `width * height * 4` bytes, row-major from the top left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Copy `width`x`height` pixels at (`x`, `y`) out of `atlas`.
///
/// The 1.8 client crops its 26 motives and the back patch out of one atlas at
/// run time (`PaintingEntityRenderer`'s texture offsets); the current client
/// wants one texture per motive, so the prepared assets are these crops.
pub fn crop(atlas: &Image, x: u32, y: u32, width: u32, height: u32) -> Result<Image> {
    if atlas.pixels.len() != (atlas.width as usize) * (atlas.height as usize) * 4 {
        return Err("atlas pixel buffer does not match its size".into());
    }
    if width == 0
        || height == 0
        || x.checked_add(width).is_none_or(|r| r > atlas.width)
        || y.checked_add(height).is_none_or(|b| b > atlas.height)
    {
        return Err(format!(
            "{width}x{height} at ({x}, {y}) does not fit {}x{}",
            atlas.width, atlas.height
        )
        .into());
    }
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in y..y + height {
        let start = ((row * atlas.width + x) * 4) as usize;
        pixels.extend_from_slice(&atlas.pixels[start..start + (width * 4) as usize]);
    }
    Ok(Image {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The enum is 1.8's 26 motives, with 1.8's block sizes, save names and
    /// atlas rectangles.
    #[test]
    fn the_list_is_1_8s_26_motives() {
        assert_eq!(VARIANTS.len(), 26);
        let find = |id: &str| VARIANTS.iter().find(|v| v.id == id).unwrap();
        assert_eq!(find("kebab").save_name, "Kebab");
        assert_eq!((find("kebab").width, find("kebab").height), (1, 1));
        assert_eq!((find("pool").width, find("pool").height), (2, 1));
        assert_eq!((find("wanderer").width, find("wanderer").height), (1, 2));
        assert_eq!((find("fighters").width, find("fighters").height), (4, 2));
        assert_eq!((find("skeleton").width, find("skeleton").height), (4, 3));
        assert_eq!((find("pointer").width, find("pointer").height), (4, 4));
        assert_eq!(find("aztec2").save_name, "Aztec2");
        assert_eq!(find("aztec2").atlas, (48, 0));
        assert_eq!(find("skeleton").atlas, (192, 64));
        assert_eq!(find("donkey_kong").atlas, (192, 112));
        assert!(!VARIANTS.iter().any(|v| v.id == "meditative"));
        // Every motive and the back patch lie inside the 256x256 atlas.
        for variant in &VARIANTS {
            assert!(
                variant.atlas.0 + variant.width as u32 * 16 <= ATLAS_SIZE
                    && variant.atlas.1 + variant.height as u32 * 16 <= ATLAS_SIZE,
                "{} leaves the atlas",
                variant.id
            );
        }
        let (x, y, w, h) = BACK_REGION;
        assert!(x + w <= ATLAS_SIZE && y + h <= ATLAS_SIZE);
    }

    /// The probe's pixel sizes become block sizes, its atlas offsets become
    /// the crop corners, and its `AZTEC_2` id is the current `aztec2`.
    #[test]
    fn the_dump_parses_pixels_into_blocks_and_normalizes_aztec2() {
        let dump = concat!(
            "kebab\tKebab\t16\t16\t0\t0\n",
            "aztec_2\tAztec2\t16\t16\t48\t0\n",
            "wanderer\tWanderer\t16\t32\t0\t64\n",
            "fighters\tFighters\t64\t32\t0\t96\n",
        );
        let variants = parse_dump(dump).unwrap();
        assert_eq!(variants.len(), 4);
        assert_eq!(variants[0].id, "kebab");
        assert_eq!(variants[1].id, "aztec2");
        assert_eq!(variants[1].atlas, (48, 0));
        assert_eq!((variants[2].width, variants[2].height), (1, 2));
        assert_eq!((variants[3].width, variants[3].height), (4, 2));
        assert_eq!(
            parse_dump("kebab\tKebab\t16\n").unwrap_err().to_string(),
            "line 1: 3 fields, want 6"
        );
        assert!(parse_dump("kebab\tKebab\t24\t16\t0\t0\n").is_err());
        // A motive may not run off the atlas.
        assert!(parse_dump("kebab\tKebab\t16\t16\t248\t0\n").is_err());
    }

    /// A crop copies whole rows, not just the top-left pixel, and refuses a
    /// rectangle outside the atlas.
    #[test]
    fn a_crop_copies_the_requested_rectangle() {
        // A 4x4 atlas whose red channel is the pixel's index.
        let mut pixels = Vec::new();
        for i in 0..16u8 {
            pixels.extend_from_slice(&[i * 16, 0, 0, 255]);
        }
        let atlas = Image {
            width: 4,
            height: 4,
            pixels,
        };
        let out = crop(&atlas, 1, 2, 2, 2).unwrap();
        assert_eq!((out.width, out.height), (2, 2));
        assert_eq!(
            out.pixels,
            [
                9 * 16,
                0,
                0,
                255,
                10 * 16,
                0,
                0,
                255,
                13 * 16,
                0,
                0,
                255,
                14 * 16,
                0,
                0,
                255
            ]
        );
        assert!(crop(&atlas, 3, 2, 2, 2).is_err());
        assert!(crop(&atlas, 0, 0, 0, 1).is_err());
        let out = crop(&atlas, 2, 2, 2, 2).unwrap();
        assert_eq!(out.pixels.len(), 16);
    }

    /// The JSON names the 1.8 texture and carries no 26.3 title/author
    /// components; the tag lists every motive and nothing else.
    #[test]
    fn the_generated_json_and_tag_are_painting_variants() {
        let pointer = VARIANTS.iter().find(|v| v.id == "pointer").unwrap();
        let json = variant_json(pointer);
        assert_eq!(json["asset_id"], "minecraft:pointer");
        assert_eq!(
            (json["width"].as_i64(), json["height"].as_i64()),
            (Some(4), Some(4))
        );
        assert!(json.get("title").is_none());
        assert!(json.get("author").is_none());
        let tag = placeable_tag();
        let values = tag["values"].as_array().unwrap();
        assert_eq!(values.len(), VARIANTS.len());
        assert_eq!(values[0], "minecraft:kebab");
        assert!(values.iter().any(|v| v == "minecraft:donkey_kong"));
    }
}
