//! Minecraft 1.8.9's per-biome natural spawn lists.
//!
//! Generated from `tools/BiomeSpawnProbe18.java`, which reads
//! `Biome.getSpawnEntries` for every registered `BiomeGenBase` id, plus the
//! swamp hut's and ocean monument's structure-specific monster lists that
//! `SurfaceChunkGenerator.getSpawnEntries` substitutes. Class names are
//! mapped to the entity ids this engine uses.
//!
//! Each biome's four lists are `EntityCategory.values()` order:
//! [`Category::Monster`], [`Category::Creature`], [`Category::Ambient`],
//! [`Category::Water`]. Mutated biomes share their base family's lists.

/// One 1.8 `Biome.SpawnEntry`: the entity, its weight and the pack size
/// (`minGroupSize`/`maxGroupSize`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnEntry18 {
    pub entity: &'static str,
    pub weight: i32,
    pub min: i32,
    pub max: i32,
}

/// The four 1.8 `EntityCategory` values, in `values()` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Monster,
    Creature,
    Ambient,
    Water,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::Monster,
        Category::Creature,
        Category::Ambient,
        Category::Water,
    ];

    pub const fn index(self) -> usize {
        match self {
            Category::Monster => 0,
            Category::Creature => 1,
            Category::Ambient => 2,
            Category::Water => 3,
        }
    }

    /// `EntityCategory.getSpawnCap`.
    pub const fn cap(self) -> i32 {
        match self {
            Category::Monster => 70,
            Category::Creature => 10,
            Category::Ambient => 15,
            Category::Water => 5,
        }
    }

    /// `EntityCategory.isHostile`.
    pub const fn hostile(self) -> bool {
        matches!(self, Category::Monster)
    }

    /// `EntityCategory.isBreedable`.
    pub const fn breedable(self) -> bool {
        matches!(self, Category::Creature)
    }
}

/// The four spawn lists of one 1.8 biome, in [`Category`] order.
pub struct BiomeSpawns18 {
    pub id: u8,
    pub lists: [&'static [SpawnEntry18]; 4],
}

const MONSTER_0: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:spider",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:zombie",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:skeleton",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:creeper",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:slime",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:enderman",
        weight: 10,
        min: 1,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:witch",
        weight: 5,
        min: 1,
        max: 1,
    },
];

const CREATURE_1: &[SpawnEntry18] = &[];

const AMBIENT_2: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:bat",
    weight: 10,
    min: 8,
    max: 8,
}];

const WATER_3: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:squid",
    weight: 10,
    min: 4,
    max: 4,
}];

const CREATURE_4: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:horse",
        weight: 5,
        min: 2,
        max: 6,
    },
];

const CREATURE_5: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
];

const CREATURE_6: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:wolf",
        weight: 5,
        min: 4,
        max: 4,
    },
];

const CREATURE_7: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:wolf",
        weight: 8,
        min: 4,
        max: 4,
    },
];

const MONSTER_8: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:spider",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:zombie",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:skeleton",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:creeper",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:slime",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:enderman",
        weight: 10,
        min: 1,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:witch",
        weight: 5,
        min: 1,
        max: 1,
    },
    SpawnEntry18 {
        entity: "minecraft:slime",
        weight: 1,
        min: 1,
        max: 1,
    },
];

const MONSTER_9: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:ghast",
        weight: 50,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:zombified_piglin",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:magma_cube",
        weight: 1,
        min: 4,
        max: 4,
    },
];

const AMBIENT_10: &[SpawnEntry18] = &[];

const WATER_11: &[SpawnEntry18] = &[];

const MONSTER_12: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:enderman",
    weight: 10,
    min: 4,
    max: 4,
}];

const MONSTER_13: &[SpawnEntry18] = &[];

const CREATURE_14: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:mooshroom",
    weight: 8,
    min: 4,
    max: 8,
}];

const MONSTER_15: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:spider",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:zombie",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:skeleton",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:creeper",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:slime",
        weight: 100,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:enderman",
        weight: 10,
        min: 1,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:witch",
        weight: 5,
        min: 1,
        max: 1,
    },
    SpawnEntry18 {
        entity: "minecraft:ocelot",
        weight: 2,
        min: 1,
        max: 1,
    },
];

const CREATURE_16: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
];

const CREATURE_17: &[SpawnEntry18] = &[
    SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 12,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:rabbit",
        weight: 10,
        min: 3,
        max: 3,
    },
    SpawnEntry18 {
        entity: "minecraft:pig",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:chicken",
        weight: 10,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:cow",
        weight: 8,
        min: 4,
        max: 4,
    },
    SpawnEntry18 {
        entity: "minecraft:horse",
        weight: 1,
        min: 2,
        max: 6,
    },
];

/// `TempleStructure`'s swamp hut monster list.
pub const SWAMP_HUT_MONSTERS: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:witch",
    weight: 1,
    min: 1,
    max: 1,
}];

/// `OceanMonumentStructure.getSpawnableMobs`.
pub const OCEAN_MONUMENT_MONSTERS: &[SpawnEntry18] = &[SpawnEntry18 {
    entity: "minecraft:guardian",
    weight: 1,
    min: 2,
    max: 4,
}];

/// Every 1.8.9 biome id whose spawn lists the probe could read.
pub static BIOMES: &[BiomeSpawns18] = &[
    BiomeSpawns18 {
        id: 0,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 1,
        lists: [MONSTER_0, CREATURE_4, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 2,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 3,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 4,
        lists: [MONSTER_0, CREATURE_6, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 5,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 6,
        lists: [MONSTER_8, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 7,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 8,
        lists: [MONSTER_9, CREATURE_1, AMBIENT_10, WATER_11],
    },
    BiomeSpawns18 {
        id: 9,
        lists: [MONSTER_12, CREATURE_1, AMBIENT_10, WATER_11],
    },
    BiomeSpawns18 {
        id: 10,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 11,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 12,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 13,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 14,
        lists: [MONSTER_13, CREATURE_14, AMBIENT_2, WATER_11],
    },
    BiomeSpawns18 {
        id: 15,
        lists: [MONSTER_13, CREATURE_14, AMBIENT_2, WATER_11],
    },
    BiomeSpawns18 {
        id: 16,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 17,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 18,
        lists: [MONSTER_0, CREATURE_6, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 19,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 20,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 21,
        lists: [MONSTER_15, CREATURE_16, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 22,
        lists: [MONSTER_15, CREATURE_16, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 23,
        lists: [MONSTER_0, CREATURE_16, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 24,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 25,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 26,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 27,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 28,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 29,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 30,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 31,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 32,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 33,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 34,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 35,
        lists: [MONSTER_0, CREATURE_17, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 36,
        lists: [MONSTER_0, CREATURE_17, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 37,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 38,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 39,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 129,
        lists: [MONSTER_0, CREATURE_4, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 130,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 131,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 132,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 133,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 134,
        lists: [MONSTER_8, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 140,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 149,
        lists: [MONSTER_15, CREATURE_16, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 151,
        lists: [MONSTER_0, CREATURE_16, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 155,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 156,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 157,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 158,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 160,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 161,
        lists: [MONSTER_0, CREATURE_7, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 162,
        lists: [MONSTER_0, CREATURE_5, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 163,
        lists: [MONSTER_0, CREATURE_17, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 164,
        lists: [MONSTER_0, CREATURE_17, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 165,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 166,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
    BiomeSpawns18 {
        id: 167,
        lists: [MONSTER_0, CREATURE_1, AMBIENT_2, WATER_3],
    },
];

/// The lists of a 1.8 biome id, if it was registered.
pub fn for_biome(id: u8) -> Option<&'static BiomeSpawns18> {
    BIOMES
        .binary_search_by_key(&id, |b| b.id)
        .ok()
        .map(|i| &BIOMES[i])
}

/// The lists of a 26.3 biome name, through [`super::biome_1_8_9`].
pub fn for_name(name: &str) -> Option<&'static BiomeSpawns18> {
    super::biome_1_8_9::legacy_biome_id(name).and_then(for_biome)
}
