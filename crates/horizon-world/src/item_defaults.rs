//! Items' default components, as everything in the game reads them: the
//! resource root's `items.json` (vanilla's default-component report, with a
//! content mod's items), then what native mods change in it
//! ([`native_modify_default_components`], NeoForge's `ModifyDefaultComponentsEvent`).
//! The item table, loot, enchanting, item models and the other readers all
//! come through [`read`], so a mod's change reaches each of them, and stacks
//! are saved and sent against the changed defaults as against vanilla's.

use serde_json::{Map, Value};
use std::{collections::BTreeMap, fs, io, path::Path};

/// Hook point for native mods (docs/native-mods.md, listed in `native-hooks.json`
/// with those of `horizon-client/src/game/native_hooks.rs`): `components` holds
/// `item`'s default components in `items.json`'s form (`"minecraft:max_stack_size": 16`,
/// `"minecraft:food": {"nutrition": 4, "saturation": 2.4}`); insert or replace one to
/// set it, remove it to take it away (a stack size outside 1 to 99 is brought back
/// into it). Asked for every item each time the game reads
/// the defaults, from any thread, several times as resources load and again when a
/// world's data packs load: answer the same each time. Client and server each ask
/// their own mods, as NeoForge does, so both sides need the mod.
/// 79.
#[inline(never)]
pub fn native_modify_default_components(item: &str, components: &mut Map<String, Value>) {
    let _ = (item, components);
}

/// Whether `root` is a 1.8.9 resource tree. The game only loads the root its
/// manifest pins, but tests read the 26.3 prepared root under a 1.8.9 binary:
/// a manifest that names another version keeps that root's own item table.
pub fn is_1_8_9_root(root: &Path) -> bool {
    if !horizon_core::target::is_1_8_9() {
        return false;
    }
    let Ok(bytes) = fs::read(root.join("horizon-resources.json")) else {
        return true;
    };
    let Ok(manifest) = serde_json::from_slice::<Value>(&bytes) else {
        return true;
    };
    manifest
        .get("minecraft")
        .and_then(Value::as_str)
        .is_none_or(|version| version == "1.8.9")
}

/// Every item's default components at `root`, by item id, with the mods'
/// changes applied and the stack size kept to vanilla's 1 to 99
/// (`ExtraCodecs.intRange`).
///
/// A 1.8 target keeps only the items 1.8.9 registered ([`crate::items_1_8_9`]):
/// the prepared root's `items.json` is still 26.3's table, and without the
/// filter commands, the creative menu and recipes reach netherite, elytra,
/// the 1.9+ spawn eggs and every other later item. Content mods' names
/// (anything outside `minecraft:`) always survive.
///
/// A root whose manifest names another Minecraft version keeps its table: the
/// tests that pin 26.3 algorithms still read the 26.3 prepared root even when
/// the binary targets 1.8.9. A root without a manifest (a temporary test
/// root) is the target's own.
pub fn read(root: &Path) -> io::Result<BTreeMap<String, Value>> {
    let mut items: BTreeMap<String, Value> =
        serde_json::from_slice(&fs::read(root.join("items.json"))?)?;
    if is_1_8_9_root(root) {
        items.retain(|item, _| {
            !item.starts_with("minecraft:") || crate::items_1_8_9::has_item(item)
        });
    }
    for (item, components) in &mut items {
        if let Value::Object(components) = components {
            native_modify_default_components(item, components);
            // ExtraCodecs.intRange(1, 99): nothing decodes a mod's change, and a
            // size of 0 would break stack arithmetic.
            if let Some(size) = components
                .get_mut("minecraft:max_stack_size")
                .filter(|size| size.is_number())
            {
                *size = size.as_f64().map_or(64, |n| (n as i64).clamp(1, 99)).into();
            }
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use std::fs;

    /// A stack size outside vanilla's codec range is kept to 1..=99 for every
    /// reader, so stack arithmetic never sees a 0.
    #[test]
    fn stack_sizes_stay_in_vanilla_range() {
        let root = std::env::temp_dir().join(format!("horizon-stack-sizes-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let items = r#"{"a:none": {"minecraft:max_stack_size": 0},
            "a:less": {"minecraft:max_stack_size": -5},
            "a:many": {"minecraft:max_stack_size": 500},
            "a:some": {"minecraft:max_stack_size": 16}, "a:unset": {}}"#;
        fs::write(root.join("items.json"), items).unwrap();
        let items = super::read(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        let size = |id: &str| items[id].get("minecraft:max_stack_size").cloned();
        assert_eq!(
            ["a:none", "a:less", "a:many", "a:some", "a:unset"].map(size),
            [
                Some(1.into()),
                Some(1.into()),
                Some(99.into()),
                Some(16.into()),
                None
            ]
        );
    }

    /// A 1.8 target exposes only the items 1.8.9 registered: the prepared
    /// root's `items.json` is still 26.3's, so a later item (netherite, the
    /// shulker's egg) and a later tab's item must not reach commands or the
    /// creative menu, while a 1.8 item and a content mod's item stay.
    #[test]
    fn post_1_8_items_are_dropped_on_the_1_8_target() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let root = std::env::temp_dir().join(format!("horizon-item-filter-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("items.json"),
            r#"{"minecraft:red_wool": {}, "minecraft:splash_potion": {},
                "minecraft:netherite_sword": {}, "minecraft:shulker_spawn_egg": {},
                "example:custom_item": {}}"#,
        )
        .unwrap();
        let items = super::read(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(items.contains_key("minecraft:red_wool"));
        assert!(items.contains_key("minecraft:splash_potion"));
        assert!(!items.contains_key("minecraft:netherite_sword"));
        assert!(!items.contains_key("minecraft:shulker_spawn_egg"));
        assert!(items.contains_key("example:custom_item"));
    }

    /// A root whose manifest names 26.3 keeps its table under a 1.8.9 binary:
    /// the tests of 26.3 algorithms read the 26.3 prepared root as their
    /// fixture, and must still see the items those algorithms use.
    #[test]
    fn a_26_3_manifest_keeps_its_own_item_table() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let root = std::env::temp_dir().join(format!("horizon-item-26-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("horizon-resources.json"),
            r#"{"minecraft": "26.3"}"#,
        )
        .unwrap();
        fs::write(
            root.join("items.json"),
            r#"{"minecraft:red_wool": {}, "minecraft:netherite_sword": {}}"#,
        )
        .unwrap();
        let items = super::read(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert!(items.contains_key("minecraft:red_wool"));
        assert!(items.contains_key("minecraft:netherite_sword"));
    }
}
