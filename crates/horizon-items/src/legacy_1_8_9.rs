//! 1.8.9 numeric item stacks.
//!
//! A 1.8 slot is `(id: i16, count: i8, damage: i16, nbt)`: numeric ids, no
//! namespaced names and no components. The datafixer already knows how to turn
//! a pre-1.13 stack into the current one, so this module builds the old NBT
//! shape and runs that pipeline instead of keeping a second flattening table.
use crate::items::ItemStack;
use horizon_world::{
    datafix::{self, CURRENT, flat_tables as t, schema::Ty},
    nbt::{JavaString, Tag},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

/// Decode a 1.8 slot. `id < 0` or `count <= 0` is an empty slot.
///
/// `damage` is both the metadata (wool colour, log species) and the wear of a
/// tool; the datafixer moves whichever applies into the current components.
/// `nbt` is the slot's optional root compound (`Tag::End` when absent).
pub fn stack(id: i16, count: i8, damage: i16, nbt: &Tag) -> Option<ItemStack> {
    if id < 0 || count <= 0 {
        return None;
    }
    let mut entries = vec![
        (
            "id",
            late_name(id).map_or(Tag::Int(i32::from(id)), |name| Tag::String(name.into())),
        ),
        ("Count", Tag::Byte(count)),
        ("Damage", Tag::Short(damage)),
    ];
    // `ItemPotionFix` reads the potion's damage only when the stack has a tag
    // compound and creates the `Potion` name there. A 1.8 potion carries its
    // potion in `Damage`, not in NBT, so give it the empty compound vanilla's
    // fixer expects (`ItemStack.setTag(new NBTTagCompound())`).
    if id == 373 && matches!(nbt, Tag::End) {
        entries.push(("tag", Tag::compound(std::iter::empty::<(&str, Tag)>())));
    } else if !matches!(nbt, Tag::End) {
        entries.push(("tag", nbt.clone()));
    }
    let mut tag = Tag::compound(entries);
    datafix::update(Ty::ItemStack, &mut tag, -1, CURRENT);
    crate::item_nbt::item_from_nbt(&tag)
}

/// The 1.8 item ids the datafixer's numeric table (the 1.7.10 `ItemIdFix`
/// list) predates: slime blocks, barriers, the 1.8 stone and prismarine
/// families, rabbit and mutton, banners and the wood-species doors. A stack
/// with one of these ids is written with its raw name instead of a number, so
/// the fixer's string branch keeps it and the 1.13 flattening can name it.
const LATE_ITEM_IDS: &[(i32, &str)] = &[
    (165, "minecraft:slime"),
    (166, "minecraft:barrier"),
    (167, "minecraft:iron_trapdoor"),
    (168, "minecraft:prismarine"),
    (169, "minecraft:sea_lantern"),
    (179, "minecraft:red_sandstone"),
    (180, "minecraft:red_sandstone_stairs"),
    (182, "minecraft:stone_slab2"),
    (183, "minecraft:spruce_fence_gate"),
    (184, "minecraft:birch_fence_gate"),
    (185, "minecraft:jungle_fence_gate"),
    (186, "minecraft:dark_oak_fence_gate"),
    (187, "minecraft:acacia_fence_gate"),
    (188, "minecraft:spruce_fence"),
    (189, "minecraft:birch_fence"),
    (190, "minecraft:jungle_fence"),
    (191, "minecraft:dark_oak_fence"),
    (192, "minecraft:acacia_fence"),
    (409, "minecraft:prismarine_shard"),
    (410, "minecraft:prismarine_crystals"),
    (411, "minecraft:rabbit"),
    (412, "minecraft:cooked_rabbit"),
    (413, "minecraft:rabbit_stew"),
    (414, "minecraft:rabbit_foot"),
    (415, "minecraft:rabbit_hide"),
    (416, "minecraft:armor_stand"),
    (423, "minecraft:mutton"),
    (424, "minecraft:cooked_mutton"),
    (425, "minecraft:banner"),
    (427, "minecraft:spruce_door"),
    (428, "minecraft:birch_door"),
    (429, "minecraft:jungle_door"),
    (430, "minecraft:acacia_door"),
    (431, "minecraft:dark_oak_door"),
];

/// The raw 1.8 name of one of the ids [`LATE_ITEM_IDS`] carries.
fn late_name(id: i16) -> Option<&'static str> {
    LATE_ITEM_IDS
        .iter()
        .find(|&&(late, _)| late == i32::from(id))
        .map(|&(_, name)| name)
}

/// The numeric id of a raw 1.8 item name (`minecraft:wool` → 35). The
/// flattened name lookup ([`legacy_id`]) does not know the raw name.
fn raw_name_id(name: &str) -> Option<i16> {
    // `Items.COOKED_FISH`'s 1.8 registry name. The datafixer's 1.7.10 numeric
    // table carries the pre-1.8 typo `cooked_fished`, so the raw name is not
    // in it.
    if name == "minecraft:cooked_fish" {
        return Some(350);
    }
    t::ITEM_IDS
        .iter()
        .find(|&&(_, item)| item == name)
        .map(|&(id, _)| id as i16)
        .or_else(|| {
            LATE_ITEM_IDS
                .iter()
                .find(|&&(_, item)| item == name)
                .map(|&(id, _)| id as i16)
        })
}

/// Decode a 1.8 slot whose `id` is the raw item name (`minecraft:wool`)
/// rather than the network number. `damage` is the 1.8 `Damage` byte, so the
/// colour or wear comes from the slot itself.
pub fn named_stack(name: &str, count: i8, damage: i16, nbt: &Tag) -> Option<ItemStack> {
    let id = legacy_id(name)
        .map(|(id, _)| id)
        .or_else(|| raw_name_id(name))?;
    stack(id, count, damage, nbt)
}

/// The 1.8 id and item damage for a current item name, if 1.8 had the item.
///
/// Several 1.8 states reshape into one current item (`red_wool` is wool with
/// metadata 14); the returned damage is the 1.8 `ItemStack.itemDamage` that
/// selects that shape, which is what [`legacy_stack`] puts on the wire.
pub fn legacy_id(name: &str) -> Option<(i16, i16)> {
    reverse()
        .first
        .get(name)
        .copied()
        .or_else(|| spawn_egg_damage(name).map(|damage| (383, damage)))
}

/// The 1.8 `EntityList.entityEggs` damage values.
///
/// 1.8 registers an egg only for a mob whose registration passed a primary
/// and secondary colour; that is these 27 ids, in `EntityList`'s static
/// initializer. The datafixer's damage-to-entity table is wider (it covers
/// every entity and the 1.9 eggs), so it cannot decide on its own whether a
/// current egg existed in 1.8.
const EGG_DAMAGE_1_8_9: [i16; 27] = [
    50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 65, 66, 67, 68, 90, 91, 92, 93, 94, 95, 96, 98,
    100, 101, 120,
];

/// The 1.8 `Items.spawn_egg` damage for a current egg item.
///
/// The damage is the 1.8 `EntityList` network id (`Creeper` is 50), which
/// `ItemSpawnEggFix` turns into an `EntityTag` on the way to the current
/// shape. 1.8.9's last egg is 120 (villager); the 1.9 shulker (69) and every
/// later mob have no 1.8 egg.
fn spawn_egg_damage(name: &str) -> Option<i16> {
    // The fixer's table is the 1.12 flattening, where the 1.8 `PigZombie` egg
    // is still `zombie_pigman_spawn_egg`; the current name arrived in 1.16.
    let name = match name {
        "minecraft:zombified_piglin_spawn_egg" => "minecraft:zombie_pigman_spawn_egg",
        name => name,
    };
    let entity = t::SPAWN_EGGS.iter().find(|&&(_, egg)| egg == name)?.0;
    // The fix's table uses the pre-flattening CamelCase entity names. 1.8's
    // horse is `EntityHorse`; the fixer renames it before `ENTITY_IDS` does.
    let wanted = if entity == "minecraft:horse" {
        "EntityHorse"
    } else {
        t::ENTITY_IDS
            .iter()
            .find(|&&(_, id)| id == entity)
            .map(|&(old, _)| old)?
    };
    t::SPAWN_EGG_ENTITIES
        .iter()
        .find(|&&(_, id)| id == wanted)
        .map(|&(damage, _)| damage as i16)
        .filter(|damage| EGG_DAMAGE_1_8_9.contains(damage))
}

/// The current item name a 1.8 numeric id stands for (its lowest damage).
///
/// 1.8 statistics name items by id (`stat.useItem.276`), so the statistics
/// screen needs the id-to-name direction; the first name that decodes from the
/// id wins, as [`legacy_id`]'s map does.
pub fn legacy_item_name(id: i16) -> Option<&'static str> {
    reverse().by_id.get(&id).map(|name| &**name)
}

/// The raw 1.8 name of a numeric item id (`minecraft:wool`, not
/// `minecraft:red_wool`). A 1.8 item stack's NBT `id` is this pre-flattening
/// name and its `Damage` selects the shape, unlike the network form, which
/// sends the number.
pub fn legacy_name(id: i16) -> Option<&'static str> {
    t::ITEM_IDS
        .binary_search_by_key(&i32::from(id), |&(id, _)| id)
        .ok()
        .map(|index| t::ITEM_IDS[index].1)
        .or_else(|| late_name(id))
}

/// `NbtHelper.fromGameProfile`: a current `minecraft:profile` (a skull's
/// `profile` tag or a head item's component) as the 1.8 GameProfile compound.
///
/// A bare string is a name. A compound carries `name`, `id` (a uuid string or
/// the four-int array a datafixed block entity uses) and the current
/// `properties` list of `{name, value, signature}`, which 1.8 regroups into
/// one list per property name with `Value`/`Signature`.
pub fn legacy_game_profile(profile: &Tag) -> Option<Tag> {
    match profile {
        Tag::String(name) => Some(Tag::compound([("Name", Tag::String(name.clone()))])),
        Tag::Compound(fields) => {
            let field = |key: &str| fields.get(&JavaString::from(key));
            let mut entries: Vec<(&str, Tag)> = Vec::new();
            if let Some(Tag::String(name)) = field("name") {
                entries.push(("Name", Tag::String(name.clone())));
            }
            if let Some(id) = field("id").and_then(profile_id_string) {
                entries.push(("Id", Tag::String(id.as_str().into())));
            }
            if let Some(Tag::List(properties)) = field("properties") {
                let mut grouped: BTreeMap<JavaString, Vec<Tag>> = BTreeMap::new();
                for property in properties {
                    let Some(name) = property.get("name").and_then(as_nbt_text) else {
                        continue;
                    };
                    let Some(value) = property.get("value").and_then(as_nbt_text) else {
                        continue;
                    };
                    let mut entry = vec![("Value", Tag::String(value.as_str().into()))];
                    if let Some(signature) = property.get("signature").and_then(as_nbt_text) {
                        entry.push(("Signature", Tag::String(signature.as_str().into())));
                    }
                    grouped
                        .entry(JavaString::from(name.as_str()))
                        .or_default()
                        .push(Tag::compound(entry));
                }
                if !grouped.is_empty() {
                    entries.push((
                        "Properties",
                        Tag::compound(
                            grouped
                                .into_iter()
                                .map(|(name, list)| (name, Tag::List(list))),
                        ),
                    ));
                }
            }
            (!entries.is_empty()).then(|| Tag::compound(entries))
        }
        _ => None,
    }
}

fn as_nbt_text(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(s) => Some(s.to_string_lossy()),
        _ => None,
    }
}

/// A profile `id` as the dashed uuid string 1.8's GameProfile NBT stores.
fn profile_id_string(id: &Tag) -> Option<String> {
    match id {
        Tag::String(s) => Some(s.to_string_lossy()),
        Tag::IntArray(words) if words.len() == 4 => {
            let hex: String = words.iter().map(|w| format!("{:08x}", *w as u32)).collect();
            Some(format!(
                "{}-{}-{}-{}-{}",
                &hex[..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..]
            ))
        }
        _ => None,
    }
}

/// Encode a current stack as a 1.8 slot, or `None` if 1.8 had no such item.
///
/// The count is clamped to the wire's signed byte (`1.8` stacks never exceed
/// 64). NBT survives for the fields 1.8 shares with the current model:
/// enchantments, a custom name, the potion and the anvil repair cost.
pub fn legacy_stack(stack: &ItemStack) -> Option<(i16, i8, i16, Tag)> {
    if stack.count == 0 {
        return None;
    }
    // A 1.8 potion keeps its potion in the slot's `Damage` (a bitmask), not in
    // a `Potion` NBT string: the same item id 373 is a splash potion with bit
    // 14 set. The datafixer names the modern potion, so map it back here.
    let potion = &*stack.item == "minecraft:potion" || &*stack.item == "minecraft:splash_potion";
    let potion_damage = potion.then(|| {
        let splash = &*stack.item == "minecraft:splash_potion";
        horizon_client_core::potions_1_8_9::legacy_damage(stack.potion(), splash).unwrap_or(0)
    });
    let (id, base_damage) = match potion_damage {
        Some(_) => (373, 0),
        None => legacy_id(&stack.item)?,
    };
    // A banner's damage is its dye metadata; the item name gives it, but a
    // `base_color` component (a 1.8 block broken into a white item, or a
    // command) is the colour 1.8 stores.
    let base_damage = if id == 425 {
        stack
            .data()
            .and_then(|data| data.base_color.as_deref())
            .and_then(crate::banner_layer::dye_meta)
            .map_or(base_damage, i16::from)
    } else {
        base_damage
    };
    // A worn tool keeps its wear; a reshaped block keeps its metadata.
    let damage = match potion_damage {
        Some(damage) => damage as i16,
        None if stack.damage > 0 => stack.damage.min(i16::MAX as u32) as i16,
        None => base_damage,
    };
    let tag = stack.data().map_or(Tag::End, |data| {
        let mut tag = legacy_tag(data, !potion);
        if id == 425 {
            tag = banner_tag(tag, damage, data);
        }
        if id == 397 {
            tag = skull_owner_tag(tag, data);
        }
        if matches!(tag, Tag::Compound(ref e) if e.is_empty()) {
            Tag::End
        } else {
            tag
        }
    });
    Some((id, stack.count.min(127) as i8, damage, tag))
}

/// A player head's `SkullOwner` from its `minecraft:profile` component.
fn skull_owner_tag(tag: Tag, data: &crate::item_data::StackData) -> Tag {
    let Some(owner) = data
        .other("minecraft:profile")
        .and_then(legacy_game_profile)
    else {
        return tag;
    };
    let Tag::Compound(mut entries) = tag else {
        return Tag::compound([("SkullOwner", owner)]);
    };
    entries.insert("SkullOwner".into(), owner);
    Tag::Compound(entries)
}

/// A current stack as the 1.8 item compound (`id` raw name, `Count`,
/// `Damage`, `tag`), for entity NBT and other places that hold a stack rather
/// than a slot list. `None` when 1.8 had no such item.
pub fn legacy_stack_tag(stack: &ItemStack) -> Option<Tag> {
    let (id, count, damage, tag) = legacy_stack(stack)?;
    let name = legacy_name(id)?;
    let mut entries = vec![
        ("id", Tag::String(name.into())),
        ("Count", Tag::Byte(count)),
        ("Damage", Tag::Short(damage)),
    ];
    if !matches!(tag, Tag::End) {
        entries.push(("tag", tag));
    }
    Some(Tag::compound(entries))
}

/// The 1.8 NBT for the components 1.8 also had. A potion's name is the
/// slot's `Damage`, not a `Potion` string, so `potion_tag` is false there.
fn legacy_tag(data: &crate::item_data::StackData, potion_tag: bool) -> Tag {
    let mut entries: Vec<(&str, Tag)> = Vec::new();
    if let Some(name) = &data.custom_name {
        entries.push(("display", Tag::compound([("Name", display_name(name))])));
    }
    if !data.enchantments.is_empty() {
        entries.push(("ench", enchantment_list(&data.enchantments)));
    }
    if !data.stored_enchantments.is_empty() {
        entries.push((
            "StoredEnchantments",
            enchantment_list(&data.stored_enchantments),
        ));
    }
    if data.repair_cost > 0 {
        entries.push(("RepairCost", Tag::Int(data.repair_cost as i32)));
    }
    if potion_tag && let Some(potion) = &data.potion {
        entries.push(("Potion", Tag::String((&**potion).into())));
    }
    // Book contents live in the item's own compound in 1.8, not in
    // components: a writable book's pages, a written book's signature.
    if !data.writable_pages.is_empty() {
        entries.push(("pages", book_pages(&data.writable_pages)));
    }
    if let Some(book) = &data.written {
        entries.push(("title", Tag::String(book.title.as_str().into())));
        entries.push(("author", Tag::String(book.author.as_str().into())));
        if book.generation > 0 {
            entries.push(("generation", Tag::Int(i32::from(book.generation))));
        }
        entries.push(("pages", book_pages(&book.pages)));
    }
    Tag::compound(entries)
}

/// The `BlockEntityTag` a 1.8 banner item carries: `Base` is the item's
/// damage (a `DyeColor.getSwappedId()` metadata) and `Patterns` the layers in
/// `BannerBlockEntity.toNbt`'s shape (`Pattern` short code, `Color` metadata).
fn banner_tag(tag: Tag, damage: i16, data: &crate::item_data::StackData) -> Tag {
    let Tag::Compound(mut entries) = tag else {
        return tag;
    };
    let mut banner = vec![("Base", Tag::Int(i32::from(damage)))];
    let patterns: Vec<Tag> = data
        .banner_patterns
        .iter()
        .filter(|layer| &*layer.pattern != "minecraft:base")
        .filter_map(|layer| {
            let code = datafix::legacy_banner_pattern(&layer.pattern)?;
            let color = crate::banner_layer::dye_meta(&layer.color).unwrap_or(0);
            Some(Tag::compound([
                ("Pattern", Tag::String(code.into())),
                ("Color", Tag::Int(i32::from(color))),
            ]))
        })
        .collect();
    if !patterns.is_empty() {
        banner.push(("Patterns", Tag::List(patterns)));
    }
    entries.insert("BlockEntityTag".into(), Tag::compound(banner));
    Tag::Compound(entries)
}

/// A book's pages as 1.8's list of strings.
fn book_pages(pages: &[String]) -> Tag {
    Tag::List(
        pages
            .iter()
            .map(|p| Tag::String(p.as_str().into()))
            .collect(),
    )
}

/// A text component as the JSON string 1.8 stores in `display.Name`.
///
/// A component that is already a JSON string is passed through; anything else
/// is flattened to `{"text": ...}`, which is what the 1.8 client parses.
fn display_name(name: &Tag) -> Tag {
    if let Tag::String(s) = name {
        let text = s.to_string_lossy();
        if text.trim_start().starts_with('{') {
            return name.clone();
        }
    }
    let plain = match name {
        Tag::String(s) => s.to_string_lossy(),
        other => format!("{other:?}"),
    };
    Tag::String(
        serde_json::json!({ "text": plain })
            .to_string()
            .as_str()
            .into(),
    )
}

/// `ench`/`StoredEnchantments`: `{id, lvl}` with 1.8's numeric ids.
fn enchantment_list(levels: &crate::item_data::Levels) -> Tag {
    let ids: BTreeMap<&str, i32> = t::ENCHANTMENTS
        .iter()
        .map(|&(id, name)| (name, id))
        .collect();
    Tag::List(
        levels
            .iter()
            .filter_map(|(name, level)| {
                let id = *ids.get(&**name)?;
                Some(Tag::compound([
                    ("id", Tag::Short(id as i16)),
                    ("lvl", Tag::Short(*level as i16)),
                ]))
            })
            .collect(),
    )
}

struct Reverse {
    /// Item name to the 1.8 id and the item damage that produces it.
    first: BTreeMap<Arc<str>, (i16, i16)>,
    /// The reverse direction: the first item name each id decodes to.
    by_id: BTreeMap<i16, Arc<str>>,
}

/// The item-name reverse index, built by decoding the same table the wire
/// decoder uses so encode and decode cannot drift.
fn reverse() -> &'static Reverse {
    static REVERSE: OnceLock<Reverse> = OnceLock::new();
    REVERSE.get_or_init(|| {
        let mut first: BTreeMap<Arc<str>, (i16, i16)> = BTreeMap::new();
        let mut by_id: BTreeMap<i16, Arc<str>> = BTreeMap::new();
        for &(id, _) in t::ITEM_IDS.iter().chain(LATE_ITEM_IDS) {
            if !(1..=i16::MAX as i32).contains(&id) {
                continue;
            }
            for damage in 0..16i16 {
                let Some(stack) = stack(id as i16, 1, damage, &Tag::End) else {
                    continue;
                };
                // A spawn egg's damage is an entity id, not a metadata variant;
                // damages 1..15 are not mobs and decode to the fixer's default
                // egg, which would claim that egg's damage. `spawn_egg_damage`
                // has the real table.
                if id != 383 {
                    first
                        .entry(stack.item.clone())
                        .or_insert((id as i16, damage));
                }
                by_id.entry(id as i16).or_insert(stack.item.clone());
            }
        }
        Reverse { first, by_id }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nbt(entries: impl IntoIterator<Item = (&'static str, Tag)>) -> Tag {
        Tag::compound(entries)
    }

    #[test]
    fn late_1_8_ids_use_their_raw_names() {
        // The datafixer's numeric table is the 1.7.10 one; these ids were
        // added in 1.8, so they travel as names and flatten from there.
        let slime = stack(165, 1, 0, &Tag::End).expect("slime block");
        assert_eq!(&*slime.item, "minecraft:slime_block");
        let banner = stack(425, 1, 0, &Tag::End).expect("banner");
        assert_eq!(&*banner.item, "minecraft:black_banner");
        let mutton = stack(424, 1, 0, &Tag::End).expect("cooked mutton");
        assert_eq!(&*mutton.item, "minecraft:cooked_mutton");
        let sandstone = stack(179, 1, 0, &Tag::End).expect("red sandstone");
        assert_eq!(&*sandstone.item, "minecraft:red_sandstone");
        assert_eq!(legacy_name(165), Some("minecraft:slime"));
        assert_eq!(legacy_item_name(424), Some("minecraft:cooked_mutton"));
        assert_eq!(legacy_id("minecraft:slime_block"), Some((165, 0)));
    }

    #[test]
    fn records_use_their_numeric_ids() {
        // 1.8's world event 1005 sends `Item.getIdFromItem`; the twelve
        // record items are 2256..2267.
        assert_eq!(legacy_item_name(2256), Some("minecraft:music_disc_13"));
        assert_eq!(legacy_item_name(2266), Some("minecraft:music_disc_11"));
        assert_eq!(legacy_item_name(2267), Some("minecraft:music_disc_wait"));
    }

    #[test]
    fn spawn_eggs_use_their_entity_damage() {
        // 1.8's egg is one item (383) whose damage is the entity's network id.
        assert_eq!(legacy_id("minecraft:creeper_spawn_egg"), Some((383, 50)));
        assert_eq!(legacy_id("minecraft:pig_spawn_egg"), Some((383, 90)));
        assert_eq!(legacy_id("minecraft:villager_spawn_egg"), Some((383, 120)));
        // Decoding the damage names the mob again, so a server round trip
        // keeps the egg.
        let egg = stack(383, 1, 50, &Tag::End).expect("creeper egg");
        assert_eq!(&*egg.item, "minecraft:creeper_spawn_egg");
        assert_eq!(legacy(egg), (383, 1, 50, Tag::End));
        // 1.9's shulker is not a 1.8 egg.
        assert_eq!(legacy_id("minecraft:shulker_spawn_egg"), None);
        // Nor is any mob 1.8's EntityList registered without egg colours,
        // even though the datafixer can name its current egg.
        assert_eq!(legacy_id("minecraft:ender_dragon_spawn_egg"), None);
        assert_eq!(legacy_id("minecraft:iron_golem_spawn_egg"), None);
        assert_eq!(legacy_id("minecraft:snow_golem_spawn_egg"), None);
        // The horse's entity id is 100 and the fixer's old name is EntityHorse.
        assert_eq!(legacy_id("minecraft:horse_spawn_egg"), Some((383, 100)));
        assert_eq!(
            legacy_id("minecraft:zombified_piglin_spawn_egg"),
            Some((383, 57))
        );
    }

    #[test]
    fn empty_slots_decode_to_nothing() {
        assert_eq!(stack(-1, 0, 0, &Tag::End), None);
        assert_eq!(stack(1, 0, 0, &Tag::End), None);
    }

    #[test]
    fn block_ids_and_metadata_become_the_flattened_item() {
        let stone = stack(1, 64, 0, &Tag::End).expect("stone");
        assert_eq!(&*stone.item, "minecraft:stone");
        assert_eq!(stone.count, 64);
        assert_eq!(stone.damage, 0);

        let red_wool = stack(35, 1, 14, &Tag::End).expect("red wool");
        assert_eq!(&*red_wool.item, "minecraft:red_wool");

        let acacia_log = stack(162, 1, 0, &Tag::End).expect("acacia log");
        assert_eq!(&*acacia_log.item, "minecraft:acacia_log");
    }

    #[test]
    fn raw_names_are_the_pre_flattening_ones() {
        // The NBT `id` of a 1.8 stack names the block, and `Damage` picks the
        // colour; the flattened name (`minecraft:red_wool`) is current-only.
        assert_eq!(legacy_name(35), Some("minecraft:wool"));
        assert_eq!(legacy_name(5), Some("minecraft:planks"));
        assert_eq!(legacy_name(276), Some("minecraft:diamond_sword"));
        assert_eq!(legacy_name(0), None);
    }

    #[test]
    fn a_raw_name_and_damage_decode_back_to_the_flattened_item() {
        let wool = named_stack("minecraft:wool", 2, 14, &Tag::End).expect("red wool");
        assert_eq!(&*wool.item, "minecraft:red_wool");
        assert_eq!(wool.count, 2);
        let planks = named_stack("minecraft:planks", 1, 4, &Tag::End).expect("acacia planks");
        assert_eq!(&*planks.item, "minecraft:acacia_planks");
        // A flattened name still works, for callers holding one.
        let sword = named_stack("minecraft:diamond_sword", 1, 5, &Tag::End).expect("sword");
        assert_eq!(sword.damage, 5);
    }

    #[test]
    fn tool_damage_becomes_the_damage_component() {
        let sword = stack(276, 1, 5, &Tag::End).expect("sword");
        assert_eq!(&*sword.item, "minecraft:diamond_sword");
        assert_eq!(sword.damage, 5);
    }

    #[test]
    fn the_slot_compound_survives_the_upgrade() {
        let tag = nbt([
            ("display", nbt([("Name", Tag::String("Blade".into()))])),
            ("Unbreakable", Tag::Byte(1)),
        ]);
        let sword = stack(276, 1, 0, &tag).expect("sword");
        assert_eq!(&*sword.item, "minecraft:diamond_sword");
        let data = sword.data().expect("components");
        assert!(data.custom_name.is_some());
        assert!(sword.data().is_some());
    }

    fn legacy(stack: ItemStack) -> (i16, i8, i16, Tag) {
        legacy_stack(&stack).expect("1.8 has this item")
    }

    #[test]
    fn decoded_stacks_encode_back_to_the_same_slot() {
        for (id, count, damage) in [(1i16, 64i8, 0i16), (35, 1, 14), (162, 3, 0), (276, 1, 0)] {
            let decoded = stack(id, count, damage, &Tag::End).expect("decodes");
            assert_eq!(
                legacy(decoded),
                (id, count, damage, Tag::End),
                "{id}:{damage}"
            );
        }
    }

    #[test]
    fn tool_wear_is_the_item_damage() {
        let mut sword = stack(276, 1, 0, &Tag::End).expect("sword");
        sword.damage = 500;
        assert_eq!(legacy(sword), (276, 1, 500, Tag::End));
    }

    #[test]
    fn a_custom_name_becomes_a_json_display_name() {
        let mut sword = stack(276, 1, 0, &Tag::End).expect("sword");
        sword.data_mut().custom_name = Some(Tag::String("Blade".into()));
        let (_, _, _, tag) = legacy(sword);
        let display = tag.get("display").expect("display");
        let name = display.get("Name").and_then(|n| match n {
            Tag::String(s) => Some(s.to_string_lossy()),
            _ => None,
        });
        assert_eq!(name.as_deref(), Some(r#"{"text":"Blade"}"#));
    }

    #[test]
    fn enchantments_use_the_1_8_numeric_ids() {
        let mut sword = stack(276, 1, 0, &Tag::End).expect("sword");
        sword.data_mut().enchantments = vec![("minecraft:sharpness".into(), 5)];
        let (_, _, _, tag) = legacy(sword);
        let Some(Tag::List(ench)) = tag.get("ench") else {
            panic!("no ench list in {tag:?}");
        };
        assert_eq!(
            ench[0],
            Tag::compound([("id", Tag::Short(16)), ("lvl", Tag::Short(5))])
        );
    }

    #[test]
    fn a_banner_carries_its_colour_and_patterns_in_block_entity_tag() {
        let mut banner = stack(425, 1, 15, &Tag::End).expect("white banner");
        assert_eq!(&*banner.item, "minecraft:white_banner");
        banner.data_mut().banner_patterns = vec![crate::banner_layer::BannerLayer::new(
            "minecraft:creeper",
            "lime",
        )];
        let (id, count, damage, tag) = legacy(banner);
        assert_eq!((id, count, damage), (425, 1, 15));
        let be = tag.get("BlockEntityTag").expect("BlockEntityTag");
        assert_eq!(be.get("Base"), Some(&Tag::Int(15)));
        let Some(Tag::List(patterns)) = be.get("Patterns") else {
            panic!("no Patterns in {be:?}");
        };
        // Lime's `DyeColor.getSwappedId()` is 10, and 1.8 names the creeper
        // pattern with the short code `cre`.
        assert_eq!(
            patterns[0],
            Tag::compound([
                ("Pattern", Tag::String("cre".into())),
                ("Color", Tag::Int(10)),
            ])
        );
        // A `base_color` component is the colour 1.8 writes, even when the
        // current item name is another one (a broken 1.8 block drops this).
        let mut banner = stack(425, 1, 15, &Tag::End).expect("white banner");
        banner.data_mut().base_color = Some("red".into());
        let (_, _, damage, tag) = legacy(banner);
        assert_eq!(damage, 1);
        assert_eq!(
            tag.get("BlockEntityTag").and_then(|be| be.get("Base")),
            Some(&Tag::Int(1))
        );
    }

    #[test]
    fn potions_travel_in_the_slot_damage() {
        // A 1.8 potion's `Damage` is the potion bitmask, not wear, and splash
        // potions are the same id 373 with bit 14. Neither carries a `Potion`
        // NBT string, so the datafixer names the modern potion from `Damage`.
        let swift = stack(373, 1, 8194, &Tag::End).expect("swiftness");
        assert_eq!(&*swift.item, "minecraft:potion");
        assert_eq!(swift.potion(), Some("minecraft:swiftness"));
        assert_eq!(legacy(swift), (373, 1, 8194, Tag::End));

        let splash = stack(373, 1, 16386, &Tag::End).expect("splash swiftness");
        assert_eq!(&*splash.item, "minecraft:splash_potion");
        assert_eq!(splash.potion(), Some("minecraft:swiftness"));
        assert_eq!(legacy(splash), (373, 1, 16386, Tag::End));

        let water = stack(373, 1, 0, &Tag::End).expect("water bottle");
        assert_eq!(&*water.item, "minecraft:potion");
        assert_eq!(water.potion(), Some("minecraft:water"));
        assert_eq!(legacy(water), (373, 1, 0, Tag::End));

        // A potion with other NBT keeps it; the potion itself stays in Damage.
        let named = stack(
            373,
            1,
            8258,
            &nbt([("display", nbt([("Name", Tag::String("Long".into()))]))]),
        )
        .expect("long swiftness");
        assert_eq!(named.potion(), Some("minecraft:long_swiftness"));
        let (_, _, damage, tag) = legacy(named);
        assert_eq!(damage, 8258);
        assert!(tag.get("Potion").is_none());
        assert!(tag.get("display").is_some());
    }

    #[test]
    fn current_potions_encode_as_1_8_damage() {
        let swift = ItemStack::new("minecraft:potion", 1).with_potion("minecraft:swiftness");
        assert_eq!(legacy(swift), (373, 1, 8194, Tag::End));
        let splash =
            ItemStack::new("minecraft:splash_potion", 1).with_potion("minecraft:long_swiftness");
        assert_eq!(legacy(splash), (373, 1, 16450, Tag::End));
        // 1.9+ potions have no 1.8 damage; they encode as a water bottle rather
        // than being dropped (documented 1.8 debt).
        let turtle = ItemStack::new("minecraft:potion", 1).with_potion("minecraft:turtle_master");
        assert_eq!(legacy(turtle), (373, 1, 0, Tag::End));
    }

    #[test]
    fn items_that_1_8_did_not_have_do_not_encode() {
        let sculk = ItemStack {
            item: "minecraft:sculk".into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        assert_eq!(legacy_stack(&sculk), None);
        let empty = ItemStack {
            item: "minecraft:stone".into(),
            count: 0,
            damage: 0,
            extra: None,
        };
        assert_eq!(legacy_stack(&empty), None);
    }

    #[test]
    fn a_player_head_keeps_its_owner_as_a_1_8_game_profile() {
        // `SkullBlock.onBreaking` copies the tile entity's `Owner`, a
        // `GameProfile` written by `NbtHelper.fromGameProfile`: `Name`, `Id`
        // as a uuid string and `Properties` grouped by property name.
        let profile = nbt([
            ("name", Tag::String("Steve".into())),
            (
                "id",
                Tag::String("00000000-0000-0000-0000-000000000001".into()),
            ),
            (
                "properties",
                Tag::List(vec![nbt([
                    ("name", Tag::String("textures".into())),
                    ("value", Tag::String("abc".into())),
                ])]),
            ),
        ]);
        let mut head = ItemStack::new("minecraft:player_head", 1);
        head.data_mut()
            .set_other("minecraft:profile", Some(profile));
        let (id, count, damage, tag) = legacy(head);
        assert_eq!((id, count, damage), (397, 1, 3));
        let owner = tag.get("SkullOwner").expect("SkullOwner");
        assert_eq!(owner.get("Name"), Some(&Tag::String("Steve".into())));
        assert_eq!(
            owner.get("Id"),
            Some(&Tag::String("00000000-0000-0000-0000-000000000001".into()))
        );
        let Some(Tag::Compound(properties)) = owner.get("Properties") else {
            panic!("no Properties in {owner:?}");
        };
        let textures = properties
            .get(&horizon_world::nbt::JavaString::from("textures"))
            .and_then(|list| match list {
                Tag::List(list) => list.first(),
                _ => None,
            })
            .expect("textures");
        assert_eq!(textures.get("Value"), Some(&Tag::String("abc".into())));
        // A skeleton skull has no owner and no item NBT.
        assert_eq!(
            legacy(ItemStack::new("minecraft:skeleton_skull", 1)),
            (397, 1, 0, Tag::End)
        );
    }
}
