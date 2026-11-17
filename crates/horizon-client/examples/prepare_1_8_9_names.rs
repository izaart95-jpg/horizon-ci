//! Generate the 1.8.9 English item names the 26.3-keyed resource root asks for.
//!
//! The 1.8.9 client jar's `en_US.lang` names its items with 1.8 keys
//! (`item.diamond.name`, `tile.cloth.red.name`, `item.banner.red.name`), while
//! the engine's item components carry the current `minecraft:item_name`
//! translate keys (`item.minecraft.diamond`, `block.minecraft.red_wool`,
//! `block.minecraft.red_banner`). This reads the item-name dump of
//! `tools/ItemNameProbe18.java`, pairs every 1.8 item variant with the current
//! item it flattens to (`horizon_items::legacy_1_8_9::legacy_id`), and writes
//! the `NAMES_1_8_9` table `lang::Language::load` uses when the resource root
//! is a 1.8 one.
//!
//! The names are the 1.8 client's own English ones (`Wool`, not "White Wool";
//! "Spawn Creeper" for a spawn egg). A language that ships its own 1.8
//! `.lang` still resolves the 1.8 keys it has; the `NAMES_1_8_9` table only
//! fills the current keys that file cannot answer, and `LEGACY_KEYS_1_8_9`
//! lets `Language::load` move each locale's own value from the 1.8 key onto
//! the current one. A spawn egg is left out of the second table: 1.8 has one
//! `item.monsterPlacer` key for all 27, so the loader composes the locale's
//! item and `entity.<Name>.name` (`EGG_ENTITY_KEYS_1_8_9`).
//!
//! ```sh
//! javac -cp "$JAR" tools/ItemNameProbe18.java
//! java -cp "$JAR:$CLIENT_JAR:$LIBS:." ItemNameProbe18 > /tmp/opencode/item-names-1.8.9.txt
//! cargo run -p horizon-client --example prepare_1_8_9_names -- \
//!     --dump /tmp/opencode/item-names-1.8.9.txt \
//!     --root .local/horizon-client/1.8.9 \
//!     --out crates/horizon-client-core/src/lang/names_1_8_9.rs
//! ```

use horizon_items::legacy_1_8_9::legacy_id;
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fs,
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut dump = PathBuf::from("/tmp/opencode/item-names-1.8.9.txt");
    let mut root = PathBuf::from(".local/horizon-client/1.8.9");
    let mut out = PathBuf::from("crates/horizon-client-core/src/lang/names_1_8_9.rs");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dump" => dump = args.next().ok_or("--dump needs a path")?.into(),
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            "--out" => out = args.next().ok_or("--out needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    // (id, damage) -> the 1.8 translation key and display name the client
    // gives that variant.
    let text = fs::read_to_string(&dump)?;
    let mut by_variant: HashMap<(i16, i16), (String, String)> = HashMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(id), Some(damage), Some(key), Some(name)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Ok(id), Ok(damage)) = (id.parse::<i16>(), damage.parse::<i16>()) else {
            continue;
        };
        // A spawn egg whose damage names no entity renders the raw entity key
        // ("Spawn entity.ThrownEnderpearl.name"); no item is named that.
        if name.is_empty() || name.contains("entity.") {
            continue;
        }
        by_variant
            .entry((id, damage))
            .or_insert_with(|| (key.to_owned(), name.to_owned()));
    }
    let items: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&fs::read(root.join("items.json"))?)?;
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut legacy_keys: BTreeMap<String, String> = BTreeMap::new();
    let mut missing = Vec::new();
    for (item, components) in &items {
        let Some((id, damage)) = legacy_id(item) else {
            // A current item 1.8 never had; the 1.8 root cannot name it.
            continue;
        };
        let Some((legacy, name)) = by_variant.get(&(id, damage)) else {
            missing.push(item.clone());
            continue;
        };
        let Some(key) = components
            .get("minecraft:item_name")
            .and_then(|name| name.get("translate"))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        names.entry(key.to_owned()).or_insert_with(|| name.clone());
        // A spawn egg has one 1.8 key for all 27 eggs; `Language::load`
        // composes each with `entity.<Name>.name` instead. `ItemBanner`
        // overrides its display name with `item.banner.<colour>.name`, so the
        // key's colour picks the 1.8 colour word (`light_gray` is `silver`).
        let legacy_key = match legacy.as_str() {
            "item.monsterPlacer" => None,
            "tile.banner" => banner_colour(key).map(|colour| format!("item.banner.{colour}.name")),
            _ => Some(format!("{legacy}.name")),
        };
        if let Some(legacy_key) = legacy_key {
            legacy_keys.entry(key.to_owned()).or_insert(legacy_key);
        }
    }
    // 1.8 has no key for the splash potion item: `ItemPotion` prefixes the
    // potion's name with `potion.prefix.grenade` ("Splash"). The current item
    // has its own key, so name its base the way the 1.8 prefix reads.
    names
        .entry("item.minecraft.splash_potion".to_owned())
        .or_insert_with(|| "Splash Potion".to_owned());
    let mut body = String::new();
    body.push_str(
        "//! Generated by `cargo run -p horizon-client --example prepare_1_8_9_names`.\n\
         //!\n\
         //! The English name the 1.8.9 client's `en_US.lang` gives each item the\n\
         //! current resource root names with a current translation key, and the\n\
         //! 1.8 `.lang` key a locale file names the same item under. Do not\n\
         //! edit: rerun the generator instead.\n\n\
         pub(super) const NAMES_1_8_9: &[(&str, &str)] = &[\n",
    );
    for (key, name) in &names {
        body.push_str(&format!("    ({key:?}, {name:?}),\n"));
    }
    body.push_str(
        "];\n\n\
         /// The 1.8 `.lang` key each current item key would have had (`tile.clay.name`,\n\
         /// `item.diamond.name`): a 1.8 language file names the item there, so\n\
         /// `Language::load` copies its value onto the current key.\n\
         pub(super) const LEGACY_KEYS_1_8_9: &[(&str, &str)] = &[\n",
    );
    for (key, legacy) in &legacy_keys {
        body.push_str(&format!("    ({key:?}, {legacy:?}),\n"));
    }
    body.push_str("];\n");
    fs::write(&out, body)?;
    println!(
        "{} names, {} legacy keys -> {} ({} items had no 1.8 variant)",
        names.len(),
        legacy_keys.len(),
        out.display(),
        missing.len()
    );
    if !missing.is_empty() {
        println!("no 1.8 variant: {missing:?}");
    }
    Ok(())
}

/// The 1.8 `DyeColor` name in `ItemBanner`'s `item.banner.<colour>.name`.
fn banner_colour(key: &str) -> Option<&'static str> {
    let colour = key
        .strip_prefix("block.minecraft.")?
        .strip_suffix("_banner")?;
    Some(match colour {
        "black" => "black",
        "blue" => "blue",
        "brown" => "brown",
        "cyan" => "cyan",
        "gray" => "gray",
        "green" => "green",
        "light_blue" => "lightBlue",
        "light_gray" => "silver",
        "lime" => "lime",
        "magenta" => "magenta",
        "orange" => "orange",
        "pink" => "pink",
        "purple" => "purple",
        "red" => "red",
        "white" => "white",
        "yellow" => "yellow",
        _ => return None,
    })
}
