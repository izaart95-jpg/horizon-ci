//! Minecraft 1.8.9's per-biome terrain parameters.
//!
//! `SurfaceChunkGenerator.method_6544` blends every biome's `depth` and
//! `variationModifier` over the 5x5 biome neighbourhood to pick the base
//! height and height variation of a density column, and `method_6546` hands
//! each column to the biome's own `genTerrainBlocks`, which starts from the
//! biome's `topBlock`/`baseBlock`. 1.8.9 keeps both sets of values on the
//! `Biome` instance, not in a data pack, so this module is the table the
//! 1.8 generator reads: `(id, depth, variation, temperature, rainfall, top,
//! filler, surface override)`.
//!
//! The surface override is which `method_6420` implementation a biome has:
//! most biomes use `Biome`'s plain sand/gravel shoreline pass, while oceans,
//! extreme hills, mesas, swamps, taigas, shattered savanna and the mutated
//! variants replace the surface (or the whole column) themselves.
//!
//! The values are `BiomeTerrain18`'s `depth`/`variationModifier` as
//! `Float.floatToIntBits`, and the blocks as the 1.8 block id and metadata
//! `Biome.topBlock`/`baseBlock` carry, so
//! [`crate::legacy_1_8_9::block_state`] turns them into a current state.
//! `tests/fixtures/biome_params_1_8_9.txt` pins every field against the
//! 1.8.9 jar.

/// A 1.8 block as `(numeric id, metadata)`, the pair
/// [`crate::legacy_1_8_9::block_state`] resolves.
pub type LegacyBlock18 = (u16, u8);

/// Which `method_6420` body a biome runs over its column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface18 {
    /// `Biome.method_6420`: stone, then the filler and top blocks, with the
    /// sand/gravel shoreline pass over beaches and oceans.
    Plain,
    /// `OceanBiome`: plain, except the top block is always gravel where the
    /// depth is shallow enough to see the floor.
    Ocean,
    /// `ExtremeHillsBiome`: grass gives way to stone above y = 95, and the
    /// gravel/stone-noise band is wider.
    ExtremeHills,
    /// `MesaBiome`: hardened clay bands and no shoreline pass.
    Mesa,
    /// `MutatedBiome`: delegates to the base biome, then adds the mutated
    /// variant's own height jitter.
    Mutated,
    /// `ShatteredSavannaBiome`: a jagged, mostly stone surface.
    ShatteredSavanna,
    /// `SwampBiome`: a water table just below sea level and clay pockets.
    Swamp,
    /// `TaigaBiome`: plain, except the top stays grass under the snow.
    Taiga,
}

/// The 1.8.9 terrain fields of one biome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeTerrain18 {
    /// `Biome.depth`: the base height, in the units `method_6544` blends.
    pub depth: f32,
    /// `Biome.variationModifier`: how far the height noise can swing.
    pub variation: f32,
    /// `Biome.temperature`, read by `method_6420` (snow, water freezing).
    pub temperature: f32,
    /// `Biome.getRainfall()`, read by `method_6420` (snow above 0.15).
    pub rainfall: f32,
    /// `Biome.topBlock` in 1.8 ids, the first block of the surface.
    pub top: LegacyBlock18,
    /// `Biome.baseBlock` in 1.8 ids, the filler below the top.
    pub filler: LegacyBlock18,
    /// Which `method_6420` body this biome runs.
    pub surface: Surface18,
}

const fn row(
    id: u8,
    depth: u32,
    variation: u32,
    temperature: u32,
    rainfall: u32,
    top: LegacyBlock18,
    filler: LegacyBlock18,
    surface: Surface18,
) -> (u8, BiomeTerrain18) {
    (
        id,
        BiomeTerrain18 {
            depth: f32::from_bits(depth),
            variation: f32::from_bits(variation),
            temperature: f32::from_bits(temperature),
            rainfall: f32::from_bits(rainfall),
            top,
            filler,
            surface,
        },
    )
}

use Surface18::{ExtremeHills, Mesa, Mutated, Ocean, Plain, ShatteredSavanna, Swamp, Taiga};

/// `(BiomeGenBase id, terrain)`, ascending by id: the 40 base biomes and the
/// 21 mutated variants (`base + 128`) that 1.8.9 registers.
const TERRAIN: &[(u8, BiomeTerrain18)] = &[
    row(
        0,
        0xbf800000,
        0x3dcccccd,
        0x3f000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Ocean,
    ), // ocean
    row(
        1,
        0x3e000000,
        0x3d4ccccd,
        0x3f4ccccd,
        0x3ecccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // plains
    row(
        2,
        0x3e000000,
        0x3d4ccccd,
        0x40000000,
        0x00000000,
        (12, 0),
        (12, 0),
        Plain,
    ), // desert
    row(
        3,
        0x3f800000,
        0x3f000000,
        0x3e4ccccd,
        0x3e99999a,
        (2, 0),
        (3, 0),
        ExtremeHills,
    ), // windswept_hills
    row(
        4,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f333333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // forest
    row(
        5,
        0x3e4ccccd,
        0x3e4ccccd,
        0x3e800000,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // taiga
    row(
        6,
        0xbe4ccccd,
        0x3dcccccd,
        0x3f4ccccd,
        0x3f666666,
        (2, 0),
        (3, 0),
        Swamp,
    ), // swamp
    row(
        7,
        0xbf000000,
        0x00000000,
        0x3f000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // river
    row(
        8,
        0x3dcccccd,
        0x3e4ccccd,
        0x40000000,
        0x00000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // nether_wastes
    row(
        9,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f000000,
        0x3f000000,
        (3, 0),
        (3, 0),
        Plain,
    ), // the_end
    row(
        10,
        0xbf800000,
        0x3dcccccd,
        0x00000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Ocean,
    ), // frozen_ocean
    row(
        11,
        0xbf000000,
        0x00000000,
        0x00000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // frozen_river
    row(
        12,
        0x3e000000,
        0x3d4ccccd,
        0x00000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // snowy_plains
    row(
        13,
        0x3ee66666,
        0x3e99999a,
        0x00000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // snowy_plains
    row(
        14,
        0x3e4ccccd,
        0x3e99999a,
        0x3f666666,
        0x3f800000,
        (110, 0),
        (3, 0),
        Plain,
    ), // mushroom_fields
    row(
        15,
        0x00000000,
        0x3ccccccd,
        0x3f666666,
        0x3f800000,
        (110, 0),
        (3, 0),
        Plain,
    ), // mushroom_fields
    row(
        16,
        0x00000000,
        0x3ccccccd,
        0x3f4ccccd,
        0x3ecccccd,
        (12, 0),
        (12, 0),
        Plain,
    ), // beach
    row(
        17,
        0x3ee66666,
        0x3e99999a,
        0x40000000,
        0x00000000,
        (12, 0),
        (12, 0),
        Plain,
    ), // desert
    row(
        18,
        0x3ee66666,
        0x3e99999a,
        0x3f333333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // forest
    row(
        19,
        0x3ee66666,
        0x3e99999a,
        0x3e800000,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // taiga
    row(
        20,
        0x3f4ccccd,
        0x3e99999a,
        0x3e4ccccd,
        0x3e99999a,
        (2, 0),
        (3, 0),
        ExtremeHills,
    ), // windswept_hills
    row(
        21,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f733333,
        0x3f666666,
        (2, 0),
        (3, 0),
        Plain,
    ), // jungle
    row(
        22,
        0x3ee66666,
        0x3e99999a,
        0x3f733333,
        0x3f666666,
        (2, 0),
        (3, 0),
        Plain,
    ), // jungle
    row(
        23,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f733333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // sparse_jungle
    row(
        24,
        0xbfe66666,
        0x3dcccccd,
        0x3f000000,
        0x3f000000,
        (2, 0),
        (3, 0),
        Ocean,
    ), // deep_ocean
    row(
        25,
        0x3dcccccd,
        0x3f4ccccd,
        0x3e4ccccd,
        0x3e99999a,
        (1, 0),
        (1, 0),
        Plain,
    ), // stony_shore
    row(
        26,
        0x00000000,
        0x3ccccccd,
        0x3d4ccccd,
        0x3e99999a,
        (12, 0),
        (12, 0),
        Plain,
    ), // snowy_beach
    row(
        27,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f19999a,
        0x3f19999a,
        (2, 0),
        (3, 0),
        Plain,
    ), // birch_forest
    row(
        28,
        0x3ee66666,
        0x3e99999a,
        0x3f19999a,
        0x3f19999a,
        (2, 0),
        (3, 0),
        Plain,
    ), // birch_forest
    row(
        29,
        0x3dcccccd,
        0x3e4ccccd,
        0x3f333333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // dark_forest
    row(
        30,
        0x3e4ccccd,
        0x3e4ccccd,
        0xbf000000,
        0x3ecccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // snowy_taiga
    row(
        31,
        0x3ee66666,
        0x3e99999a,
        0xbf000000,
        0x3ecccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // snowy_taiga
    row(
        32,
        0x3e4ccccd,
        0x3e4ccccd,
        0x3e99999a,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // old_growth_pine_taiga
    row(
        33,
        0x3ee66666,
        0x3e99999a,
        0x3e99999a,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // old_growth_pine_taiga
    row(
        34,
        0x3f800000,
        0x3f000000,
        0x3e4ccccd,
        0x3e99999a,
        (2, 0),
        (3, 0),
        ExtremeHills,
    ), // windswept_forest
    row(
        35,
        0x3e000000,
        0x3d4ccccd,
        0x3f99999a,
        0x00000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // savanna
    row(
        36,
        0x3fc00000,
        0x3ccccccd,
        0x3f800000,
        0x00000000,
        (2, 0),
        (3, 0),
        Plain,
    ), // savanna_plateau
    row(
        37,
        0x3dcccccd,
        0x3e4ccccd,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // badlands
    row(
        38,
        0x3fc00000,
        0x3ccccccd,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // wooded_badlands
    row(
        39,
        0x3fc00000,
        0x3ccccccd,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // badlands
    row(
        129,
        0x3e000000,
        0x3d4ccccd,
        0x3f4ccccd,
        0x3ecccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // sunflower_plains
    row(
        130,
        0x3e666666,
        0x3e800000,
        0x40000000,
        0x00000000,
        (12, 0),
        (12, 0),
        Mutated,
    ), // desert
    row(
        131,
        0x3f800000,
        0x3f000000,
        0x3e4ccccd,
        0x3e99999a,
        (2, 0),
        (3, 0),
        ExtremeHills,
    ), // windswept_gravelly_hills
    row(
        132,
        0x3dcccccd,
        0x3ecccccd,
        0x3f333333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Plain,
    ), // flower_forest
    row(
        133,
        0x3e99999a,
        0x3ecccccd,
        0x3e800000,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Mutated,
    ), // taiga
    row(
        134,
        0xbdcccccd,
        0x3e99999a,
        0x3f4ccccd,
        0x3f666666,
        (2, 0),
        (3, 0),
        Mutated,
    ), // swamp
    row(
        140,
        0x3ed9999a,
        0x3ee66667,
        0x00000000,
        0x3f000000,
        (80, 0),
        (3, 0),
        Plain,
    ), // ice_spikes
    row(
        149,
        0x3e4ccccd,
        0x3ecccccd,
        0x3f733333,
        0x3f666666,
        (2, 0),
        (3, 0),
        Mutated,
    ), // jungle
    row(
        151,
        0x3e4ccccd,
        0x3ecccccd,
        0x3f733333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Mutated,
    ), // sparse_jungle
    row(
        155,
        0x3e4ccccd,
        0x3ecccccd,
        0x3f19999a,
        0x3f19999a,
        (2, 0),
        (3, 0),
        Mutated,
    ), // old_growth_birch_forest
    row(
        156,
        0x3f0ccccd,
        0x3f000000,
        0x3f19999a,
        0x3f19999a,
        (2, 0),
        (3, 0),
        Mutated,
    ), // old_growth_birch_forest
    row(
        157,
        0x3e4ccccd,
        0x3ecccccd,
        0x3f333333,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Mutated,
    ), // dark_forest
    row(
        158,
        0x3e99999a,
        0x3ecccccd,
        0xbf000000,
        0x3ecccccd,
        (2, 0),
        (3, 0),
        Mutated,
    ), // snowy_taiga
    row(
        160,
        0x3e4ccccd,
        0x3e4ccccd,
        0x3e800000,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // old_growth_spruce_taiga
    row(
        161,
        0x3e4ccccd,
        0x3e4ccccd,
        0x3e800000,
        0x3f4ccccd,
        (2, 0),
        (3, 0),
        Taiga,
    ), // old_growth_spruce_taiga
    row(
        162,
        0x3f800000,
        0x3f000000,
        0x3e4ccccd,
        0x3e99999a,
        (2, 0),
        (3, 0),
        ExtremeHills,
    ), // windswept_gravelly_hills
    row(
        163,
        0x3eb9999a,
        0x3f9ccccd,
        0x3f8ccccd,
        0x00000000,
        (2, 0),
        (3, 0),
        ShatteredSavanna,
    ), // windswept_savanna
    row(
        164,
        0x3f866666,
        0x3f9b3334,
        0x3f800000,
        0x00000000,
        (2, 0),
        (3, 0),
        ShatteredSavanna,
    ), // windswept_savanna
    row(
        165,
        0x3dcccccd,
        0x3e4ccccd,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // eroded_badlands
    row(
        166,
        0x3ee66666,
        0x3e99999a,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // wooded_badlands
    row(
        167,
        0x3ee66666,
        0x3e99999a,
        0x40000000,
        0x00000000,
        (12, 1),
        (159, 0),
        Mesa,
    ), // badlands
];

/// The 1.8.9 terrain fields of biome `id`, or `None` for an id 1.8.9 never
/// registered.
pub fn terrain(id: u8) -> Option<BiomeTerrain18> {
    TERRAIN
        .binary_search_by_key(&id, |(biome, _)| *biome)
        .ok()
        .map(|index| TERRAIN[index].1)
}

/// The plains row, the fallback for an id outside 1.8.9's table.
pub const PLAINS: BiomeTerrain18 = BiomeTerrain18 {
    depth: f32::from_bits(0x3e000000),
    variation: f32::from_bits(0x3d4ccccd),
    temperature: f32::from_bits(0x3f4ccccd),
    rainfall: f32::from_bits(0x3ecccccd),
    top: (2, 0),
    filler: (3, 0),
    surface: Surface18::Plain,
};

#[cfg(test)]
mod tests {
    use super::*;

    struct Row {
        id: u8,
        depth: u32,
        variation: u32,
        temperature: u32,
        rainfall: u32,
        top: LegacyBlock18,
        filler: LegacyBlock18,
        surface: Surface18,
    }

    fn oracle() -> Vec<Row> {
        let mut rows = Vec::new();
        for line in include_str!("../tests/fixtures/biome_params_1_8_9.txt").lines() {
            if !line.starts_with("biome id=") {
                continue;
            }
            let mut fields = std::collections::BTreeMap::new();
            for field in line["biome ".len()..].split(';') {
                let (key, value) = field.split_once('=').expect("key=value");
                fields.insert(key, value);
            }
            if fields["id"] == "-1" {
                continue;
            }
            rows.push(Row {
                id: fields["id"].parse().expect("id"),
                depth: fields["depth"].parse::<i64>().expect("depth") as u32,
                variation: fields["variation"].parse::<i64>().expect("variation") as u32,
                temperature: fields["temperature"].parse::<i64>().expect("temperature") as u32,
                rainfall: fields["rainfall"].parse::<i64>().expect("rainfall") as u32,
                top: (
                    fields["top_id"].parse().expect("top id"),
                    fields["top_meta"].parse().expect("top meta"),
                ),
                filler: (
                    fields["filler_id"].parse().expect("filler id"),
                    fields["filler_meta"].parse().expect("filler meta"),
                ),
                surface: match fields["surface"] {
                    "Biome" => Surface18::Plain,
                    "OceanBiome" => Surface18::Ocean,
                    "ExtremeHillsBiome" => Surface18::ExtremeHills,
                    "MesaBiome" => Surface18::Mesa,
                    "MutatedBiome" => Surface18::Mutated,
                    "ShatteredSavannaBiome" => Surface18::ShatteredSavanna,
                    "SwampBiome" => Surface18::Swamp,
                    "TaigaBiome" => Surface18::Taiga,
                    other => panic!("unexpected surface override {other}"),
                },
            });
        }
        rows
    }

    /// Every registered biome's terrain fields against the 1.8.9 jar, and no
    /// extra rows: the table is data the generator reads, so a typo in one
    /// value changes every world grown on that biome.
    #[test]
    fn biome_terrain_matches_the_one_eight_nine_jar() {
        let rows = oracle();
        assert_eq!(rows.len(), 61, "fixture biomes");
        assert_eq!(TERRAIN.len(), rows.len(), "table rows");
        for (row, (id, terrain)) in rows.iter().zip(TERRAIN) {
            assert_eq!(*id, row.id, "biome id order");
            assert_eq!(terrain.depth.to_bits(), row.depth, "{} depth", row.id);
            assert_eq!(
                terrain.variation.to_bits(),
                row.variation,
                "{} variation",
                row.id
            );
            assert_eq!(
                terrain.temperature.to_bits(),
                row.temperature,
                "{} temperature",
                row.id
            );
            assert_eq!(
                terrain.rainfall.to_bits(),
                row.rainfall,
                "{} rainfall",
                row.id
            );
            assert_eq!(terrain.top, row.top, "{} top block", row.id);
            assert_eq!(terrain.filler, row.filler, "{} filler block", row.id);
            assert_eq!(terrain.surface, row.surface, "{} surface override", row.id);
        }
        assert_eq!(
            terrain(1),
            Some(PLAINS),
            "the plains fallback is the plains row"
        );
        assert_eq!(terrain(40), None, "40..=128 had no 1.8.9 biome");
        assert_eq!(terrain(255), None, "168..=255 had no 1.8.9 biome");
    }
}
