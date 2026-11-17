//! Minecraft 1.8.9's biome layer chain (`GenLayer` in 1.8, `Layer.init`).
//!
//! A 1.8.9 chunk stores one biome id per column. [`crate::biome_1_8_9`] names
//! those ids; this module produces them, porting the layer chain `Layer.init`
//! wires for the default generator. `LayeredBiomeSource` (the 1.8.9
//! `WorldChunkManager`) keeps the same chain's outputs:
//!
//! * `layers[1]` (the `VoronoiZoom` over the river-mixed biomes) is what
//!   `method_3858` writes into the chunk's `Biomes` array, called at
//!   `(chunkX * 16, chunkZ * 16, 16, 16)`. [`BiomeLayers18::chunk_biomes`]
//!   returns that array.
//! * `layers[0]` (the river-mixed biomes without the Voronoi zoom) is the
//!   10x10 neighbourhood `SurfaceChunkGenerator` reads at
//!   `(chunkX * 4 - 2, chunkZ * 4 - 2, 10, 10)` for its depth/scale blend.
//!
//! Every layer has its own linear congruential seed (`Layer`'s `field_174`)
//! that is rebased from the world seed and the cell's layer-space coordinate
//! for every cell, so a `get_ints` call is independent of the calls before it.
//! The 1.8.9 `IntArrayCache` only recycles arrays; this port allocates fresh
//! ones.
//!
//! The layer chain is deterministic apart from those seeds. The one
//! comparison that is not a plain integer comparison is `Layer.compareBiomes`:
//! it uses the biome's Java class, and `MutatedBiome.asClass` delegates to the
//! biome it mutates, so a mutated variant compares as its original. The
//! extreme-hills edge rule additionally accepts two biomes with the same
//! temperature class, or either of them at `MEDIUM`. [`BIOMES`] carries the
//! class and temperature each 1.8.9 id registered.

/// The 1.8.9 `BiomeGenBase` id of every biome this chain can produce.
pub const OCEAN: i32 = 0;
pub const PLAINS: i32 = 1;
pub const DESERT: i32 = 2;
pub const EXTREME_HILLS: i32 = 3;
pub const FOREST: i32 = 4;
pub const TAIGA: i32 = 5;
pub const SWAMPLAND: i32 = 6;
pub const RIVER: i32 = 7;
pub const HELL: i32 = 8;
pub const THE_END: i32 = 9;
pub const FROZEN_OCEAN: i32 = 10;
pub const FROZEN_RIVER: i32 = 11;
pub const ICE_PLAINS: i32 = 12;
pub const ICE_MOUNTAINS: i32 = 13;
pub const MUSHROOM_ISLAND: i32 = 14;
pub const MUSHROOM_ISLAND_SHORE: i32 = 15;
pub const BEACH: i32 = 16;
pub const DESERT_HILLS: i32 = 17;
pub const FOREST_HILLS: i32 = 18;
pub const TAIGA_HILLS: i32 = 19;
pub const EXTREME_HILLS_EDGE: i32 = 20;
pub const JUNGLE: i32 = 21;
pub const JUNGLE_HILLS: i32 = 22;
pub const JUNGLE_EDGE: i32 = 23;
pub const DEEP_OCEAN: i32 = 24;
pub const STONE_BEACH: i32 = 25;
pub const COLD_BEACH: i32 = 26;
pub const BIRCH_FOREST: i32 = 27;
pub const BIRCH_FOREST_HILLS: i32 = 28;
pub const ROOFED_FOREST: i32 = 29;
pub const COLD_TAIGA: i32 = 30;
pub const COLD_TAIGA_HILLS: i32 = 31;
pub const MEGA_TAIGA: i32 = 32;
pub const MEGA_TAIGA_HILLS: i32 = 33;
pub const EXTREME_HILLS_PLUS: i32 = 34;
pub const SAVANNA: i32 = 35;
pub const SAVANNA_PLATEAU: i32 = 36;
pub const MESA: i32 = 37;
pub const MESA_PLATEAU_F: i32 = 38;
pub const MESA_PLATEAU: i32 = 39;
pub const SUNFLOWER_PLAINS: i32 = 129;
pub const DESERT_M: i32 = 130;
pub const EXTREME_HILLS_M: i32 = 131;
pub const FLOWER_FOREST: i32 = 132;
pub const TAIGA_M: i32 = 133;
pub const SWAMPLAND_M: i32 = 134;
pub const ICE_SPIKES: i32 = 140;
pub const JUNGLE_M: i32 = 149;
pub const JUNGLE_EDGE_M: i32 = 151;
pub const BIRCH_FOREST_M: i32 = 155;
pub const BIRCH_FOREST_HILLS_M: i32 = 156;
pub const ROOFED_FOREST_M: i32 = 157;
pub const COLD_TAIGA_M: i32 = 158;
pub const MEGA_SPRUCE_TAIGA: i32 = 160;
pub const MEGA_SPRUCE_TAIGA_HILLS: i32 = 161;
pub const EXTREME_HILLS_PLUS_M: i32 = 162;
pub const SAVANNA_M: i32 = 163;
pub const SAVANNA_PLATEAU_M: i32 = 164;
pub const MESA_BRYCE: i32 = 165;
pub const MESA_PLATEAU_F_M: i32 = 166;
pub const MESA_PLATEAU_M: i32 = 167;

/// The Java class `Biome.asClass` answers with.
///
/// `MutatedBiome.asClass` delegates to the original, and the specialised
/// mutated variants keep their base class, so 129 is a `PlainsBiome`, 140 an
/// `IceBiome`, 160 a `TaigaBiome`, and so on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BiomeClass {
    Ocean,
    Plains,
    Desert,
    ExtremeHills,
    Forest,
    Taiga,
    Swamp,
    River,
    Nether,
    End,
    Ice,
    Mushroom,
    Beach,
    Jungle,
    StoneBeach,
    Savanna,
    Mesa,
}

/// `Biome.Temperature` as `getBiomeTemperature` derives it from `temperature`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Temperature {
    Cold,
    Medium,
    Warm,
}

/// `(id, class, temperature)` for the biomes `BiomeGenBase` registered in 1.8.9.
///
/// The temperature is the value `Biome.getBiomeTemperature` thresholds, not
/// the snowfall flag: 20 `extreme_hills_edge` and 25 `stone_beach` are 0.2 and
/// therefore `MEDIUM`; 26 `cold_beach` is 0.05 and `COLD`.
const BIOMES: &[(i32, BiomeClass, Temperature)] = &[
    (OCEAN, BiomeClass::Ocean, Temperature::Medium),
    (PLAINS, BiomeClass::Plains, Temperature::Medium),
    (DESERT, BiomeClass::Desert, Temperature::Warm),
    (EXTREME_HILLS, BiomeClass::ExtremeHills, Temperature::Medium),
    (FOREST, BiomeClass::Forest, Temperature::Medium),
    (TAIGA, BiomeClass::Taiga, Temperature::Medium),
    (SWAMPLAND, BiomeClass::Swamp, Temperature::Medium),
    (RIVER, BiomeClass::River, Temperature::Medium),
    (HELL, BiomeClass::Nether, Temperature::Warm),
    (THE_END, BiomeClass::End, Temperature::Medium),
    (FROZEN_OCEAN, BiomeClass::Ocean, Temperature::Cold),
    (FROZEN_RIVER, BiomeClass::River, Temperature::Cold),
    (ICE_PLAINS, BiomeClass::Ice, Temperature::Cold),
    (ICE_MOUNTAINS, BiomeClass::Ice, Temperature::Cold),
    (MUSHROOM_ISLAND, BiomeClass::Mushroom, Temperature::Medium),
    (
        MUSHROOM_ISLAND_SHORE,
        BiomeClass::Mushroom,
        Temperature::Medium,
    ),
    (BEACH, BiomeClass::Beach, Temperature::Medium),
    (DESERT_HILLS, BiomeClass::Desert, Temperature::Warm),
    (FOREST_HILLS, BiomeClass::Forest, Temperature::Medium),
    (TAIGA_HILLS, BiomeClass::Taiga, Temperature::Medium),
    (
        EXTREME_HILLS_EDGE,
        BiomeClass::ExtremeHills,
        Temperature::Medium,
    ),
    (JUNGLE, BiomeClass::Jungle, Temperature::Medium),
    (JUNGLE_HILLS, BiomeClass::Jungle, Temperature::Medium),
    (JUNGLE_EDGE, BiomeClass::Jungle, Temperature::Medium),
    (DEEP_OCEAN, BiomeClass::Ocean, Temperature::Medium),
    (STONE_BEACH, BiomeClass::StoneBeach, Temperature::Medium),
    (COLD_BEACH, BiomeClass::Beach, Temperature::Cold),
    (BIRCH_FOREST, BiomeClass::Forest, Temperature::Medium),
    (BIRCH_FOREST_HILLS, BiomeClass::Forest, Temperature::Medium),
    (ROOFED_FOREST, BiomeClass::Forest, Temperature::Medium),
    (COLD_TAIGA, BiomeClass::Taiga, Temperature::Cold),
    (COLD_TAIGA_HILLS, BiomeClass::Taiga, Temperature::Cold),
    (MEGA_TAIGA, BiomeClass::Taiga, Temperature::Medium),
    (MEGA_TAIGA_HILLS, BiomeClass::Taiga, Temperature::Medium),
    (
        EXTREME_HILLS_PLUS,
        BiomeClass::ExtremeHills,
        Temperature::Medium,
    ),
    (SAVANNA, BiomeClass::Savanna, Temperature::Warm),
    (SAVANNA_PLATEAU, BiomeClass::Savanna, Temperature::Warm),
    (MESA, BiomeClass::Mesa, Temperature::Warm),
    (MESA_PLATEAU_F, BiomeClass::Mesa, Temperature::Warm),
    (MESA_PLATEAU, BiomeClass::Mesa, Temperature::Warm),
    (SUNFLOWER_PLAINS, BiomeClass::Plains, Temperature::Medium),
    (DESERT_M, BiomeClass::Desert, Temperature::Warm),
    (
        EXTREME_HILLS_M,
        BiomeClass::ExtremeHills,
        Temperature::Medium,
    ),
    (FLOWER_FOREST, BiomeClass::Forest, Temperature::Medium),
    (TAIGA_M, BiomeClass::Taiga, Temperature::Medium),
    (SWAMPLAND_M, BiomeClass::Swamp, Temperature::Medium),
    (ICE_SPIKES, BiomeClass::Ice, Temperature::Cold),
    (JUNGLE_M, BiomeClass::Jungle, Temperature::Medium),
    (JUNGLE_EDGE_M, BiomeClass::Jungle, Temperature::Medium),
    (BIRCH_FOREST_M, BiomeClass::Forest, Temperature::Medium),
    (
        BIRCH_FOREST_HILLS_M,
        BiomeClass::Forest,
        Temperature::Medium,
    ),
    (ROOFED_FOREST_M, BiomeClass::Forest, Temperature::Medium),
    (COLD_TAIGA_M, BiomeClass::Taiga, Temperature::Cold),
    (MEGA_SPRUCE_TAIGA, BiomeClass::Taiga, Temperature::Medium),
    (
        MEGA_SPRUCE_TAIGA_HILLS,
        BiomeClass::Taiga,
        Temperature::Medium,
    ),
    (
        EXTREME_HILLS_PLUS_M,
        BiomeClass::ExtremeHills,
        Temperature::Medium,
    ),
    (SAVANNA_M, BiomeClass::Savanna, Temperature::Warm),
    (SAVANNA_PLATEAU_M, BiomeClass::Savanna, Temperature::Warm),
    (MESA_BRYCE, BiomeClass::Mesa, Temperature::Warm),
    (MESA_PLATEAU_F_M, BiomeClass::Mesa, Temperature::Warm),
    (MESA_PLATEAU_M, BiomeClass::Mesa, Temperature::Warm),
];

fn biome(id: i32) -> Option<(BiomeClass, Temperature)> {
    BIOMES
        .iter()
        .find(|entry| entry.0 == id)
        .map(|entry| (entry.1, entry.2))
}

/// `Biome.byId(id) != null` for the ids 1.8.9 registered.
fn biome_exists(id: i32) -> bool {
    biome(id).is_some()
}

/// `Layer.isOcean`: plain, deep and frozen ocean, but not a frozen river.
fn is_ocean(id: i32) -> bool {
    matches!(id, OCEAN | DEEP_OCEAN | FROZEN_OCEAN)
}

/// `Layer.compareBiomes`.
fn compare_biomes(a: i32, b: i32) -> bool {
    if a == b {
        return true;
    }
    if a == MESA_PLATEAU_F || a == MESA_PLATEAU {
        return b == MESA_PLATEAU_F || b == MESA_PLATEAU;
    }
    match (biome(a), biome(b)) {
        (Some((class_a, _)), Some((class_b, _))) => class_a == class_b,
        _ => false,
    }
}

/// `XBiomeLayer.method_6593`: equal biomes, equal temperature classes, or a
/// `MEDIUM` temperature on either side.
fn temperatures_match(a: i32, b: i32) -> bool {
    if compare_biomes(a, b) {
        return true;
    }
    match (biome(a), biome(b)) {
        (Some((_, ta)), Some((_, tb))) => {
            ta == tb || ta == Temperature::Medium || tb == Temperature::Medium
        }
        _ => false,
    }
}

/// `Biome.isMutatedBiome`, which for `MutatedBiome` copies the original's flag.
///
/// The flag is not "is a mutated variant": the base biomes 10, 11, 12, 13, 26,
/// 30 and 31 are marked, mutated 140 inherits 12's flag, and mutated 158
/// inherits 30's. The other mutated variants are not marked.
fn is_mutated(id: i32) -> bool {
    matches!(
        id,
        FROZEN_OCEAN
            | FROZEN_RIVER
            | ICE_PLAINS
            | ICE_MOUNTAINS
            | COLD_BEACH
            | COLD_TAIGA
            | COLD_TAIGA_HILLS
            | ICE_SPIKES
            | COLD_TAIGA_M
    )
}

/// The Java class a 1.8.9 biome id reports, used by the shore layer.
fn biome_class(id: i32) -> Option<BiomeClass> {
    biome(id).map(|(class, _)| class)
}

/// The linear congruential generator `Layer` mixes coordinates with.
///
/// Java's `long` arithmetic wraps; every operation here is `wrapping_*`.
struct LayerRng {
    /// `field_173`: the world seed after `method_144`.
    world_seed: i64,
    /// `field_174`: the per-cell seed `method_145` rebases from the world seed.
    chunk_seed: i64,
    /// `field_175`: the constant derived from the layer's own seed.
    base_seed: i64,
}

const MULTIPLIER: i64 = 6364136223846793005;
const INCREMENT: i64 = 1442695040888963407;

impl LayerRng {
    fn new(seed: i64) -> Self {
        let mut base = seed;
        for _ in 0..3 {
            base = base
                .wrapping_mul(base.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT))
                .wrapping_add(seed);
        }
        Self {
            world_seed: 0,
            chunk_seed: 0,
            base_seed: base,
        }
    }

    fn init_world_seed(&mut self, seed: i64) {
        self.world_seed = seed;
        for _ in 0..3 {
            self.world_seed = self
                .world_seed
                .wrapping_mul(
                    self.world_seed
                        .wrapping_mul(MULTIPLIER)
                        .wrapping_add(INCREMENT),
                )
                .wrapping_add(self.base_seed);
        }
    }

    fn init_chunk_seed(&mut self, x: i32, z: i32) {
        self.chunk_seed = self.world_seed;
        for coordinate in [x as i64, z as i64, x as i64, z as i64] {
            self.chunk_seed = self
                .chunk_seed
                .wrapping_mul(
                    self.chunk_seed
                        .wrapping_mul(MULTIPLIER)
                        .wrapping_add(INCREMENT),
                )
                .wrapping_add(coordinate);
        }
    }

    /// `Layer.nextInt`: the top bits of the cell seed modulo `n`.
    fn next_int(&mut self, n: i32) -> i32 {
        let mut value = ((self.chunk_seed >> 24) % n as i64) as i32;
        if value < 0 {
            value += n;
        }
        self.chunk_seed = self
            .chunk_seed
            .wrapping_mul(
                self.chunk_seed
                    .wrapping_mul(MULTIPLIER)
                    .wrapping_add(INCREMENT),
            )
            .wrapping_add(self.world_seed);
        value
    }

    fn random_of(&mut self, choices: &[i32]) -> i32 {
        choices[self.next_int(choices.len() as i32) as usize]
    }

    /// `Layer.method_6598`: the value shared by three of four neighbours, or a
    /// random one when there is no unambiguous mode.
    fn mode_or_random(&mut self, i: i32, j: i32, k: i32, l: i32) -> i32 {
        if j == k && k == l {
            return j;
        }
        if i == j && i == k {
            return i;
        }
        if i == j && i == l {
            return i;
        }
        if i == k && i == l {
            return i;
        }
        if i == j && k != l {
            return i;
        }
        if i == k && j != l {
            return i;
        }
        if i == l && j != k {
            return i;
        }
        if j == k && i != l {
            return j;
        }
        if j == l && i != k {
            return j;
        }
        if k == l && i != j {
            return k;
        }
        self.random_of(&[i, j, k, l])
    }
}

/// One `GenLayer` node: draws a rectangle of layer-space ids.
trait BiomeLayer {
    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32>;
    fn seed(&mut self, seed: i64);
}

/// The two outputs of `Layer.init` the generator reads.
pub struct BiomeLayers18 {
    /// `layers[0]`: river-mixed biomes, the 10x10 terrain neighbourhood.
    river_mixed: Box<dyn BiomeLayer>,
    /// `layers[1]`: Voronoi zoom over `layers[0]`, the chunk `Biomes` array.
    voronoi: Box<dyn BiomeLayer>,
}

impl BiomeLayers18 {
    /// The default generator's layers for `seed` (`LevelGeneratorType.DEFAULT`).
    pub fn new(seed: i64) -> Self {
        Self::build(seed, 4)
    }

    /// `LevelGeneratorType.LARGE_BIOMES`: the same chain with `biomeSize = 6`.
    pub fn large_biomes(seed: i64) -> Self {
        Self::build(seed, 6)
    }

    fn build(seed: i64, biome_size: u32) -> Self {
        let mut river_mixed = river_mix_chain(biome_size);
        let voronoi_parent = river_mix_chain(biome_size);
        let mut voronoi: Box<dyn BiomeLayer> = Box::new(VoronoiZoomLayer::new(10, voronoi_parent));
        // `Layer.init` seeds `layers[0]` and then `layers[1]`. Both calls
        // assign each layer's world seed before mixing it, so seeding twice is
        // idempotent, as it is in vanilla.
        river_mixed.seed(seed);
        voronoi.seed(seed);
        Self {
            river_mixed,
            voronoi,
        }
    }

    /// `layers[0].getInts`, at the coordinates the 1.8.9 generator asks for.
    pub fn river_mixed(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        self.river_mixed.get_ints(x, z, width, height)
    }

    /// `layers[1].getInts`, the layer the chunk's `Biomes` array comes from.
    pub fn voronoi(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        self.voronoi.get_ints(x, z, width, height)
    }

    /// The 256 biome ids a 1.8.9 chunk at `(chunk_x, chunk_z)` stores.
    ///
    /// This is `layers[1].method_143(chunk_x * 16, chunk_z * 16, 16, 16)` cast
    /// to the byte each column holds.
    pub fn chunk_biomes(&mut self, chunk_x: i32, chunk_z: i32) -> [u8; 256] {
        let ids = self.voronoi(chunk_x * 16, chunk_z * 16, 16, 16);
        let mut biomes = [0; 256];
        for (biome, id) in biomes.iter_mut().zip(ids) {
            debug_assert!((0..=255).contains(&id));
            *biome = id as u8;
        }
        biomes
    }
}

/// `Layer.init`'s shared chain up to `class_1781`, the deep-ocean layer.
///
/// Java shares one instance between the biome and river branches; each branch
/// rebases every cell's seed, so separate instances draw the same values.
fn base_chain() -> Box<dyn BiomeLayer> {
    let mut layer: Box<dyn BiomeLayer> = Box::new(IslandLayer::new(1));
    layer = Box::new(ZoomLayer::new(2000, layer, true));
    layer = Box::new(AddIslandLayer::new(1, layer));
    layer = ZoomLayer::add_zooms(2001, layer, 1);
    layer = Box::new(AddIslandLayer::new(2, layer));
    layer = Box::new(AddIslandLayer::new(50, layer));
    layer = Box::new(AddIslandLayer::new(70, layer));
    layer = Box::new(RemoveTooMuchOceanLayer::new(2, layer));
    layer = Box::new(AddSnowLayer::new(2, layer));
    layer = Box::new(AddIslandLayer::new(3, layer));
    layer = Box::new(EdgeLayer::new(2, layer, EdgeKind::CoolWarm));
    layer = Box::new(EdgeLayer::new(2, layer, EdgeKind::HeatIce));
    layer = Box::new(EdgeLayer::new(3, layer, EdgeKind::Special));
    layer = ZoomLayer::add_zooms(2002, layer, 1);
    layer = ZoomLayer::add_zooms(2003, layer, 1);
    layer = Box::new(AddIslandLayer::new(4, layer));
    layer = Box::new(MushroomIslandLayer::new(5, layer));
    layer = Box::new(DeepOceanLayer::new(4, layer));
    // `Layer.init`'s `class_84.method_148(1000, layer, 0)` adds no zoom; the
    // SetBase and rarity branches both start from this raw chain.
    layer
}

/// The rarity layer (`class_77`) the hill and river branches both start from.
///
/// `Layer.init` applies the rarity layer straight to the base chain; the
/// callers zoom the result twice.
fn rarity_chain() -> Box<dyn BiomeLayer> {
    Box::new(RarityLayer::new(100, base_chain()))
}

/// `Layer.init` from `SetBaseBiomesLayer` to `class_79` (river mix).
fn river_mix_chain(biome_size: u32) -> Box<dyn BiomeLayer> {
    let mut biomes: Box<dyn BiomeLayer> = Box::new(SetBaseBiomesLayer::new(200, base_chain()));
    biomes = ZoomLayer::add_zooms(1000, biomes, 2);
    biomes = Box::new(XBiomeLayer::new(1000, biomes));

    // `AddHillsLayer` never seeds its `field_7604` rarity parent; the shared
    // rarity layer only receives the world seed through the river branch. The
    // zooms above it therefore keep `field_173 == 0`.
    let hills_rarity = ZoomLayer::add_unseeded_zooms(1000, rarity_chain(), 2);
    biomes = Box::new(AddHillsLayer::new(1000, biomes, hills_rarity));

    let mut river = ZoomLayer::add_zooms(1000, rarity_chain(), 2);
    river = ZoomLayer::add_zooms(1000, river, 4);
    river = Box::new(AddRiverLayer::new(1, river));
    river = Box::new(SmoothLayer::new(1000, river));

    biomes = Box::new(RareBiomeLayer::new(1001, biomes));
    for k in 0..biome_size {
        biomes = ZoomLayer::add_zooms(1000 + k as i64, biomes, 1);
        if k == 0 {
            biomes = Box::new(AddIslandLayer::new(3, biomes));
        }
        if k == 1 || biome_size == 1 {
            biomes = Box::new(ShoreLayer::new(1000, biomes));
        }
    }
    biomes = Box::new(SmoothLayer::new(1000, biomes));
    Box::new(RiverMixLayer::new(100, biomes, river))
}

/// `class_74`: the 1-in-10 island seed grid, with one forced island at the
/// origin block so the default spawn has land.
struct IslandLayer {
    rng: LayerRng,
}

impl IslandLayer {
    fn new(seed: i64) -> Self {
        Self {
            rng: LayerRng::new(seed),
        }
    }
}

impl BiomeLayer for IslandLayer {
    fn seed(&mut self, seed: i64) {
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                self.rng.init_chunk_seed(x + column, z + row);
                ints[(column + row * width) as usize] =
                    if self.rng.next_int(10) == 0 { 1 } else { 0 };
            }
        }
        if x > -width && x <= 0 && z > -height && z <= 0 {
            ints[(-x + -z * width) as usize] = 1;
        }
        ints
    }
}

/// `class_84`, and `class_71` when `fuzzy` is set: a zoom that picks a
/// neighbour at random instead of taking the mode.
struct ZoomLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
    fuzzy: bool,
    /// Whether `seed` rebases this layer's own world seed.
    ///
    /// `Layer.init` seeds a chain through `Layer.method_144`, which only walks
    /// `field_172`. `AddHillsLayer`'s rarity parent is `field_7604`, so its
    /// zoom wrappers keep `field_173 == 0`; only the shared rarity layer below
    /// them is seeded, through the river branch. This flag models that.
    seeded: bool,
}

impl ZoomLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>, fuzzy: bool) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
            fuzzy,
            seeded: true,
        }
    }

    fn add_zooms(seed: i64, mut layer: Box<dyn BiomeLayer>, count: u32) -> Box<dyn BiomeLayer> {
        for i in 0..count {
            layer = Box::new(ZoomLayer::new(seed + i as i64, layer, false));
        }
        layer
    }

    /// Zooms whose own world seed is never assigned (`AddHillsLayer`'s rarity
    /// chain). `seed` still reaches the parent.
    fn add_unseeded_zooms(
        seed: i64,
        mut layer: Box<dyn BiomeLayer>,
        count: u32,
    ) -> Box<dyn BiomeLayer> {
        for i in 0..count {
            let mut zoom = ZoomLayer::new(seed + i as i64, layer, false);
            zoom.seeded = false;
            layer = Box::new(zoom);
        }
        layer
    }

    fn mode(&mut self, i: i32, j: i32, k: i32, l: i32) -> i32 {
        if self.fuzzy {
            self.rng.random_of(&[i, j, k, l])
        } else {
            self.rng.mode_or_random(i, j, k, l)
        }
    }
}

impl BiomeLayer for ZoomLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        if self.seeded {
            self.rng.init_world_seed(seed);
        }
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let base_x = x >> 1;
        let base_z = z >> 1;
        let parent_width = (width >> 1) + 2;
        let parent_height = (height >> 1) + 2;
        let parent = self
            .parent
            .get_ints(base_x, base_z, parent_width, parent_height);
        let stride = (parent_width - 1) << 1;
        let rows = (parent_height - 1) << 1;
        let mut zoomed = vec![0; (stride * rows) as usize];
        for row in 0..parent_height - 1 {
            let mut index = (row << 1) * stride;
            let mut left = parent[(row * parent_width) as usize];
            let mut down = parent[((row + 1) * parent_width) as usize];
            for column in 0..parent_width - 1 {
                self.rng
                    .init_chunk_seed((column + base_x) << 1, (row + base_z) << 1);
                let right = parent[(column + 1 + row * parent_width) as usize];
                let down_right = parent[(column + 1 + (row + 1) * parent_width) as usize];
                zoomed[index as usize] = left;
                zoomed[(index + stride) as usize] = self.rng.random_of(&[left, down]);
                index += 1;
                zoomed[index as usize] = self.rng.random_of(&[left, right]);
                zoomed[(index + stride) as usize] = self.mode(left, right, down, down_right);
                index += 1;
                left = right;
                down = down_right;
            }
        }
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            let source = (row + (z & 1)) * stride + (x & 1);
            ints[(row * width) as usize..((row + 1) * width) as usize]
                .copy_from_slice(&zoomed[source as usize..(source + width) as usize]);
        }
        ints
    }
}

/// `class_67`: fills single-cell holes next to land and erodes single-cell
/// land, biased towards the surrounding value.
struct AddIslandLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl AddIslandLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for AddIslandLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent_width = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, parent_width, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let north = parent[(column + row * parent_width) as usize];
                let east = parent[(column + 2 + row * parent_width) as usize];
                let west = parent[(column + (row + 2) * parent_width) as usize];
                let south = parent[(column + 2 + (row + 2) * parent_width) as usize];
                let center = parent[(column + 1 + (row + 1) * parent_width) as usize];
                self.rng.init_chunk_seed(column + x, row + z);
                let out = if center == 0 && (north != 0 || east != 0 || west != 0 || south != 0) {
                    let mut choices = 1;
                    let mut picked = 1;
                    if north != 0 {
                        if self.rng.next_int(choices) == 0 {
                            picked = north;
                        }
                        choices += 1;
                    }
                    if east != 0 {
                        if self.rng.next_int(choices) == 0 {
                            picked = east;
                        }
                        choices += 1;
                    }
                    if west != 0 {
                        if self.rng.next_int(choices) == 0 {
                            picked = west;
                        }
                        choices += 1;
                    }
                    if south != 0 && self.rng.next_int(choices) == 0 {
                        picked = south;
                    }
                    if self.rng.next_int(3) == 0 {
                        picked
                    } else if picked == 4 {
                        4
                    } else {
                        0
                    }
                } else if center > 0 && (north == 0 || east == 0 || west == 0 || south == 0) {
                    if self.rng.next_int(5) == 0 {
                        if center == 4 { 4 } else { 0 }
                    } else {
                        center
                    }
                } else {
                    center
                };
                ints[(column + row * width) as usize] = out;
            }
        }
        ints
    }
}

/// `class_1788`: the ocean cell surrounded by four ocean cells becomes land
/// half the time.
struct RemoveTooMuchOceanLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl RemoveTooMuchOceanLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for RemoveTooMuchOceanLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent_width = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, parent_width, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let north = parent[(column + 1 + row * parent_width) as usize];
                let east = parent[(column + 2 + (row + 1) * parent_width) as usize];
                let west = parent[(column + (row + 1) * parent_width) as usize];
                let south = parent[(column + 1 + (row + 2) * parent_width) as usize];
                let center = parent[(column + 1 + (row + 1) * parent_width) as usize];
                self.rng.init_chunk_seed(column + x, row + z);
                ints[(column + row * width) as usize] = if center == 0
                    && north == 0
                    && east == 0
                    && west == 0
                    && south == 0
                    && self.rng.next_int(2) == 0
                {
                    1
                } else {
                    center
                };
            }
        }
        ints
    }
}

/// `class_69`: assigns each land cell a temperature category 1 (warm) to 4
/// (icy); the six-draw split is 4 on 0, 3 on 1, and 1 otherwise.
struct AddSnowLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl AddSnowLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for AddSnowLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent = self.parent.get_ints(x - 1, z - 1, width + 2, height + 2);
        let stride = width + 2;
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                self.rng.init_chunk_seed(column + x, row + z);
                ints[(column + row * width) as usize] = if center == 0 {
                    0
                } else {
                    let draw = self.rng.next_int(6);
                    if draw == 0 {
                        4
                    } else if draw <= 1 {
                        3
                    } else {
                        1
                    }
                };
            }
        }
        ints
    }
}

#[derive(Clone, Copy)]
enum EdgeKind {
    /// Warm land next to cool or icy land becomes medium.
    CoolWarm,
    /// Icy land next to medium or warm land becomes cool.
    HeatIce,
    /// Marks some land with a random 1..15 in bits 8..11.
    Special,
}

/// `EdgeLayer`: sharpens the boundary between temperature bands.
struct EdgeLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
    kind: EdgeKind,
}

impl EdgeLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>, kind: EdgeKind) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
            kind,
        }
    }
}

impl BiomeLayer for EdgeLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        match self.kind {
            EdgeKind::Special => {
                let parent = self.parent.get_ints(x, z, width, height);
                let mut ints = vec![0; (width * height) as usize];
                for row in 0..height {
                    for column in 0..width {
                        self.rng.init_chunk_seed(column + x, row + z);
                        let mut value = parent[(column + row * width) as usize];
                        if value != 0 && self.rng.next_int(13) == 0 {
                            value |= ((1 + self.rng.next_int(15)) << 8) & 0xF00;
                        }
                        ints[(column + row * width) as usize] = value;
                    }
                }
                ints
            }
            EdgeKind::CoolWarm | EdgeKind::HeatIce => {
                let stride = width + 2;
                let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
                let mut ints = vec![0; (width * height) as usize];
                for row in 0..height {
                    for column in 0..width {
                        let center = parent[(column + 1 + (row + 1) * stride) as usize];
                        self.rng.init_chunk_seed(column + x, row + z);
                        let north = parent[(column + 1 + row * stride) as usize];
                        let east = parent[(column + 2 + (row + 1) * stride) as usize];
                        let west = parent[(column + (row + 1) * stride) as usize];
                        let south = parent[(column + 1 + (row + 2) * stride) as usize];
                        let out = match self.kind {
                            EdgeKind::CoolWarm if center == 1 => {
                                if [north, east, west, south]
                                    .iter()
                                    .any(|&neighbour| neighbour == 3 || neighbour == 4)
                                {
                                    2
                                } else {
                                    center
                                }
                            }
                            EdgeKind::HeatIce if center == 4 => {
                                if [north, east, west, south]
                                    .iter()
                                    .any(|&neighbour| neighbour == 2 || neighbour == 1)
                                {
                                    3
                                } else {
                                    center
                                }
                            }
                            _ => center,
                        };
                        ints[(column + row * width) as usize] = out;
                    }
                }
                ints
            }
        }
    }
}

/// `MushroomIslandLayer`: a rare 14 on an ocean cell whose four corners are
/// ocean too.
struct MushroomIslandLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl MushroomIslandLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for MushroomIslandLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let north_west = parent[(column + row * stride) as usize];
                let north_east = parent[(column + 2 + row * stride) as usize];
                let south_west = parent[(column + (row + 2) * stride) as usize];
                let south_east = parent[(column + 2 + (row + 2) * stride) as usize];
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                self.rng.init_chunk_seed(column + x, row + z);
                ints[(column + row * width) as usize] = if center == 0
                    && north_west == 0
                    && north_east == 0
                    && south_west == 0
                    && south_east == 0
                    && self.rng.next_int(100) == 0
                {
                    MUSHROOM_ISLAND
                } else {
                    center
                };
            }
        }
        ints
    }
}

/// `class_1781`: an ocean cell with more than three ocean neighbours becomes
/// deep ocean.
struct DeepOceanLayer {
    parent: Box<dyn BiomeLayer>,
}

impl DeepOceanLayer {
    fn new(_seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self { parent }
    }
}

impl BiomeLayer for DeepOceanLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let north = parent[(column + 1 + row * stride) as usize];
                let east = parent[(column + 2 + (row + 1) * stride) as usize];
                let west = parent[(column + (row + 1) * stride) as usize];
                let south = parent[(column + 1 + (row + 2) * stride) as usize];
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                let ocean_neighbours = [north, east, west, south]
                    .iter()
                    .filter(|&&neighbour| neighbour == 0)
                    .count();
                ints[(column + row * width) as usize] = if center == 0 && ocean_neighbours > 3 {
                    DEEP_OCEAN
                } else {
                    center
                };
            }
        }
        ints
    }
}

/// `class_77`: land gets a random 2..=300000 rarity value, ocean stays 0.
struct RarityLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl RarityLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for RarityLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent = self.parent.get_ints(x, z, width, height);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                self.rng.init_chunk_seed(column + x, row + z);
                let value = parent[(column + row * width) as usize];
                ints[(column + row * width) as usize] = if value > 0 {
                    self.rng.next_int(299999) + 2
                } else {
                    0
                };
            }
        }
        ints
    }
}

/// `SetBaseBiomesLayer`: turns the temperature category and the special bits
/// into a real biome, with the 1.8.9 biome lists.
struct SetBaseBiomesLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

const WARM_BIOMES: [i32; 6] = [DESERT, DESERT, DESERT, SAVANNA, SAVANNA, PLAINS];
const MEDIUM_BIOMES: [i32; 6] = [
    FOREST,
    ROOFED_FOREST,
    EXTREME_HILLS,
    PLAINS,
    BIRCH_FOREST,
    SWAMPLAND,
];
const COOL_BIOMES: [i32; 4] = [FOREST, EXTREME_HILLS, TAIGA, PLAINS];
const ICY_BIOMES: [i32; 4] = [ICE_PLAINS, ICE_PLAINS, ICE_PLAINS, COLD_TAIGA];

impl SetBaseBiomesLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for SetBaseBiomesLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent = self.parent.get_ints(x, z, width, height);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                self.rng.init_chunk_seed(column + x, row + z);
                let value = parent[(column + row * width) as usize];
                let special = (value & 0xF00) >> 8;
                let value = value & !0xF00;
                let out = if is_ocean(value) || value == MUSHROOM_ISLAND {
                    value
                } else if value == 1 {
                    if special > 0 {
                        if self.rng.next_int(3) == 0 {
                            MESA_PLATEAU
                        } else {
                            MESA_PLATEAU_F
                        }
                    } else {
                        self.rng.random_of(&WARM_BIOMES)
                    }
                } else if value == 2 {
                    if special > 0 {
                        JUNGLE
                    } else {
                        self.rng.random_of(&MEDIUM_BIOMES)
                    }
                } else if value == 3 {
                    if special > 0 {
                        MEGA_TAIGA
                    } else {
                        self.rng.random_of(&COOL_BIOMES)
                    }
                } else if value == 4 {
                    self.rng.random_of(&ICY_BIOMES)
                } else {
                    MUSHROOM_ISLAND
                };
                ints[(column + row * width) as usize] = out;
            }
        }
        ints
    }
}

/// `XBiomeLayer`: turns extreme hills into their edge variant next to a
/// different climate, keeps mesas coherent, and derives jungle edge and a few
/// climate-specific variants.
struct XBiomeLayer {
    parent: Box<dyn BiomeLayer>,
}

impl XBiomeLayer {
    fn new(_seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self { parent }
    }

    /// `method_6594`: a top biome whose four neighbours are not all the base
    /// biome becomes the replacement.
    fn replace_if_neighbour(
        parent: &[i32],
        stride: i32,
        column: i32,
        row: i32,
        top: i32,
        base: i32,
        replacement: i32,
    ) -> Option<i32> {
        if top != base {
            return None;
        }
        let neighbours = Self::neighbours(parent, stride, column, row);
        if neighbours
            .iter()
            .all(|&neighbour| compare_biomes(neighbour, base))
        {
            Some(top)
        } else {
            Some(replacement)
        }
    }

    fn neighbours(parent: &[i32], stride: i32, column: i32, row: i32) -> [i32; 4] {
        [
            parent[(column + 1 + row * stride) as usize],
            parent[(column + 2 + (row + 1) * stride) as usize],
            parent[(column + (row + 1) * stride) as usize],
            parent[(column + 1 + (row + 2) * stride) as usize],
        ]
    }
}

impl BiomeLayer for XBiomeLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                let neighbours = Self::neighbours(&parent, stride, column, row);
                let mut out = center;
                if compare_biomes(center, EXTREME_HILLS) {
                    out = if neighbours
                        .iter()
                        .all(|&neighbour| temperatures_match(neighbour, EXTREME_HILLS_EDGE))
                    {
                        center
                    } else {
                        EXTREME_HILLS_EDGE
                    };
                } else if let Some(value) = Self::replace_if_neighbour(
                    &parent,
                    stride,
                    column,
                    row,
                    center,
                    MESA_PLATEAU_F,
                    MESA,
                ) {
                    out = value;
                } else if let Some(value) = Self::replace_if_neighbour(
                    &parent,
                    stride,
                    column,
                    row,
                    center,
                    MESA_PLATEAU,
                    MESA,
                ) {
                    out = value;
                } else if let Some(value) = Self::replace_if_neighbour(
                    &parent, stride, column, row, center, MEGA_TAIGA, TAIGA,
                ) {
                    out = value;
                } else if center == DESERT {
                    if neighbours.iter().any(|&neighbour| neighbour == ICE_PLAINS) {
                        out = EXTREME_HILLS_PLUS;
                    }
                } else if center == SWAMPLAND {
                    if neighbours
                        .iter()
                        .any(|&neighbour| matches!(neighbour, DESERT | COLD_TAIGA | ICE_PLAINS))
                    {
                        out = PLAINS;
                    } else if neighbours.iter().any(|&neighbour| neighbour == JUNGLE) {
                        out = JUNGLE_EDGE;
                    }
                }
                ints[(column + row * width) as usize] = out;
            }
        }
        ints
    }
}

/// `AddHillsLayer`: a third of the land grows hills, with the rarity layer
/// forcing a rarer mutated variant.
struct AddHillsLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
    rarity: Box<dyn BiomeLayer>,
}

impl AddHillsLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>, rarity: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
            rarity,
        }
    }
}

impl BiomeLayer for AddHillsLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        // `AddHillsLayer` inherits `Layer.method_144`, which does not reach
        // `field_7604`. Vanilla still seeds the shared rarity layer through the
        // river branch; seeding this separate copy gives the same values
        // because `class_77` rebases from the world seed per cell.
        self.rarity.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let rarity = self.rarity.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                self.rng.init_chunk_seed(column + x, row + z);
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                let rarity_value = rarity[(column + 1 + (row + 1) * stride) as usize];
                let forced = (rarity_value - 2) % 29 == 0;
                if center != 0 && rarity_value >= 2 && (rarity_value - 2) % 29 == 1 && center < 128
                {
                    ints[(column + row * width) as usize] = if biome_exists(center + 128) {
                        center + 128
                    } else {
                        center
                    };
                    continue;
                }
                if self.rng.next_int(3) == 0 || forced {
                    let mut hill = center;
                    if center == DESERT {
                        hill = DESERT_HILLS;
                    } else if center == FOREST {
                        hill = FOREST_HILLS;
                    } else if center == BIRCH_FOREST {
                        hill = BIRCH_FOREST_HILLS;
                    } else if center == ROOFED_FOREST {
                        hill = PLAINS;
                    } else if center == TAIGA {
                        hill = TAIGA_HILLS;
                    } else if center == MEGA_TAIGA {
                        hill = MEGA_TAIGA_HILLS;
                    } else if center == COLD_TAIGA {
                        hill = COLD_TAIGA_HILLS;
                    } else if center == PLAINS {
                        hill = if self.rng.next_int(3) == 0 {
                            FOREST_HILLS
                        } else {
                            FOREST
                        };
                    } else if center == ICE_PLAINS {
                        hill = ICE_MOUNTAINS;
                    } else if center == JUNGLE {
                        hill = JUNGLE_HILLS;
                    } else if center == OCEAN {
                        hill = DEEP_OCEAN;
                    } else if center == EXTREME_HILLS {
                        hill = EXTREME_HILLS_PLUS;
                    } else if center == SAVANNA {
                        hill = SAVANNA_PLATEAU;
                    } else if compare_biomes(center, MESA_PLATEAU_F) {
                        hill = MESA;
                    } else if center == DEEP_OCEAN && self.rng.next_int(3) == 0 {
                        hill = if self.rng.next_int(2) == 0 {
                            PLAINS
                        } else {
                            FOREST
                        };
                    }
                    if forced && hill != center {
                        hill = if biome_exists(hill + 128) {
                            hill + 128
                        } else {
                            center
                        };
                    }
                    if hill == center {
                        ints[(column + row * width) as usize] = center;
                        continue;
                    }
                    let neighbours = XBiomeLayer::neighbours(&parent, stride, column, row);
                    let same = neighbours
                        .iter()
                        .filter(|&&neighbour| compare_biomes(neighbour, center))
                        .count();
                    ints[(column + row * width) as usize] = if same >= 3 { hill } else { center };
                    continue;
                }
                ints[(column + row * width) as usize] = center;
            }
        }
        ints
    }
}

/// `AddRiverLayer`: -1 on a cell whose climate category differs from any
/// neighbour, 7 (river) otherwise.
struct AddRiverLayer {
    parent: Box<dyn BiomeLayer>,
}

impl AddRiverLayer {
    fn new(_seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self { parent }
    }

    /// `method_6601`: preserves 2/3 while collapsing the rest to those bands.
    fn band(value: i32) -> i32 {
        if value >= 2 { 2 + (value & 1) } else { value }
    }
}

impl BiomeLayer for AddRiverLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let west = Self::band(parent[(column + (row + 1) * stride) as usize]);
                let east = Self::band(parent[(column + 2 + (row + 1) * stride) as usize]);
                let north = Self::band(parent[(column + 1 + row * stride) as usize]);
                let south = Self::band(parent[(column + 1 + (row + 2) * stride) as usize]);
                let center = Self::band(parent[(column + 1 + (row + 1) * stride) as usize]);
                ints[(column + row * width) as usize] =
                    if center != west || center != north || center != east || center != south {
                        RIVER
                    } else {
                        -1
                    };
            }
        }
        ints
    }
}

/// `class_81`: removes single-cell noise by majority of the four direct
/// neighbours.
struct SmoothLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl SmoothLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for SmoothLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let west = parent[(column + (row + 1) * stride) as usize];
                let east = parent[(column + 2 + (row + 1) * stride) as usize];
                let north = parent[(column + 1 + row * stride) as usize];
                let south = parent[(column + 1 + (row + 2) * stride) as usize];
                let mut center = parent[(column + 1 + (row + 1) * stride) as usize];
                if west == east && north == south {
                    self.rng.init_chunk_seed(column + x, row + z);
                    center = if self.rng.next_int(2) == 0 {
                        west
                    } else {
                        north
                    };
                } else {
                    if west == east {
                        center = west;
                    }
                    if north == south {
                        center = north;
                    }
                }
                ints[(column + row * width) as usize] = center;
            }
        }
        ints
    }
}

/// `class_82`: one in 57 plains cells becomes sunflower plains.
struct RareBiomeLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl RareBiomeLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for RareBiomeLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let parent = self.parent.get_ints(x, z, width, height);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                self.rng.init_chunk_seed(column + x, row + z);
                let value = parent[(column + row * width) as usize];
                ints[(column + row * width) as usize] = if self.rng.next_int(57) == 0 {
                    if value == PLAINS {
                        SUNFLOWER_PLAINS
                    } else {
                        value
                    }
                } else {
                    value
                };
            }
        }
        ints
    }
}

/// `class_80`: turns coastlines into beaches, and jungle or mushroom coasts
/// into their own shore variants.
struct ShoreLayer {
    parent: Box<dyn BiomeLayer>,
}

impl ShoreLayer {
    fn new(_seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self { parent }
    }

    /// `method_6602`: replace `top` with `shore` when any neighbour is ocean.
    fn shore(parent: &[i32], stride: i32, column: i32, row: i32, top: i32, shore: i32) -> i32 {
        if is_ocean(top) {
            return top;
        }
        let neighbours = XBiomeLayer::neighbours(parent, stride, column, row);
        if neighbours.iter().any(|&neighbour| is_ocean(neighbour)) {
            shore
        } else {
            top
        }
    }

    /// `method_6603`: a biome a jungle may border without becoming jungle edge.
    fn jungle_compatible(id: i32) -> bool {
        biome_class(id) == Some(BiomeClass::Jungle)
            || matches!(id, JUNGLE_EDGE | JUNGLE | JUNGLE_HILLS | FOREST | TAIGA)
            || is_ocean(id)
    }

    /// `method_6604`: an actual `MesaBiome`, which excludes the mutated mesas
    /// (`MutatedBiome` is not a `MesaBiome`).
    fn mesa(id: i32) -> bool {
        matches!(id, MESA | MESA_PLATEAU_F | MESA_PLATEAU)
    }
}

fn bio_class_is(id: i32, class: BiomeClass) -> bool {
    biome_class(id) == Some(class)
}

impl BiomeLayer for ShoreLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let stride = width + 2;
        let parent = self.parent.get_ints(x - 1, z - 1, stride, height + 2);
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            for column in 0..width {
                let center = parent[(column + 1 + (row + 1) * stride) as usize];
                let neighbours = XBiomeLayer::neighbours(&parent, stride, column, row);
                let out = if center == MUSHROOM_ISLAND {
                    if neighbours.iter().any(|&neighbour| neighbour == OCEAN) {
                        MUSHROOM_ISLAND_SHORE
                    } else {
                        center
                    }
                } else if bio_class_is(center, BiomeClass::Jungle) {
                    if !neighbours
                        .iter()
                        .all(|&neighbour| Self::jungle_compatible(neighbour))
                    {
                        JUNGLE_EDGE
                    } else if neighbours.iter().any(|&neighbour| is_ocean(neighbour)) {
                        BEACH
                    } else {
                        center
                    }
                } else if matches!(
                    center,
                    EXTREME_HILLS | EXTREME_HILLS_PLUS | EXTREME_HILLS_EDGE
                ) {
                    Self::shore(&parent, stride, column, row, center, STONE_BEACH)
                } else if is_mutated(center) {
                    Self::shore(&parent, stride, column, row, center, COLD_BEACH)
                } else if matches!(center, MESA | MESA_PLATEAU_F) {
                    if neighbours.iter().any(|&neighbour| is_ocean(neighbour)) {
                        center
                    } else if !neighbours.iter().all(|&neighbour| Self::mesa(neighbour)) {
                        DESERT
                    } else {
                        center
                    }
                } else if !matches!(center, OCEAN | DEEP_OCEAN | RIVER | SWAMPLAND) {
                    Self::shore(&parent, stride, column, row, center, BEACH)
                } else {
                    center
                };
                ints[(column + row * width) as usize] = out;
            }
        }
        ints
    }
}

/// `class_79`: lays the river band over the biome layer.
struct RiverMixLayer {
    parent: Box<dyn BiomeLayer>,
    river: Box<dyn BiomeLayer>,
}

impl RiverMixLayer {
    fn new(_seed: i64, parent: Box<dyn BiomeLayer>, river: Box<dyn BiomeLayer>) -> Self {
        Self { parent, river }
    }
}

impl BiomeLayer for RiverMixLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.river.seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let biomes = self.parent.get_ints(x, z, width, height);
        let rivers = self.river.get_ints(x, z, width, height);
        let mut ints = vec![0; (width * height) as usize];
        for i in 0..(width * height) as usize {
            let biome = biomes[i];
            ints[i] = if biome == OCEAN || biome == DEEP_OCEAN {
                biome
            } else if rivers[i] == RIVER {
                if biome == ICE_PLAINS {
                    FROZEN_RIVER
                } else if biome == MUSHROOM_ISLAND || biome == MUSHROOM_ISLAND_SHORE {
                    MUSHROOM_ISLAND_SHORE
                } else {
                    rivers[i] & 0xFF
                }
            } else {
                biome
            };
        }
        ints
    }
}

/// `class_83` (`GenLayerVoronoiZoom`): the noisy 4x4 block zoom whose output is
/// the chunk's stored biome array.
struct VoronoiZoomLayer {
    rng: LayerRng,
    parent: Box<dyn BiomeLayer>,
}

impl VoronoiZoomLayer {
    fn new(seed: i64, parent: Box<dyn BiomeLayer>) -> Self {
        Self {
            rng: LayerRng::new(seed),
            parent,
        }
    }
}

impl BiomeLayer for VoronoiZoomLayer {
    fn seed(&mut self, seed: i64) {
        self.parent.seed(seed);
        self.rng.init_world_seed(seed);
    }

    fn get_ints(&mut self, x: i32, z: i32, width: i32, height: i32) -> Vec<i32> {
        let x = x - 2;
        let z = z - 2;
        let base_x = x >> 2;
        let base_z = z >> 2;
        let parent_width = (width >> 2) + 2;
        let parent_height = (height >> 2) + 2;
        let parent = self
            .parent
            .get_ints(base_x, base_z, parent_width, parent_height);
        let stride = (parent_width - 1) << 2;
        let rows = (parent_height - 1) << 2;
        // `IntArrayCache.get` hands back a 256-entry array for any request up
        // to 256, so a smaller `q * r` leaves a zeroed tail that the final
        // copy can read. Vanilla only asks the Voronoi layer for 16x16 chunks,
        // where the buffer is exactly sized.
        let mut zoomed = vec![0; ((stride * rows) as usize).max(256)];
        for row in 0..parent_height - 1 {
            let mut top_left = parent[(row * parent_width) as usize];
            let mut bottom_left = parent[((row + 1) * parent_width) as usize];
            for column in 0..parent_width - 1 {
                self.rng
                    .init_chunk_seed((column + base_x) << 2, (row + base_z) << 2);
                let left = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                let top = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                self.rng
                    .init_chunk_seed((column + base_x + 1) << 2, (row + base_z) << 2);
                let right = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let top_right = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                self.rng
                    .init_chunk_seed((column + base_x) << 2, (row + base_z + 1) << 2);
                let bottom = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                let bottom_left_noise = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                self.rng
                    .init_chunk_seed((column + base_x + 1) << 2, (row + base_z + 1) << 2);
                let bottom_right = (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let bottom_right_noise =
                    (self.rng.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let top_right_value = parent[(column + 1 + row * parent_width) as usize] & 0xFF;
                let bottom_right_value =
                    parent[(column + 1 + (row + 1) * parent_width) as usize] & 0xFF;
                for cell_z in 0..4 {
                    let mut index = ((row << 2) + cell_z) * stride + (column << 2);
                    for cell_x in 0..4 {
                        let dz = cell_z as f64;
                        let dx = cell_x as f64;
                        let top_left_distance = (dz - top) * (dz - top) + (dx - left) * (dx - left);
                        let bottom_left_distance =
                            (dz - top_right) * (dz - top_right) + (dx - right) * (dx - right);
                        let top_right_distance = (dz - bottom_left_noise)
                            * (dz - bottom_left_noise)
                            + (dx - bottom) * (dx - bottom);
                        let bottom_right_distance = (dz - bottom_right_noise)
                            * (dz - bottom_right_noise)
                            + (dx - bottom_right) * (dx - bottom_right);
                        zoomed[index as usize] = if top_left_distance < bottom_left_distance
                            && top_left_distance < top_right_distance
                            && top_left_distance < bottom_right_distance
                        {
                            top_left
                        } else if bottom_left_distance < top_left_distance
                            && bottom_left_distance < top_right_distance
                            && bottom_left_distance < bottom_right_distance
                        {
                            top_right_value
                        } else if top_right_distance < top_left_distance
                            && top_right_distance < bottom_left_distance
                            && top_right_distance < bottom_right_distance
                        {
                            bottom_left
                        } else {
                            bottom_right_value
                        };
                        index += 1;
                    }
                }
                top_left = top_right_value;
                bottom_left = bottom_right_value;
            }
        }
        let mut ints = vec![0; (width * height) as usize];
        for row in 0..height {
            let source = (row + (z & 3)) * stride + (x & 3);
            ints[(row * width) as usize..((row + 1) * width) as usize]
                .copy_from_slice(&zoomed[source as usize..(source + width) as usize]);
        }
        ints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case {
        layer: String,
        kind: String,
        seed: i64,
        x: i32,
        z: i32,
        width: i32,
        height: i32,
        values: Vec<i32>,
    }

    fn oracle() -> Vec<Case> {
        include_str!("../tests/fixtures/biome_layers_1_8_9.txt")
            .lines()
            .filter(|line| line.starts_with("layer="))
            .map(|line| {
                let mut case = Case {
                    layer: String::new(),
                    kind: String::new(),
                    seed: 0,
                    x: 0,
                    z: 0,
                    width: 0,
                    height: 0,
                    values: Vec::new(),
                };
                for field in line.split(';') {
                    let (key, value) = field.split_once('=').expect("key=value");
                    match key {
                        "layer" => case.layer = value.to_owned(),
                        "type" => case.kind = value.to_owned(),
                        "seed" => case.seed = value.parse().expect("seed"),
                        "x" => case.x = value.parse().expect("x"),
                        "z" => case.z = value.parse().expect("z"),
                        "w" => case.width = value.parse().expect("width"),
                        "h" => case.height = value.parse().expect("height"),
                        "values" => {
                            case.values = value
                                .split(',')
                                .map(|id| id.parse().expect("biome id"))
                                .collect()
                        }
                        other => panic!("unknown fixture field {other:?}"),
                    }
                }
                assert_eq!(case.values.len(), (case.width * case.height) as usize);
                case
            })
            .collect()
    }

    /// The fixture is `LayerOracle18` run against the 1.8.9 named jar: five
    /// seeds on the default generator and one on large biomes, each at two
    /// Voronoi chunk origins, a small negative origin, and the generator's
    /// 10x10 and 5x3 river-layer neighbourhoods.
    #[test]
    fn matches_the_one_eight_nine_layer_chain() {
        let cases = oracle();
        assert_eq!(cases.len(), 30);
        for case in cases {
            let mut layers = if case.kind == "large" {
                BiomeLayers18::large_biomes(case.seed)
            } else {
                BiomeLayers18::new(case.seed)
            };
            let got = if case.layer == "voronoi" {
                layers.voronoi(case.x, case.z, case.width, case.height)
            } else {
                assert_eq!(case.layer, "river");
                layers.river_mixed(case.x, case.z, case.width, case.height)
            };
            assert_eq!(
                got, case.values,
                "{} {} seed {} at ({}, {}) {}x{}",
                case.layer, case.kind, case.seed, case.x, case.z, case.width, case.height
            );
        }
    }

    #[test]
    fn the_chunk_biomes_are_the_voronoi_layer_as_bytes() {
        let mut layers = BiomeLayers18::new(12345);
        let ids = layers.voronoi(32, -48, 16, 16);
        let bytes = layers.chunk_biomes(2, -3);
        assert_eq!(ids.len(), bytes.len());
        for (id, byte) in ids.iter().zip(bytes) {
            assert_eq!(*id as u8, byte);
        }
        assert_eq!(bytes, layers.chunk_biomes(2, -3));
    }

    #[test]
    fn every_produced_biome_is_a_one_eight_nine_id() {
        for seed in [0, 1, 12345, -987654321] {
            let mut layers = BiomeLayers18::new(seed);
            for (x, z) in [(0, 0), (16, -16), (-32, 48), (160, 160), (-160, -160)] {
                for id in layers.chunk_biomes(x / 16, z / 16) {
                    assert!(
                        biome_exists(id as i32),
                        "seed {seed} at ({x}, {z}) produced {id}"
                    );
                }
            }
        }
    }
}
