//! Minecraft 1.8.9 numeric biome ids to 26.3 biome names.
//!
//! A 1.8.9 world stores one biome byte per column: [`crate::chunk_1_8_9`] keeps
//! that array in the chunk's `Biomes` tag and decodes the 256-byte bulk biome
//! payload of the network format into the same bytes. The value is
//! `BiomeGenBase.biomeID`. This module names those ids.
//!
//! # The 1.8.9 ids
//!
//! `BiomeGenBase` registers 0..=39, plus a mutated variant at `id + 128` for
//! every base biome that has one (129..=167). `GenLayerHills` puts a mutated
//! id in a chunk as a rare variant of its base biome. Nothing else is
//! registered: 40..=128 and 168..=255 had no 1.8.9 biome, so [`biome_name`]
//! returns `None` there instead of substituting `minecraft:plains`. The
//! datafixer's `biome_by_id` falls back to plains because it is also fed the
//! *later* legacy ids (127 `the_void`, 168/169 bamboo jungle, the 1.13 end and
//! ocean ids, 1.16+ biomes), which are not 1.8.9 ids and are left to the
//! datafixer here.
//!
//! The names are the 26.3 ones, not the intermediate names the datafixer
//! carries midway: the forward table is `ChunkHeightAndBiomeFix.BIOMES_BY_ID`
//! restricted to the 1.8 ids and run through vanilla's `CavesAndCliffsRenames`,
//! so 3 `mountains` becomes `windswept_hills`, 12 `snowy_tundra` becomes
//! `snowy_plains`, 32 `giant_tree_taiga` becomes `old_growth_pine_taiga`, 38
//! `wooded_badlands_plateau` becomes `wooded_badlands`, and so on.
//!
//! Several ids share one 26.3 name: the hills variants were folded into their
//! base biome, as were the mutated variants that `CavesAndCliffsRenames`
//! collapses. [`legacy_biome_id`] returns the lowest id with the name. Names a
//! biome had before 26.3 are accepted as aliases and keep the id they denoted
//! at the time, so old level and chunk data round trips.

/// `(BiomeGenBase id, 26.3 name)`, ascending by id.
///
/// 0..=39 are the base biomes and 129..=167 the mutated variants a 1.8.9 chunk
/// can contain (`base + 128`).
const BIOMES: &[(u8, &str)] = &[
    (0, "minecraft:ocean"),
    (1, "minecraft:plains"),
    (2, "minecraft:desert"),
    (3, "minecraft:windswept_hills"),
    (4, "minecraft:forest"),
    (5, "minecraft:taiga"),
    (6, "minecraft:swamp"),
    (7, "minecraft:river"),
    (8, "minecraft:nether_wastes"),
    (9, "minecraft:the_end"),
    (10, "minecraft:frozen_ocean"),
    (11, "minecraft:frozen_river"),
    (12, "minecraft:snowy_plains"),
    (13, "minecraft:snowy_plains"),
    (14, "minecraft:mushroom_fields"),
    (15, "minecraft:mushroom_fields"),
    (16, "minecraft:beach"),
    (17, "minecraft:desert"),
    (18, "minecraft:forest"),
    (19, "minecraft:taiga"),
    (20, "minecraft:windswept_hills"),
    (21, "minecraft:jungle"),
    (22, "minecraft:jungle"),
    (23, "minecraft:sparse_jungle"),
    (24, "minecraft:deep_ocean"),
    (25, "minecraft:stony_shore"),
    (26, "minecraft:snowy_beach"),
    (27, "minecraft:birch_forest"),
    (28, "minecraft:birch_forest"),
    (29, "minecraft:dark_forest"),
    (30, "minecraft:snowy_taiga"),
    (31, "minecraft:snowy_taiga"),
    (32, "minecraft:old_growth_pine_taiga"),
    (33, "minecraft:old_growth_pine_taiga"),
    (34, "minecraft:windswept_forest"),
    (35, "minecraft:savanna"),
    (36, "minecraft:savanna_plateau"),
    (37, "minecraft:badlands"),
    (38, "minecraft:wooded_badlands"),
    (39, "minecraft:badlands"),
    (129, "minecraft:sunflower_plains"),
    (130, "minecraft:desert"),
    (131, "minecraft:windswept_gravelly_hills"),
    (132, "minecraft:flower_forest"),
    (133, "minecraft:taiga"),
    (134, "minecraft:swamp"),
    (140, "minecraft:ice_spikes"),
    (149, "minecraft:jungle"),
    (151, "minecraft:sparse_jungle"),
    (155, "minecraft:old_growth_birch_forest"),
    (156, "minecraft:old_growth_birch_forest"),
    (157, "minecraft:dark_forest"),
    (158, "minecraft:snowy_taiga"),
    (160, "minecraft:old_growth_spruce_taiga"),
    (161, "minecraft:old_growth_spruce_taiga"),
    (162, "minecraft:windswept_gravelly_hills"),
    (163, "minecraft:windswept_savanna"),
    (164, "minecraft:windswept_savanna"),
    (165, "minecraft:eroded_badlands"),
    (166, "minecraft:wooded_badlands"),
    (167, "minecraft:badlands"),
];

/// Names these biomes had before 26.3, with the 1.8.9 id the name denoted.
///
/// An alias keeps its original id rather than collapsing to the lowest id of
/// its eventual 26.3 name: 39 was `badlands_plateau` in 1.13, not `badlands`
/// (37), and 167 was `modified_badlands_plateau`.
const OLD_NAMES: &[(&str, u8)] = &[
    // 1.9-1.12 registry names, which `BiomeFix.BIOMES` rewrites.
    ("minecraft:beaches", 16),
    ("minecraft:cold_beach", 26),
    ("minecraft:extreme_hills", 3),
    ("minecraft:extreme_hills_with_trees", 34),
    ("minecraft:forest_hills", 18),
    ("minecraft:hell", 8),
    ("minecraft:ice_flats", 12),
    ("minecraft:ice_mountains", 13),
    ("minecraft:mesa", 37),
    ("minecraft:mesa_clear_rock", 39),
    ("minecraft:mesa_rock", 38),
    ("minecraft:mushroom_island", 14),
    ("minecraft:mushroom_island_shore", 15),
    ("minecraft:mutated_birch_forest", 155),
    ("minecraft:mutated_birch_forest_hills", 156),
    ("minecraft:mutated_desert", 130),
    ("minecraft:mutated_extreme_hills", 131),
    ("minecraft:mutated_extreme_hills_with_trees", 162),
    ("minecraft:mutated_forest", 132),
    ("minecraft:mutated_ice_flats", 140),
    ("minecraft:mutated_jungle", 149),
    ("minecraft:mutated_jungle_edge", 151),
    ("minecraft:mutated_mesa", 165),
    ("minecraft:mutated_mesa_clear_rock", 167),
    ("minecraft:mutated_mesa_rock", 166),
    ("minecraft:mutated_plains", 129),
    ("minecraft:mutated_redwood_taiga", 160),
    ("minecraft:mutated_redwood_taiga_hills", 161),
    ("minecraft:mutated_roofed_forest", 157),
    ("minecraft:mutated_savanna", 163),
    ("minecraft:mutated_savanna_rock", 164),
    ("minecraft:mutated_swampland", 134),
    ("minecraft:mutated_taiga", 133),
    ("minecraft:mutated_taiga_cold", 158),
    ("minecraft:redwood_taiga", 32),
    ("minecraft:redwood_taiga_hills", 33),
    ("minecraft:roofed_forest", 29),
    ("minecraft:savanna_rock", 36),
    ("minecraft:sky", 9),
    ("minecraft:smaller_extreme_hills", 20),
    ("minecraft:stone_beach", 25),
    ("minecraft:swampland", 6),
    ("minecraft:taiga_cold", 30),
    ("minecraft:taiga_cold_hills", 31),
    // 1.13-1.18 names, which `CavesAndCliffsRenames` rewrites.
    ("minecraft:badlands_plateau", 39),
    ("minecraft:birch_forest_hills", 28),
    ("minecraft:dark_forest_hills", 157),
    ("minecraft:desert_hills", 17),
    ("minecraft:desert_lakes", 130),
    ("minecraft:giant_spruce_taiga", 160),
    ("minecraft:giant_spruce_taiga_hills", 161),
    ("minecraft:giant_tree_taiga", 32),
    ("minecraft:giant_tree_taiga_hills", 33),
    ("minecraft:gravelly_mountains", 131),
    ("minecraft:jungle_edge", 23),
    ("minecraft:jungle_hills", 22),
    ("minecraft:modified_badlands_plateau", 167),
    ("minecraft:modified_gravelly_mountains", 162),
    ("minecraft:modified_jungle", 149),
    ("minecraft:modified_jungle_edge", 151),
    ("minecraft:modified_wooded_badlands_plateau", 166),
    ("minecraft:mountain_edge", 20),
    ("minecraft:mountains", 3),
    ("minecraft:mushroom_field_shore", 15),
    ("minecraft:shattered_savanna", 163),
    ("minecraft:shattered_savanna_plateau", 164),
    ("minecraft:snowy_mountains", 13),
    ("minecraft:snowy_taiga_hills", 31),
    ("minecraft:snowy_taiga_mountains", 158),
    ("minecraft:snowy_tundra", 12),
    ("minecraft:stone_shore", 25),
    ("minecraft:swamp_hills", 134),
    ("minecraft:taiga_hills", 19),
    ("minecraft:taiga_mountains", 133),
    ("minecraft:tall_birch_forest", 155),
    ("minecraft:tall_birch_hills", 156),
    ("minecraft:wooded_badlands_plateau", 38),
    ("minecraft:wooded_hills", 18),
    ("minecraft:wooded_mountains", 34),
];

/// The 1.8.9 id `BiomeGenBase` registered, or `None` when it registered none.
pub fn biome_name(id: u8) -> Option<&'static str> {
    BIOMES
        .binary_search_by_key(&id, |&(id, _)| id)
        .ok()
        .map(|i| BIOMES[i].1)
}

/// The 1.8.9 id a 26.3 biome name, or a name it had before 26.3, denoted.
///
/// Current names that several ids share return the lowest id (for example
/// `minecraft:badlands` is 37, not 39). Old names keep the id they denoted
/// (`minecraft:badlands_plateau` is 39). The `minecraft:` namespace is
/// optional; the rest of the name is matched exactly.
pub fn legacy_biome_id(name: &str) -> Option<u8> {
    let bare = without_namespace(name);
    if let Some(&(id, _)) = BIOMES.iter().find(|&&(_, n)| without_namespace(n) == bare) {
        return Some(id);
    }
    OLD_NAMES
        .iter()
        .find(|&&(old, _)| without_namespace(old) == bare)
        .map(|&(_, id)| id)
}

fn without_namespace(name: &str) -> &str {
    name.strip_prefix("minecraft:").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_1_8_9_biomes() {
        assert_eq!(biome_name(0), Some("minecraft:ocean"));
        assert_eq!(biome_name(1), Some("minecraft:plains"));
        assert_eq!(biome_name(2), Some("minecraft:desert"));
        assert_eq!(biome_name(6), Some("minecraft:swamp"));
        assert_eq!(biome_name(8), Some("minecraft:nether_wastes"));
        assert_eq!(biome_name(9), Some("minecraft:the_end"));
        assert_eq!(biome_name(12), Some("minecraft:snowy_plains"));
        assert_eq!(biome_name(16), Some("minecraft:beach"));
        assert_eq!(biome_name(37), Some("minecraft:badlands"));
        assert_eq!(biome_name(38), Some("minecraft:wooded_badlands"));
    }

    #[test]
    fn applies_the_renames_after_1_8_9() {
        assert_eq!(biome_name(3), Some("minecraft:windswept_hills"));
        assert_eq!(biome_name(14), Some("minecraft:mushroom_fields"));
        assert_eq!(biome_name(25), Some("minecraft:stony_shore"));
        assert_eq!(biome_name(29), Some("minecraft:dark_forest"));
        assert_eq!(biome_name(32), Some("minecraft:old_growth_pine_taiga"));
        assert_eq!(biome_name(160), Some("minecraft:old_growth_spruce_taiga"));
        assert_eq!(biome_name(165), Some("minecraft:eroded_badlands"));
    }

    #[test]
    fn the_whole_byte_range_is_answered() {
        let mut mapped = Vec::new();
        for id in 0..=u8::MAX {
            if biome_name(id).is_some() {
                mapped.push(id);
            }
        }
        let mut expected: Vec<u8> = (0..=39).collect();
        expected.extend(129..=134);
        expected.extend([140, 149, 151]);
        expected.extend(155..=158);
        expected.extend(160..=167);
        assert_eq!(mapped, expected);
    }

    #[test]
    fn the_table_is_sorted() {
        assert!(BIOMES.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn the_inverse_round_trips_to_the_lowest_id() {
        for id in 0..=u8::MAX {
            let Some(name) = biome_name(id) else {
                continue;
            };
            let lowest = (0..=u8::MAX)
                .find(|&other| biome_name(other) == Some(name))
                .expect("the id the name came from");
            assert_eq!(legacy_biome_id(name), Some(lowest), "{name}");
        }
    }

    #[test]
    fn old_names_keep_their_id() {
        assert_eq!(legacy_biome_id("minecraft:mountains"), Some(3));
        assert_eq!(legacy_biome_id("minecraft:snowy_tundra"), Some(12));
        assert_eq!(legacy_biome_id("minecraft:stone_shore"), Some(25));
        assert_eq!(
            legacy_biome_id("minecraft:giant_tree_taiga_hills"),
            Some(33)
        );
        assert_eq!(legacy_biome_id("minecraft:badlands_plateau"), Some(39));
        assert_eq!(
            legacy_biome_id("minecraft:wooded_badlands_plateau"),
            Some(38)
        );
        assert_eq!(legacy_biome_id("minecraft:redwood_taiga"), Some(32));
        assert_eq!(
            legacy_biome_id("minecraft:mutated_mesa_clear_rock"),
            Some(167)
        );
        assert_eq!(legacy_biome_id("minecraft:swampland"), Some(6));
        assert_eq!(legacy_biome_id("minecraft:sky"), Some(9));
        assert_eq!(legacy_biome_id("minecraft:ice_flats"), Some(12));
    }

    #[test]
    fn aliases_stay_inside_the_table() {
        for &(old, id) in OLD_NAMES {
            assert!(biome_name(id).is_some(), "{old} -> {id}");
            assert!(
                !BIOMES.iter().any(|&(_, current)| current == old),
                "{old} shadows a current name"
            );
        }
    }

    #[test]
    fn current_names_win_and_the_namespace_is_optional() {
        assert_eq!(legacy_biome_id("minecraft:badlands"), Some(37));
        assert_eq!(legacy_biome_id("badlands"), Some(37));
        assert_eq!(legacy_biome_id("ocean"), Some(0));
        assert_eq!(legacy_biome_id("minecraft:snowy_plains"), Some(12));
        assert_eq!(legacy_biome_id("minecraft:the_void"), None);
        assert_eq!(legacy_biome_id("minecraft:not_a_biome"), None);
        assert_eq!(legacy_biome_id(""), None);
    }

    #[test]
    fn unregistered_ids_have_no_name() {
        for id in [40, 50, 127, 128, 135, 141, 150, 159, 168, 169, 255] {
            assert_eq!(biome_name(id), None, "{id}");
        }
    }
}
