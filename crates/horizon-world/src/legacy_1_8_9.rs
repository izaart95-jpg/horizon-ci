//! The numeric 1.8.9 block registry.
//!
//! A 1.8 block state is `id << 4 | metadata` (ids 0..=197). The datafixer's
//! `BlockStateData` table maps those legacy ids to the 1.13 states, and the
//! rest of the datafixer renames the states that changed after 1.13. This is a
//! **context-free** mapping: a few blocks (snowy grass, pumpkins, beds, skulls,
//! flower pots) also depend on neighbours or block entities. Whole chunks go
//! through [`crate::chunk_1_8_9::Chunk1_8::to_current_nbt`] instead, which runs
//! those fixes with their context.
use crate::{
    chunk::BlockState,
    datafix::{self, block_state_data, schema::Ty},
};
use std::{collections::BTreeMap, sync::OnceLock};

/// The highest block id that exists in 1.8.9.
pub const MAX_BLOCK_ID: u16 = 197;

/// 1.8's flower pot contents as `(tile entity Item, Data, modern block)`.
///
/// 1.8 has one block (id 140) with no metadata: the contents live in its
/// `FlowerPot` tile entity. 1.13 flattened that into one block per content.
/// The order is the one the 1.13 datafixer's pot map uses.
pub const POTTED_CONTENTS: [(&str, i32, &str); 22] = [
    ("minecraft:air", 0, "minecraft:flower_pot"),
    ("minecraft:red_flower", 0, "minecraft:potted_poppy"),
    ("minecraft:red_flower", 1, "minecraft:potted_blue_orchid"),
    ("minecraft:red_flower", 2, "minecraft:potted_allium"),
    ("minecraft:red_flower", 3, "minecraft:potted_azure_bluet"),
    ("minecraft:red_flower", 4, "minecraft:potted_red_tulip"),
    ("minecraft:red_flower", 5, "minecraft:potted_orange_tulip"),
    ("minecraft:red_flower", 6, "minecraft:potted_white_tulip"),
    ("minecraft:red_flower", 7, "minecraft:potted_pink_tulip"),
    ("minecraft:red_flower", 8, "minecraft:potted_oxeye_daisy"),
    ("minecraft:yellow_flower", 0, "minecraft:potted_dandelion"),
    ("minecraft:sapling", 0, "minecraft:potted_oak_sapling"),
    ("minecraft:sapling", 1, "minecraft:potted_spruce_sapling"),
    ("minecraft:sapling", 2, "minecraft:potted_birch_sapling"),
    ("minecraft:sapling", 3, "minecraft:potted_jungle_sapling"),
    ("minecraft:sapling", 4, "minecraft:potted_acacia_sapling"),
    ("minecraft:sapling", 5, "minecraft:potted_dark_oak_sapling"),
    ("minecraft:red_mushroom", 0, "minecraft:potted_red_mushroom"),
    (
        "minecraft:brown_mushroom",
        0,
        "minecraft:potted_brown_mushroom",
    ),
    ("minecraft:deadbush", 0, "minecraft:potted_dead_bush"),
    ("minecraft:tallgrass", 2, "minecraft:potted_fern"),
    ("minecraft:cactus", 0, "minecraft:potted_cactus"),
];

/// The 1.8 skull block (id 144) has no type in its metadata: `SkullType`
/// lives in the tile entity. The context-free mapping reads the skeleton
/// skull and the chunk converter ([`crate::datafix::chunk_flatten`]) picks
/// the typed state from the tile entity. `wall` variants face one of the
/// four horizontal directions.
pub const SKULLS: [(&str, bool); 12] = [
    ("minecraft:skeleton_skull", false),
    ("minecraft:wither_skeleton_skull", false),
    ("minecraft:zombie_head", false),
    ("minecraft:player_head", false),
    ("minecraft:creeper_head", false),
    ("minecraft:dragon_head", false),
    ("minecraft:skeleton_wall_skull", true),
    ("minecraft:wither_skeleton_wall_skull", true),
    ("minecraft:zombie_wall_head", true),
    ("minecraft:player_wall_head", true),
    ("minecraft:creeper_wall_head", true),
    ("minecraft:dragon_wall_head", true),
];

/// `SkullBlock.getData`/`getStateFromData` for id 144: metadata 0..5 is the
/// facing (`down`, `up`, then the four walls) and bit 3 is `NO_DROP` (the
/// modern `powered` property). 1.8 registers no metadata 6/7 or 14/15.
pub fn skull_state(meta: u8) -> BlockState {
    let meta = meta & 0x0F;
    let powered = meta & 8 != 0;
    let facing = meta & 7;
    let (name, property, value) = match facing {
        0 | 1 => ("minecraft:skeleton_skull", "rotation", "0"),
        2 => ("minecraft:skeleton_wall_skull", "facing", "north"),
        3 => ("minecraft:skeleton_wall_skull", "facing", "south"),
        4 => ("minecraft:skeleton_wall_skull", "facing", "west"),
        5 => ("minecraft:skeleton_wall_skull", "facing", "east"),
        _ => return air(),
    };
    let mut state = air();
    state.name = name.to_owned();
    state
        .properties
        .insert(property.to_owned(), value.to_owned());
    state
        .properties
        .insert("powered".to_owned(), powered.to_string());
    state
}

/// The 1.8 `SkullType` (the item damage, 0..=5) of a current skull/head block
/// name, floor or wall.
pub fn skull_type(name: &str) -> Option<u8> {
    let base = name.replace("_wall_", "_");
    SKULLS
        .iter()
        .enumerate()
        .find_map(|(i, (known, _))| (known.replace("_wall_", "_") == base).then_some((i % 6) as u8))
}

/// The current floor (`wall == false`) or wall skull/head block name of a
/// 1.8 `SkullType`.
pub fn skull_block_name(skull_type: u8, wall: bool) -> Option<&'static str> {
    let index = usize::from(skull_type) + if wall { 6 } else { 0 };
    let (name, known_wall) = *SKULLS.get(index)?;
    (known_wall == wall).then_some(name)
}

/// The modern pot block for a 1.8 tile entity's item and data.
pub fn potted_state(item: &str, data: i32) -> Option<BlockState> {
    let (_, _, name) = POTTED_CONTENTS
        .iter()
        .find(|(known, known_data, _)| *known == item && *known_data == data)?;
    let mut state = air();
    state.name = (*name).to_owned();
    Some(state)
}

/// The 1.8 item and data a modern potted block saves as, for the `FlowerPot`
/// tile entity. The empty pot writes `minecraft:air`.
pub fn potted_item(name: &str) -> Option<(&'static str, i32)> {
    POTTED_CONTENTS
        .iter()
        .find(|(_, _, known)| *known == name)
        .map(|(item, data, _)| (*item, *data))
}

/// Whether 1.8's block-state table names this id and metadata. The table's
/// `finalizeMaps` defaults fill every metadata of a block, but the runtime
/// only creates the states `register` named; an unregistered pair reads as air.
pub fn is_registered(id: u16, meta: u8) -> bool {
    block_state_data::is_registered(i32::from(id) << 4 | i32::from(meta & 0x0F))
}

/// The 1.8 state for a block id and metadata, without any context.
///
/// `meta` is masked to four bits, as on the wire. Unknown ids are air. A
/// metadata value that the 1.12 state table dropped still reads through the
/// datafixer's per-block default (1.8 snowy mycelium is the one the block
/// writes back below).
pub fn block_state(id: u16, meta: u8) -> BlockState {
    if id > MAX_BLOCK_ID {
        return air();
    }
    // 1.8's flower pot has no metadata: its contents are in the tile entity.
    // The context-free datafixer can only pick one content for the bare id,
    // so read the block as the empty pot and let the chunk converter apply
    // the tile entity (`block_state` has no context).
    if id == 140 {
        let mut pot = air();
        pot.name = "minecraft:flower_pot".to_owned();
        return pot;
    }
    // Skulls: the type is in the tile entity, not the metadata (see `SKULLS`).
    if id == 144 {
        return skull_state(meta);
    }
    let meta = meta & 0x0F;
    let mut tag = block_state_data::get_tag(i32::from(id & 0x0FFF) << 4 | i32::from(meta));
    datafix::update(Ty::BlockState, &mut tag, 1631, datafix::CURRENT);
    let mut state = BlockState::from_nbt(&tag).unwrap_or_else(|_| air());
    // 1.8 stores the snowy flag in the grass and mycelium metadata; 1.12
    // dropped it (the snow layer above is the state there), so the table
    // defaults both ids to `snowy=false`.
    if meta & 1 == 1
        && matches!(
            state.name.as_str(),
            "minecraft:grass_block" | "minecraft:mycelium"
        )
    {
        state
            .properties
            .insert("snowy".to_owned(), "true".to_owned());
    }
    // 1.13's flattening renamed the leaf properties: `decayable` (whether the
    // leaf decays at all) became the inverted `persistent`, and `check_decay`
    // became `distance` (7 while the leaf still searches for logs, 0 when it
    // does not). The engine's leaf decay and the current worldgen files speak
    // the current names.
    if state.name.ends_with("_leaves")
        && let Some(decayable) = state.properties.remove("decayable")
    {
        let check_decay = state
            .properties
            .remove("check_decay")
            .is_some_and(|value| value == "true");
        state.properties.insert(
            "persistent".to_owned(),
            if decayable == "false" {
                "true"
            } else {
                "false"
            }
            .to_owned(),
        );
        state.properties.insert(
            "distance".to_owned(),
            if check_decay { "7" } else { "0" }.to_owned(),
        );
    }
    state
}

/// The 1.8 state as its wire integer (`id << 4 | metadata`).
pub fn block_state_id(id: u16, meta: u8) -> u16 {
    id << 4 | u16::from(meta & 0x0F)
}

/// The 1.8 state packed by `Block.getByBlockState`: the block id in the low
/// 12 bits and the metadata in the top 4. Block particles and world event
/// 2001 send a state this way.
pub fn block_state_from_packed(data: i32) -> BlockState {
    block_state((data & 0x0FFF) as u16, ((data >> 12) & 0x0F) as u8)
}

/// The canonical 1.8 id and metadata for a current block state, if 1.8 had it.
///
/// This is the inverse of [`block_state`] for states 1.8 can express. Several
/// 1.8 states flatten to the same current state (still and flowing water at
/// level 0, unused metadata of a block); the lowest canonical metadata wins,
/// and source fluids prefer the still id (`9` water, `11` lava) over the
/// flowing one. States added after 1.8 return `None`.
pub fn legacy_block_state(state: &BlockState) -> Option<(u16, u8)> {
    static REVERSE: OnceLock<BTreeMap<BlockState, (u16, u8)>> = OnceLock::new();
    let map = REVERSE.get_or_init(|| {
        let mut map = BTreeMap::new();
        for id in 0..=MAX_BLOCK_ID {
            for meta in 0..16 {
                // The 1.8 runtime only holds a state for metadata vanilla
                // registered (a chest starts at meta 2, a furnace at meta 2);
                // the others are datafixer defaults that read back as air.
                if !block_state_data::is_registered(i32::from(id) << 4 | i32::from(meta)) {
                    continue;
                }
                // Metadata 0 of a skull is `facing=down`; the flattened floor
                // skull has no such state, and floor skulls save as 1 (`up`).
                if id == 144 && meta == 0 {
                    continue;
                }
                map.entry(block_state(id, meta)).or_insert((id, meta));
            }
        }
        // Source fluids must save as the still ids, not the flowing ones.
        for (id, meta) in [(9, 0), (11, 0)] {
            map.insert(block_state(id, meta), (id, meta));
        }
        // 1.12 dropped the snowy flag from grass and mycelium and kept one
        // table id for the dry state; 1.8 stores the flag in the metadata, so
        // the snowy states need their own reverse entries.
        for (id, meta) in [(2, 0), (2, 1), (110, 0), (110, 1)] {
            map.entry(block_state(id, meta)).or_insert((id, meta));
        }
        // 1.8 has one pumpkin block; 1.13 split the uncarved one off. A whole
        // chunk conversion keeps both names, so accept the uncarved alias.
        for meta in 0..4 {
            let mut pumpkin = block_state(86, meta);
            pumpkin.name = "minecraft:pumpkin".to_owned();
            map.entry(pumpkin).or_insert((86, meta));
        }
        // Every flower pot content saves as the one 1.8 block id; the save
        // writer adds the `FlowerPot` tile entity that holds the content.
        for (_, _, name) in POTTED_CONTENTS {
            let mut pot = air();
            pot.name = name.to_owned();
            map.insert(pot, (140, 0));
        }
        // 1.8 skulls: every type shares block id 144; the type is the item
        // damage (and `SkullType` in the tile entity). Floors use metadata 1
        // (`facing=up`) with the rotation in the tile entity, walls metadata
        // 2..5, and bit 3 is `NO_DROP` (the modern `powered` property).
        for (name, wall) in SKULLS {
            let mut state = air();
            state.name = name.to_owned();
            if wall {
                for (facing, meta) in [("north", 2u8), ("south", 3), ("west", 4), ("east", 5)] {
                    for powered in [false, true] {
                        let mut state = state.clone();
                        state
                            .properties
                            .insert("facing".to_owned(), facing.to_owned());
                        state
                            .properties
                            .insert("powered".to_owned(), powered.to_string());
                        map.insert(state, (144, meta | if powered { 8 } else { 0 }));
                    }
                }
            } else {
                for rotation in 0..16u8 {
                    for powered in [false, true] {
                        let mut state = state.clone();
                        state
                            .properties
                            .insert("rotation".to_owned(), rotation.to_string());
                        state
                            .properties
                            .insert("powered".to_owned(), powered.to_string());
                        map.insert(state, (144, 1 | if powered { 8 } else { 0 }));
                    }
                }
            }
        }
        map
    });
    if let Some(found) = map.get(state) {
        return Some(*found);
    }
    // 1.13 renamed the leaf properties. 1.8 only records "always checks"
    // or "never checks", so every distance the engine produces (1..=7) saves
    // as `check_decay=true`; loading it back starts the search again. The
    // reverse map is keyed by current-shaped states, so normalize the
    // distance instead of translating the property names here.
    if state.name.ends_with("_leaves")
        && state.properties.contains_key("persistent")
        && let Some(distance) = state.properties.get("distance")
        && distance != "0"
    {
        let mut canonical = state.clone();
        canonical
            .properties
            .insert("distance".to_owned(), "7".to_owned());
        return map.get(&canonical).copied();
    }
    None
}

/// The 1.8 name of a current block (its pre-flattening id name), for the
/// `i` of a 1.8 `TileTicks` entry. `None` when 1.8 had no such block.
pub fn legacy_block_name(name: &str) -> Option<&'static str> {
    static NAMES: OnceLock<BTreeMap<String, &'static str>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        use crate::datafix::flat_tables::BLOCK_IDS;
        let mut names = BTreeMap::new();
        for id in 0..=MAX_BLOCK_ID {
            let Some(&(legacy, _)) = BLOCK_IDS
                .iter()
                .find(|&&(_, block_id)| block_id == i32::from(id))
            else {
                continue;
            };
            for meta in 0..16 {
                let state = block_state(id, meta);
                names.entry(state.name).or_insert(legacy);
            }
        }
        names
    });
    names.get(name).copied()
}

/// The nearest 1.8 block for a current block name the 1.8 registry lacks.
///
/// The 1.8 build still reads 26.3 world generation (the debt in
/// `docs/mc-1.8.9.md`), whose surface rules, features and processors name
/// blocks 1.8 never had. Mapping the common stone, nether, wood and plant
/// families keeps the terrain and structures shaped the same; content with no
/// 1.8 counterpart maps to air, which makes the feature a no-op.
pub fn legacy_alias(name: &str) -> Option<&'static str> {
    // Explicit pairs rather than prefix rules: every alias is a decision
    // about what a player should see in the 1.8 world.
    static ALIASES: &[(&str, &str)] = &[
        ("minecraft:acacia_wood", "minecraft:acacia_bark"),
        ("minecraft:amethyst_block", "minecraft:air"),
        ("minecraft:amethyst_cluster", "minecraft:air"),
        ("minecraft:ancient_debris", "minecraft:obsidian"),
        ("minecraft:attached_melon_stem", "minecraft:melon_stem"),
        ("minecraft:attached_pumpkin_stem", "minecraft:pumpkin_stem"),
        ("minecraft:azalea", "minecraft:air"),
        ("minecraft:azalea_leaves", "minecraft:oak_leaves"),
        ("minecraft:bamboo", "minecraft:air"),
        ("minecraft:basalt", "minecraft:netherrack"),
        ("minecraft:beetroots", "minecraft:air"),
        ("minecraft:big_dripleaf", "minecraft:air"),
        ("minecraft:big_dripleaf_stem", "minecraft:air"),
        ("minecraft:birch_wood", "minecraft:birch_bark"),
        ("minecraft:blackstone", "minecraft:netherrack"),
        ("minecraft:blue_ice", "minecraft:packed_ice"),
        ("minecraft:brain_coral_block", "minecraft:air"),
        ("minecraft:bubble_coral_block", "minecraft:air"),
        ("minecraft:budding_amethyst", "minecraft:air"),
        ("minecraft:bush", "minecraft:air"),
        ("minecraft:cactus_flower", "minecraft:air"),
        ("minecraft:calcite", "minecraft:stone"),
        ("minecraft:campfire", "minecraft:air"),
        ("minecraft:cave_vines", "minecraft:air"),
        ("minecraft:cave_vines_plant", "minecraft:air"),
        ("minecraft:cherry_leaves", "minecraft:oak_leaves"),
        ("minecraft:cherry_log", "minecraft:oak_log"),
        ("minecraft:cherry_sapling", "minecraft:air"),
        ("minecraft:chiseled_deepslate", "minecraft:stone_bricks"),
        (
            "minecraft:chiseled_polished_blackstone",
            "minecraft:stone_bricks",
        ),
        ("minecraft:chorus_flower", "minecraft:air"),
        ("minecraft:chorus_plant", "minecraft:air"),
        ("minecraft:cinnabar", "minecraft:stone"),
        ("minecraft:closed_eyeblossom", "minecraft:air"),
        ("minecraft:cobbled_deepslate", "minecraft:cobblestone"),
        ("minecraft:copper_ore", "minecraft:stone"),
        ("minecraft:cornflower", "minecraft:air"),
        (
            "minecraft:cracked_deepslate_bricks",
            "minecraft:cracked_stone_bricks",
        ),
        (
            "minecraft:cracked_deepslate_tiles",
            "minecraft:cracked_stone_bricks",
        ),
        (
            "minecraft:cracked_polished_blackstone_bricks",
            "minecraft:cracked_stone_bricks",
        ),
        ("minecraft:creaking_heart", "minecraft:air"),
        ("minecraft:crimson_fungus", "minecraft:air"),
        ("minecraft:crimson_nylium", "minecraft:netherrack"),
        ("minecraft:crimson_roots", "minecraft:air"),
        ("minecraft:crimson_stem", "minecraft:netherrack"),
        ("minecraft:dark_oak_wood", "minecraft:dark_oak_bark"),
        ("minecraft:deepslate", "minecraft:stone"),
        ("minecraft:deepslate_bricks", "minecraft:stone_bricks"),
        ("minecraft:deepslate_coal_ore", "minecraft:coal_ore"),
        ("minecraft:deepslate_copper_ore", "minecraft:stone"),
        ("minecraft:deepslate_diamond_ore", "minecraft:diamond_ore"),
        ("minecraft:deepslate_emerald_ore", "minecraft:emerald_ore"),
        ("minecraft:deepslate_gold_ore", "minecraft:gold_ore"),
        ("minecraft:deepslate_iron_ore", "minecraft:iron_ore"),
        ("minecraft:deepslate_lapis_ore", "minecraft:lapis_ore"),
        ("minecraft:deepslate_redstone_ore", "minecraft:redstone_ore"),
        (
            "minecraft:deepslate_tile_slab",
            "minecraft:stone_brick_slab",
        ),
        ("minecraft:deepslate_tiles", "minecraft:stone_bricks"),
        ("minecraft:dirt_path", "minecraft:dirt"),
        ("minecraft:dripstone_block", "minecraft:stone"),
        ("minecraft:end_gateway", "minecraft:air"),
        ("minecraft:fire_coral_block", "minecraft:air"),
        ("minecraft:firefly_bush", "minecraft:air"),
        ("minecraft:flowering_azalea", "minecraft:air"),
        ("minecraft:flowering_azalea_leaves", "minecraft:oak_leaves"),
        ("minecraft:gilded_blackstone", "minecraft:netherrack"),
        ("minecraft:glow_lichen", "minecraft:air"),
        ("minecraft:hanging_roots", "minecraft:air"),
        ("minecraft:horn_coral_block", "minecraft:air"),
        ("minecraft:infested_deepslate", "minecraft:stone"),
        ("minecraft:jigsaw", "minecraft:air"),
        ("minecraft:jungle_wood", "minecraft:jungle_bark"),
        ("minecraft:kelp", "minecraft:air"),
        ("minecraft:kelp_plant", "minecraft:air"),
        ("minecraft:lantern", "minecraft:torch"),
        ("minecraft:large_amethyst_bud", "minecraft:air"),
        ("minecraft:leaf_litter", "minecraft:air"),
        ("minecraft:lily_of_the_valley", "minecraft:air"),
        ("minecraft:magma_block", "minecraft:netherrack"),
        ("minecraft:mangrove_leaves", "minecraft:oak_leaves"),
        ("minecraft:mangrove_log", "minecraft:oak_log"),
        ("minecraft:mangrove_propagule", "minecraft:air"),
        ("minecraft:mangrove_roots", "minecraft:oak_log"),
        ("minecraft:medium_amethyst_bud", "minecraft:air"),
        ("minecraft:melon", "minecraft:melon_block"),
        ("minecraft:moss_block", "minecraft:air"),
        ("minecraft:moss_carpet", "minecraft:air"),
        ("minecraft:mud", "minecraft:dirt"),
        ("minecraft:mud_bricks", "minecraft:bricks"),
        ("minecraft:muddy_mangrove_roots", "minecraft:dirt"),
        ("minecraft:nether_gold_ore", "minecraft:nether_quartz_ore"),
        ("minecraft:nether_sprouts", "minecraft:air"),
        ("minecraft:nether_wart_block", "minecraft:netherrack"),
        ("minecraft:oak_wood", "minecraft:oak_bark"),
        ("minecraft:orange_poplar_leaves", "minecraft:birch_leaves"),
        ("minecraft:packed_mud", "minecraft:dirt"),
        ("minecraft:pale_moss_block", "minecraft:air"),
        ("minecraft:pale_moss_carpet", "minecraft:air"),
        ("minecraft:pale_oak_leaves", "minecraft:oak_leaves"),
        ("minecraft:pale_oak_log", "minecraft:oak_log"),
        ("minecraft:pale_oak_sapling", "minecraft:air"),
        ("minecraft:pink_petals", "minecraft:air"),
        ("minecraft:pitcher_crop", "minecraft:air"),
        ("minecraft:pointed_dripstone", "minecraft:air"),
        (
            "minecraft:polished_blackstone_bricks",
            "minecraft:stone_bricks",
        ),
        ("minecraft:poplar_log", "minecraft:birch_log"),
        ("minecraft:poplar_sapling", "minecraft:air"),
        ("minecraft:potent_sulfur", "minecraft:stone"),
        ("minecraft:powder_snow", "minecraft:snow_block"),
        ("minecraft:pumpkin", "minecraft:carved_pumpkin"),
        ("minecraft:raw_copper_block", "minecraft:stone"),
        ("minecraft:raw_gold_block", "minecraft:gold_block"),
        ("minecraft:raw_iron_block", "minecraft:iron_block"),
        ("minecraft:red_poplar_leaves", "minecraft:birch_leaves"),
        ("minecraft:red_shrub", "minecraft:air"),
        ("minecraft:rooted_dirt", "minecraft:dirt"),
        ("minecraft:sculk_catalyst", "minecraft:air"),
        ("minecraft:sculk_shrieker", "minecraft:air"),
        ("minecraft:sculk_vein", "minecraft:air"),
        ("minecraft:sea_pickle", "minecraft:air"),
        ("minecraft:seagrass", "minecraft:air"),
        ("minecraft:shelf_mushroom", "minecraft:air"),
        ("minecraft:shroomlight", "minecraft:glowstone"),
        ("minecraft:short_dry_grass", "minecraft:air"),
        ("minecraft:small_amethyst_bud", "minecraft:air"),
        ("minecraft:small_dripleaf", "minecraft:air"),
        ("minecraft:smooth_basalt", "minecraft:stone"),
        ("minecraft:smooth_sandstone", "minecraft:sandstone"),
        (
            "minecraft:smooth_sandstone_slab",
            "minecraft:sandstone_slab",
        ),
        (
            "minecraft:smooth_sandstone_stairs",
            "minecraft:sandstone_stairs",
        ),
        ("minecraft:soul_fire", "minecraft:fire"),
        ("minecraft:soul_lantern", "minecraft:torch"),
        ("minecraft:soul_soil", "minecraft:soul_sand"),
        ("minecraft:spore_blossom", "minecraft:air"),
        ("minecraft:spruce_wood", "minecraft:spruce_bark"),
        ("minecraft:stripped_acacia_log", "minecraft:acacia_log"),
        ("minecraft:stripped_acacia_wood", "minecraft:acacia_bark"),
        ("minecraft:stripped_birch_log", "minecraft:birch_log"),
        ("minecraft:stripped_birch_wood", "minecraft:birch_bark"),
        ("minecraft:stripped_dark_oak_log", "minecraft:dark_oak_log"),
        (
            "minecraft:stripped_dark_oak_wood",
            "minecraft:dark_oak_bark",
        ),
        ("minecraft:stripped_jungle_log", "minecraft:jungle_log"),
        ("minecraft:stripped_jungle_wood", "minecraft:jungle_bark"),
        ("minecraft:stripped_oak_log", "minecraft:oak_log"),
        ("minecraft:stripped_oak_wood", "minecraft:oak_bark"),
        ("minecraft:stripped_spruce_log", "minecraft:spruce_log"),
        ("minecraft:stripped_spruce_wood", "minecraft:spruce_bark"),
        ("minecraft:sulfur", "minecraft:stone"),
        ("minecraft:sulfur_spike", "minecraft:air"),
        ("minecraft:suspicious_gravel", "minecraft:gravel"),
        ("minecraft:suspicious_sand", "minecraft:sand"),
        ("minecraft:sweet_berry_bush", "minecraft:air"),
        ("minecraft:tall_dry_grass", "minecraft:air"),
        ("minecraft:tall_seagrass", "minecraft:air"),
        ("minecraft:torchflower", "minecraft:air"),
        ("minecraft:torchflower_crop", "minecraft:air"),
        ("minecraft:tube_coral_block", "minecraft:air"),
        ("minecraft:tuff", "minecraft:stone"),
        ("minecraft:twisting_vines", "minecraft:air"),
        ("minecraft:twisting_vines_plant", "minecraft:air"),
        ("minecraft:warped_fungus", "minecraft:air"),
        ("minecraft:warped_nylium", "minecraft:netherrack"),
        ("minecraft:warped_roots", "minecraft:air"),
        ("minecraft:warped_stem", "minecraft:netherrack"),
        ("minecraft:warped_wart_block", "minecraft:netherrack"),
        ("minecraft:waxed_copper_bulb", "minecraft:glowstone"),
        ("minecraft:waxed_exposed_copper_bulb", "minecraft:glowstone"),
        (
            "minecraft:waxed_oxidized_copper_bulb",
            "minecraft:glowstone",
        ),
        (
            "minecraft:waxed_weathered_copper_bulb",
            "minecraft:glowstone",
        ),
        ("minecraft:weeping_vines", "minecraft:air"),
        ("minecraft:weeping_vines_plant", "minecraft:air"),
        ("minecraft:wildflowers", "minecraft:air"),
        ("minecraft:wither_rose", "minecraft:air"),
        ("minecraft:yellow_poplar_leaves", "minecraft:birch_leaves"),
    ];
    ALIASES
        .iter()
        .find(|(from, _)| *from == name)
        .map(|(_, to)| *to)
}

fn air() -> BlockState {
    BlockState {
        name: "minecraft:air".to_owned(),
        properties: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(id: u16, meta: u8) -> (String, Vec<(String, String)>) {
        let state = block_state(id, meta);
        (
            state.name,
            state
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }

    fn prop<'a>(props: &'a [(String, String)], name: &str) -> &'a str {
        props
            .iter()
            .find(|(k, _)| k == name)
            .map_or("", |(_, v)| v.as_str())
    }

    #[test]
    fn simple_blocks_keep_their_identity() {
        assert_eq!(state(0, 0).0, "minecraft:air");
        assert_eq!(state(1, 0).0, "minecraft:stone");
        assert_eq!(state(4, 0).0, "minecraft:cobblestone");
        assert_eq!(state(20, 0).0, "minecraft:glass");
        assert_eq!(state(45, 0).0, "minecraft:bricks");
        assert_eq!(state(49, 0).0, "minecraft:obsidian");
    }

    #[test]
    fn effect_data_packs_the_id_low_and_the_metadata_high() {
        let packed = i32::from(17 | (2 << 12));
        assert_eq!(block_state_from_packed(packed), block_state(17, 2));
        assert_eq!(block_state_from_packed(packed).name, "minecraft:birch_log");
        assert_eq!(
            block_state_from_packed(i32::from(MAX_BLOCK_ID | (1 << 12))),
            block_state(MAX_BLOCK_ID, 1)
        );
    }

    #[test]
    fn metadata_selects_the_flat_variant() {
        assert_eq!(state(5, 0).0, "minecraft:oak_planks");
        assert_eq!(state(5, 1).0, "minecraft:spruce_planks");
        assert_eq!(state(5, 5).0, "minecraft:dark_oak_planks");
        assert_eq!(state(35, 14).0, "minecraft:red_wool");
        assert_eq!(state(95, 11).0, "minecraft:blue_stained_glass");
        assert_eq!(state(162, 0).0, "minecraft:acacia_log");
    }

    #[test]
    fn skull_metadata_is_a_facing_and_a_no_drop_flag() {
        // 1.8 metadata 1 is a floor skull (`facing=up`); the rotation and the
        // SkullType live in the tile entity, so the context-free state is the
        // skeleton floor skull at rotation 0.
        let floor = block_state(144, 1);
        assert_eq!(floor.name, "minecraft:skeleton_skull");
        assert_eq!(
            floor.properties.get("rotation").map(String::as_str),
            Some("0")
        );
        assert_eq!(
            floor.properties.get("powered").map(String::as_str),
            Some("false")
        );
        // 2..5 are walls north, south, west, east; bit 3 is NO_DROP.
        assert_eq!(block_state(144, 2).name, "minecraft:skeleton_wall_skull");
        assert_eq!(
            block_state(144, 2)
                .properties
                .get("facing")
                .map(String::as_str),
            Some("north")
        );
        assert_eq!(
            block_state(144, 10)
                .properties
                .get("powered")
                .map(String::as_str),
            Some("true")
        );
        // Unregistered facings read as air.
        for meta in [6u8, 7, 14, 15] {
            assert_eq!(block_state(144, meta).name, "minecraft:air");
        }
        // Every typed skull state saves back to id 144 with its own metadata,
        // floor states to `facing=up`.
        for (name, wall) in SKULLS {
            let mut state = air();
            state.name = name.to_owned();
            if wall {
                state
                    .properties
                    .insert("facing".to_owned(), "south".to_owned());
            } else {
                state
                    .properties
                    .insert("rotation".to_owned(), "9".to_owned());
            }
            state
                .properties
                .insert("powered".to_owned(), "false".to_owned());
            let expected = if wall { 3 } else { 1 };
            assert_eq!(legacy_block_state(&state), Some((144, expected)), "{name}");
        }
        // The tile entity's type and the wall flag pick the typed name; 1.8
        // has six types, floor and wall.
        assert_eq!(skull_type("minecraft:player_head"), Some(3));
        assert_eq!(skull_type("minecraft:player_wall_head"), Some(3));
        assert_eq!(skull_block_name(3, false), Some("minecraft:player_head"));
        assert_eq!(
            skull_block_name(5, true),
            Some("minecraft:dragon_wall_head")
        );
        assert_eq!(skull_block_name(6, false), None);
        assert_eq!(skull_type("minecraft:stone"), None);
    }

    #[test]
    fn flower_pot_contents_round_trip_through_the_1_8_tile_entity() {
        // 1.8's flower pot has no metadata: every meta is the bare pot, and
        // the contents come from the tile entity.
        for meta in 0..16 {
            assert_eq!(state(140, meta).0, "minecraft:flower_pot");
        }
        for (item, data, name) in POTTED_CONTENTS {
            let pot = potted_state(item, data).unwrap_or_else(|| panic!("{item}:{data}"));
            assert_eq!(pot.name, name);
            assert_eq!(potted_item(name), Some((item, data)));
            assert_eq!(legacy_block_state(&pot), Some((140, 0)));
        }
        assert_eq!(
            potted_state("minecraft:red_flower", 3).unwrap().name,
            "minecraft:potted_azure_bluet"
        );
        assert!(potted_state("minecraft:red_flower", 15).is_none());
        assert!(potted_item("minecraft:potted_azalea_bush").is_none());
    }

    #[test]
    fn post_1_13_renames_are_applied() {
        // 1.8 `tallgrass` (id 31) meta 1 is the plant that 1.13 called
        // `grass` and 1.20.3 renamed to `short_grass`.
        let (name, _) = state(31, 1);
        assert_eq!(name, "minecraft:short_grass");
    }

    #[test]
    fn snowy_grass_and_mycelium_keep_the_1_8_flag() {
        let (name, props) = state(2, 0);
        assert_eq!(name, "minecraft:grass_block");
        assert_eq!(prop(&props, "snowy"), "false");
        let (name, props) = state(2, 1);
        assert_eq!(name, "minecraft:grass_block");
        assert_eq!(prop(&props, "snowy"), "true");

        let (name, props) = state(110, 1);
        assert_eq!(name, "minecraft:mycelium");
        assert_eq!(prop(&props, "snowy"), "true");
    }

    #[test]
    fn leaves_use_the_current_property_names() {
        // 1.8 metadata 4 is "player placed" (`decayable=false`), bit 8 is
        // "still checks for logs" (`check_decay`). 1.13 renamed both.
        let (name, props) = state(18, 0);
        assert_eq!(name, "minecraft:oak_leaves");
        assert_eq!(prop(&props, "persistent"), "false");
        assert_eq!(prop(&props, "distance"), "0");
        let (name, props) = state(18, 0b1000);
        assert_eq!(name, "minecraft:oak_leaves");
        assert_eq!(prop(&props, "distance"), "7");
        let (name, props) = state(18, 0b0100);
        assert_eq!(name, "minecraft:oak_leaves");
        assert_eq!(prop(&props, "persistent"), "true");
        // Every engine distance saves as the nearest 1.8 leaf state.
        let mut far = block_state(18, 0b1000);
        far.properties.insert("distance".into(), "3".into());
        let (id, meta) = legacy_block_state(&far).expect("leaf distance 3 saves");
        assert_eq!((id, meta & 0b1100), (18, 0b1000));
    }

    #[test]
    fn current_only_names_map_to_the_nearest_1_8_block() {
        assert_eq!(legacy_alias("minecraft:deepslate"), Some("minecraft:stone"));
        assert_eq!(
            legacy_alias("minecraft:deepslate_iron_ore"),
            Some("minecraft:iron_ore")
        );
        assert_eq!(
            legacy_alias("minecraft:nether_wart_block"),
            Some("minecraft:netherrack")
        );
        assert_eq!(
            legacy_alias("minecraft:stripped_spruce_wood"),
            Some("minecraft:spruce_bark")
        );
        assert_eq!(
            legacy_alias("minecraft:acacia_wood"),
            Some("minecraft:acacia_bark")
        );
        assert_eq!(
            legacy_alias("minecraft:melon"),
            Some("minecraft:melon_block")
        );
        assert_eq!(
            legacy_alias("minecraft:pumpkin"),
            Some("minecraft:carved_pumpkin")
        );
        assert_eq!(legacy_alias("minecraft:sculk_vein"), Some("minecraft:air"));
        assert_eq!(legacy_alias("minecraft:stone"), None);
    }

    #[test]
    fn fluids_keep_their_level() {
        let (name, props) = state(9, 0);
        assert_eq!(name, "minecraft:water");
        assert_eq!(prop(&props, "level"), "0");
        let (name, props) = state(8, 3);
        assert_eq!(name, "minecraft:water");
        assert_eq!(prop(&props, "level"), "3");
        let (name, props) = state(11, 0);
        assert_eq!(name, "minecraft:lava");
        assert_eq!(prop(&props, "level"), "0");
    }

    #[test]
    fn unknown_ids_are_air() {
        assert_eq!(state(198, 0).0, "minecraft:air");
        assert_eq!(state(255, 0).0, "minecraft:air");
        assert_eq!(state(0x0FFF, 15).0, "minecraft:air");
    }

    #[test]
    fn every_1_8_state_round_trips_through_its_canonical_id() {
        for id in 0..=MAX_BLOCK_ID {
            for meta in 0..16 {
                let state = block_state(id, meta);
                let (back_id, back_meta) =
                    legacy_block_state(&state).unwrap_or_else(|| panic!("{id}:{meta}"));
                assert_eq!(
                    block_state(back_id, back_meta),
                    state,
                    "{id}:{meta} came back as {back_id}:{back_meta}"
                );
            }
        }
    }

    #[test]
    fn source_fluids_prefer_the_still_id() {
        assert_eq!(legacy_block_state(&block_state(8, 0)), Some((9, 0)));
        assert_eq!(legacy_block_state(&block_state(10, 0)), Some((11, 0)));
        // Flowing levels are not level 0 and keep the flowing id.
        assert_eq!(legacy_block_state(&block_state(8, 3)), Some((8, 3)));
    }

    #[test]
    fn a_chest_uses_a_facing_metadata_that_1_8_registers() {
        // 1.8 registers chest states at metadata 2..=5 only. The datafixer
        // defaults 54:0 to the same north-facing state, but a chunk written
        // with 54:0 reads as air in a 1.8 game, so the writer must use 54:2.
        let north = block_state(54, 2);
        assert_eq!(north.name, "minecraft:chest");
        assert_eq!(legacy_block_state(&north), Some((54, 2)));
        assert_eq!(legacy_block_state(&block_state(54, 0)), Some((54, 2)));
        // Furnaces likewise start at metadata 2.
        assert_eq!(legacy_block_state(&block_state(61, 0)), Some((61, 2)));
        assert!(is_registered(54, 2));
        assert!(!is_registered(54, 0));
    }

    #[test]
    fn uncarved_pumpkin_maps_back_to_the_1_8_pumpkin() {
        let carved = block_state(86, 1);
        let mut pumpkin = carved.clone();
        pumpkin.name = "minecraft:pumpkin".to_owned();
        assert_eq!(legacy_block_state(&pumpkin), Some((86, 1)));
        assert_eq!(legacy_block_state(&carved), Some((86, 1)));
    }

    #[test]
    fn states_that_did_not_exist_in_1_8_have_no_id() {
        let state = BlockState {
            name: "minecraft:sculk".to_owned(),
            properties: Default::default(),
        };
        assert_eq!(legacy_block_state(&state), None);
    }
}
