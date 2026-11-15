//! Build the client item definitions a 1.8.9 resource root needs.
//!
//! A 26.3 resource root binds every item through `assets/minecraft/items/<name>.json`
//! (`ClientItem`): a model tree, not a direct model file. The 1.8.9 jar predates
//! that folder — it ships `models/item/<name>.json` and lets the (obfuscated)
//! item classes pick one, so `ItemModels` found no definitions at all and every
//! item icon resolved to the missing model. This tool writes the definitions
//! for the 1.8 item set, keyed by the flattened names the engine already uses.
//!
//! The model for an item is, in order:
//!
//! 1. a special model (`minecraft:special`) for the items 1.8 renders from
//!    code: skulls, banners, chests, potions and spawn eggs;
//! 2. `minecraft:item/<name>` when the 1.8 jar has that model;
//! 3. a small rename table for the items 1.8 named differently (dyes, records,
//!    boats, fireworks);
//! 4. the model of the block's first blockstate variant (a 1.8 block item has
//!    no `models/item` file for most blocks; the block model is its icon).
//!
//! Clock and compass need their time-dependent frames. The 1.8 textures are one
//! animated strip each (`clock.png` is 16x1024, `compass.png` 16x512); the tool
//! crops them into `items/clock_00`..`items/clock_63` and
//! `items/compass_00`..`items/compass_31`, writes a model for each frame and
//! emits a `minecraft:range_dispatch` on 1.8's own time/compass rules, so the
//! icon follows the world clock and the world spawn instead of looping the
//! animation strip.
//!
//! Spawn eggs are one 1.8 item (383) whose damage is the entity id; its two
//! grayscale layers are tinted with the `EntityList.entityEggs` primary and
//! secondary colours, which 1.8 registers in `EntityList`'s static initializer
//! (the table below is from that class, not from 26.3 textures). The result is
//! a composite of the two layers, so the colour does not depend on a tint
//! index the 1.8 model never declared.
//!
//! The generated files stay in `.local/`; the tool also writes
//! `item-assets.json` so `tools/android_build.py` can bundle the files the
//! 1.8 jar does not have (definitions, cropped frames, template models, egg
//! layers).
//!
//! ```sh
//! cargo run --profile fast -p horizon-client --example prepare_1_8_9_items -- \
//!     --root .local/horizon-client/1.8.9
//! ```

use horizon_client::hud;
use horizon_items::legacy_1_8_9::legacy_id;
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

/// The sixteen 1.8 dye models, by the `Items.dye` damage byte.
const DYE_MODELS: [&str; 16] = [
    "dye_black",
    "dye_red",
    "dye_green",
    "dye_brown",
    "dye_blue",
    "dye_purple",
    "dye_cyan",
    "dye_silver",
    "dye_gray",
    "dye_pink",
    "dye_lime",
    "dye_yellow",
    "dye_light_blue",
    "dye_magenta",
    "dye_orange",
    "dye_white",
];

/// `EntityList.entityEggs`' primary and secondary colours, by the 1.8 entity
/// network id the egg's damage carries. From `EntityList`'s static
/// initializer (`new EntityEggInfo(name, id, primary, secondary)`); the two
/// mobs 1.8 registers without colours (snow golem, iron golem) have no egg,
/// and neither does 1.9's shulker.
const EGG_COLORS: [(i16, i32, i32); 27] = [
    (50, 894731, 0),
    (51, 12698049, 4802889),
    (52, 3419431, 11013646),
    (54, 44975, 7969893),
    (55, 5349438, 8306542),
    (56, 16382457, 12369084),
    (57, 15373203, 5009705),
    (58, 1447446, 0),
    (59, 803406, 11013646),
    (60, 7237230, 3158064),
    (61, 16167425, 16775294),
    (62, 3407872, 16579584),
    (65, 4996656, 986895),
    (66, 3407872, 5349438),
    (67, 1447446, 7237230),
    (68, 5931634, 15826224),
    (90, 15771042, 14377823),
    (91, 15198183, 16758197),
    (92, 4470310, 10592673),
    (93, 10592673, 16711680),
    (94, 2243405, 7375001),
    (95, 14144467, 13545366),
    (96, 10489616, 12040119),
    (98, 15720061, 5653556),
    (100, 12623485, 15656192),
    (101, 10051392, 7555121),
    (120, 5651507, 12422002),
];

/// The sixteen 1.8 banner colours, by the `Items.banner` damage byte. The
/// 1.8 banner model is `builtin/entity`; each flattened item gets a `special`
/// banner, as 26.3 does for its sixteen banner items.
const BANNER_COLORS: [&str; 16] = [
    "black",
    "red",
    "green",
    "brown",
    "blue",
    "purple",
    "cyan",
    "light_gray",
    "gray",
    "pink",
    "lime",
    "yellow",
    "light_blue",
    "magenta",
    "orange",
    "white",
];

fn head_transformation() -> Value {
    json!({
        "left_rotation": [1.0, 0.0, 0.0, -0.0],
        "right_rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0],
        "translation": [0.5, 0.0, 0.5],
    })
}

fn banner_transformation() -> Value {
    json!({
        "left_rotation": [0.0, -0.0, 0.0, 1.0],
        "right_rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [0.6666667, -0.6666667, -0.6666667],
        "translation": [0.5, 0.0, 0.5],
    })
}

fn model(id: &str) -> Value {
    json!({ "type": "minecraft:model", "model": id })
}

fn tinted(id: &str, color: i32) -> Value {
    json!({
        "type": "minecraft:model",
        "model": id,
        "tints": [{ "type": "minecraft:constant", "value": color }],
    })
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

/// The model of a block's first blockstate variant (1.8 block items take their
/// icon from the block model; only some blocks carry a `models/item` file).
fn block_model(blockstates: &Path, name: &str) -> Option<String> {
    let value = read_json(&blockstates.join(format!("{name}.json"))).ok()?;
    let model = if let Some(variants) = value.get("variants").and_then(Value::as_object) {
        variants
            .get("inventory")
            .or_else(|| variants.values().next())
            .and_then(|v| v.get("model"))
            .and_then(Value::as_str)?
    } else {
        value
            .get("multipart")?
            .as_array()?
            .first()?
            .get("apply")?
            .get("model")?
            .as_str()?
    };
    Some(if model.contains(':') {
        model.to_owned()
    } else {
        format!("minecraft:block/{model}")
    })
}

/// A 1.8 item whose model name differs from the current item name.
fn renamed_model(name: &str) -> Option<String> {
    let short = name.strip_prefix("minecraft:").unwrap_or(name);
    if let Some(disc) = short.strip_prefix("music_disc_") {
        return Some(format!("minecraft:item/record_{disc}"));
    }
    if short.ends_with("_bed") {
        // 1.8 has one bed item (355) with one red icon for every colour.
        return Some("minecraft:item/bed".to_owned());
    }
    Some(
        match short {
            "oak_boat" => "minecraft:item/boat",
            "firework_rocket" => "minecraft:item/fireworks",
            "firework_star" => "minecraft:item/firework_charge",
            "enchanted_golden_apple" => "minecraft:item/golden_apple",
            "grass_block" => "minecraft:item/grass",
            "lily_pad" => "minecraft:item/waterlily",
            "melon_slice" => "minecraft:item/melon",
            "glistering_melon_slice" => "minecraft:item/speckled_melon",
            "tropical_fish" => "minecraft:item/clownfish",
            "oak_sign" => "minecraft:item/sign",
            "spawner" => "minecraft:item/mob_spawner",
            _ => return None,
        }
        .to_owned(),
    )
}

fn head(kind: &str) -> Value {
    json!({
        "type": "minecraft:special",
        "base": "minecraft:item/template_skull",
        "model": { "type": "minecraft:head", "kind": kind },
        "transformation": head_transformation(),
    })
}

fn player_head() -> Value {
    json!({
        "type": "minecraft:special",
        "base": "minecraft:item/template_skull",
        "model": { "type": "minecraft:player_head" },
        "transformation": head_transformation(),
    })
}

fn banner(color: &str) -> Value {
    json!({
        "type": "minecraft:special",
        "base": "minecraft:item/template_banner",
        "model": { "type": "minecraft:banner", "color": color },
        "transformation": banner_transformation(),
    })
}

fn chest(texture: &str) -> Value {
    json!({
        "type": "minecraft:special",
        "base": "minecraft:item/template_chest",
        "model": { "type": "minecraft:chest", "texture": texture },
    })
}

/// A potion model with the 1.8 layer order (the overlay is layer 0, so the
/// one tint entry colours the liquid and leaves the bottle white).
fn potion(id: &str) -> Value {
    json!({
        "type": "minecraft:model",
        "model": id,
        "tints": [{ "type": "minecraft:potion", "default": -13083194 }],
    })
}

/// A `minecraft:range_dispatch` over a numeric property. `extra` carries the
/// property's own fields (`source`, `target`, `wobble`), which the decoder
/// reads from the same object.
fn dispatch(property: &str, scale: f32, extra: Value, entries: Vec<Value>) -> Value {
    let mut node = json!({
        "type": "minecraft:range_dispatch",
        "property": property,
        "scale": scale,
        "entries": entries,
    });
    if let (Some(node), Value::Object(extra)) = (node.as_object_mut(), extra) {
        for (key, value) in extra {
            node.insert(key, value);
        }
    }
    node
}

fn entry(threshold: f32, id: &str) -> Value {
    json!({ "threshold": threshold, "model": model(id) })
}

/// Crop every frame of a vertical animation strip and write one model per
/// frame. Returns the frame count.
fn crop_frames(
    assets: &Path,
    name: &str,
    display: &Value,
    manifest: &mut Vec<String>,
) -> Result<usize, Box<dyn Error>> {
    let png = assets.join(format!("textures/items/{name}.png"));
    let (width, height, rgba) = hud::decode(&png)?;
    if width == 0 || height % width != 0 {
        return Err(format!("{} is not a vertical strip", png.display()).into());
    }
    let frames = (height / width) as usize;
    let frame_bytes = (width * width * 4) as usize;
    for frame in 0..frames {
        let name = format!("{name}_{frame:02}");
        let texture = assets.join(format!("textures/items/{name}.png"));
        let mut encoder = png::Encoder::new(fs::File::create(&texture)?, width, width);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        let start = frame * frame_bytes;
        writer.write_image_data(&rgba[start..start + frame_bytes])?;
        writer.finish()?;
        manifest.push(format!("assets/minecraft/textures/items/{name}.png"));
        write_json(
            &assets.join(format!("models/item/{name}.json")),
            &json!({
                "parent": "builtin/generated",
                "textures": { "layer0": format!("items/{name}") },
                "display": display,
            }),
        )?;
        manifest.push(format!("assets/minecraft/models/item/{name}.json"));
    }
    Ok(frames)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut root = PathBuf::from(".local/horizon-client/1.8.9");
    let mut allow_missing = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            "--allow-missing" => allow_missing = true,
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let assets = root.join("assets/minecraft");
    let items: Map<String, Value> = read_json(&root.join("items.json"))?
        .as_object()
        .cloned()
        .ok_or("items.json is not an object")?;
    let item_models = assets.join("models/item");
    let blockstates = assets.join("blockstates");
    let definitions = assets.join("items");
    if definitions.is_dir() {
        fs::remove_dir_all(&definitions)?;
    }
    fs::create_dir_all(&definitions)?;

    let mut manifest: Vec<String> = Vec::new();
    let mut written = 0usize;
    let mut specials = 0usize;
    let mut unresolved: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // 1.8's splash potion is `Items.potionitem` with damage bit 14 set, so the
    // datafixer only names it when a stack carries that damage; it is not in
    // the reverse map (`legacy_id`), but it is a 1.8 item and needs an icon.
    let names: Vec<String> = items
        .keys()
        .filter(|name| legacy_id(name).is_some() || name.as_str() == "minecraft:splash_potion")
        .cloned()
        .collect();

    for name in names {
        let short = name.strip_prefix("minecraft:").unwrap_or(&name).to_owned();
        if !seen.insert(short.clone()) {
            continue;
        }
        let (id, damage) = legacy_id(&name).unwrap_or((373, 0));
        let node = if let Some(node) = special_item(&short, id, damage)? {
            specials += 1;
            node
        } else if item_models.join(format!("{short}.json")).is_file() {
            model(&format!("minecraft:item/{short}"))
        } else if let Some(id) = renamed_model(&name) {
            model(&id)
        } else if let Some(id) = block_model(&blockstates, &short) {
            model(&id)
        } else {
            unresolved.push(name.clone());
            continue;
        };
        write_json(
            &definitions.join(format!("{short}.json")),
            &json!({ "model": node }),
        )?;
        manifest.push(format!("assets/minecraft/items/{short}.json"));
        written += 1;
    }

    // The clock and compass frames are generated from their animation strips.
    for (name, count) in [("clock", 64usize), ("compass", 32)] {
        let display = read_json(&item_models.join(format!("{name}.json")))?
            .get("display")
            .cloned()
            .unwrap_or(Value::Null);
        let frames = crop_frames(&assets, name, &display, &mut manifest)?;
        if frames != count {
            return Err(format!("{name} strip has {frames} frames, expected {count}").into());
        }
    }
    write_json(
        &definitions.join("clock.json"),
        &json!({ "model": clock_model() }),
    )?;
    write_json(
        &definitions.join("compass.json"),
        &json!({ "model": compass_model() }),
    )?;

    // Special bases: an empty model with the 1.8 display transforms is all the
    // special renderer needs (the geometry comes from the special itself).
    for (model, source) in [
        ("template_skull", "skull_skeleton"),
        ("template_banner", "banner"),
        ("template_chest", "chest"),
    ] {
        let source = read_json(&item_models.join(format!("{source}.json")))?;
        let mut base = json!({
            "textures": { "particle": "block/soul_sand" },
        });
        if let Some(display) = source.get("display") {
            base["display"] = display.clone();
        }
        write_json(&item_models.join(format!("{model}.json")), &base)?;
        manifest.push(format!("assets/minecraft/models/item/{model}.json"));
    }
    // The two 1.8 spawn egg layers are shared by every egg.
    for (model, texture) in [
        ("spawn_egg_base", "items/spawn_egg"),
        ("spawn_egg_overlay", "items/spawn_egg_overlay"),
    ] {
        let source = read_json(&item_models.join("spawn_egg.json"))?;
        let mut layer = json!({
            "parent": "builtin/generated",
            "textures": { "layer0": texture },
        });
        if let Some(display) = source.get("display") {
            layer["display"] = display.clone();
        }
        write_json(&item_models.join(format!("{model}.json")), &layer)?;
        manifest.push(format!("assets/minecraft/models/item/{model}.json"));
    }

    manifest.sort();
    manifest.dedup();
    write_json(
        &root.join("item-assets.json"),
        &json!({
            "format": 1,
            "definitions": written,
            "specials": specials,
            "item_assets": manifest,
            "unresolved": unresolved,
        }),
    )?;
    println!(
        "{written} item definitions ({specials} special) -> {}; {} generated assets",
        definitions.display(),
        manifest.len()
    );
    if !unresolved.is_empty() {
        println!("no model for {} items:", unresolved.len());
        for name in &unresolved {
            println!("  {name}");
        }
        if !allow_missing {
            return Err(format!("{} items have no model", unresolved.len()).into());
        }
    }
    Ok(())
}

/// A definition for a 1.8 item whose icon is drawn by code.
fn special_item(name: &str, id: i16, damage: i16) -> Result<Option<Value>, Box<dyn Error>> {
    let node = match name {
        "skeleton_skull" => head("skeleton"),
        "wither_skeleton_skull" => head("wither_skeleton"),
        "zombie_head" => head("zombie"),
        "creeper_head" => head("creeper"),
        "player_head" => player_head(),
        "chest" => chest("minecraft:normal"),
        "ender_chest" => chest("minecraft:ender"),
        "trapped_chest" => chest("minecraft:trapped"),
        "potion" => potion("minecraft:item/bottle_drinkable"),
        "splash_potion" => potion("minecraft:item/bottle_splash"),
        _ if id == 351 => model(&format!(
            "minecraft:item/{}",
            DYE_MODELS[(damage & 15) as usize]
        )),
        _ if id == 425 => banner(BANNER_COLORS[(damage & 15) as usize]),
        _ if id == 383 => {
            // A spawn egg: 1.8 tints the grayscale base with the primary
            // colour and the overlay with the secondary one.
            let (primary, secondary) = EGG_COLORS
                .iter()
                .find(|&&(entity, _, _)| entity == damage)
                .map(|&(_, primary, secondary)| (primary, secondary))
                .ok_or_else(|| format!("{name} has no EntityList egg colours"))?;
            json!({
                "type": "minecraft:composite",
                "models": [
                    tinted("minecraft:item/spawn_egg_base", primary),
                    tinted("minecraft:item/spawn_egg_overlay", secondary),
                ],
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(node))
}

fn clock_model() -> Value {
    let mut entries: Vec<Value> = vec![entry(0.0, "minecraft:item/clock_00")];
    for frame in 1..64 {
        entries.push(entry(
            frame as f32 - 0.5,
            &format!("minecraft:item/clock_{frame:02}"),
        ));
    }
    entries.push(entry(63.5, "minecraft:item/clock_00"));
    dispatch(
        "minecraft:time",
        64.0,
        json!({ "source": "daytime", "wobble": false }),
        entries,
    )
}

fn compass_model() -> Value {
    let mut entries: Vec<Value> = vec![entry(0.0, "minecraft:item/compass_16")];
    for frame in 1..32 {
        entries.push(entry(
            frame as f32 - 0.5,
            &format!("minecraft:item/compass_{:02}", (16 + frame) % 32),
        ));
    }
    entries.push(entry(31.5, "minecraft:item/compass_16"));
    dispatch(
        "minecraft:compass",
        32.0,
        json!({ "target": "spawn", "wobble": false }),
        entries,
    )
}
