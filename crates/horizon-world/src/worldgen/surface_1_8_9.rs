//! Minecraft 1.8.9's per-biome surface pass.
//!
//! `SurfaceChunkGenerator.method_6546` samples the 4-octave surface noise over
//! the chunk at a 1-block step, then hands every column to its biome's
//! `method_6420`. The base body (`Biome.method_8590`) walks the column from the
//! top down: bedrock pockets from the per-chunk `Random`, then the top block
//! and a filler depth around sea level, gravel deep below it, water or ice over
//! the sea, and a sandstone floor under sand. Each biome class can replace the
//! top/filler pair or the whole column first: extreme hills turn to gravel and
//! stone high up, taigas grow podzol and coarse dirt, swamps flood y = 62 and
//! grow lily pads, savanna variants go stony, and mesas paint hardened clay
//! bands with their own seed-keyed layer palette and plateau noise.
//!
//! 1.8.9 passes the **transposed** column coordinates: `method_6546`'s first
//! argument is `chunk_x * 16 + z_local` and its second is
//! `chunk_z * 16 + x_local`, while the two low bits index `(x, z)` as
//! `(second & 15, first & 15)`. `Biome.getTemperature`, the swamp foliage
//! noise and the mesa plateau noise therefore read transposed world
//! coordinates. The port keeps that exactly; a chunk only matches a 1.8.9
//! server if the bug is reproduced.
//!
//! `tests/fixtures/surface_1_8_9.txt` pins nine chunks from the 1.8.9 jar:
//! their 16x16 biome ids, the 256 surface-noise doubles by bits, and every
//! column's block-state runs, with mesa, extreme hills, mega taiga, swamp,
//! snowy and beach chunks among them.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use horizon_core::random::{LegacyRandom, RandomSource};

use super::noise_1_8_9::{PerlinNoise18, TerrainNoise18};
use super::terrain_1_8_9::{self, CHUNK_VOLUME, DensityBlock18, TerrainDensity18};
use crate::biome_terrain_1_8_9::{self, BiomeTerrain18, LegacyBlock18, PLAINS, Surface18};

/// One chunk's block states, `id << 4 | meta`, indexed
/// `x << 12 | z << 8 | y` as `ChunkBlockStateStorage` indexes them.
pub const STORAGE_LEN: usize = CHUNK_VOLUME;

const AIR: u16 = 0;
const STONE: u16 = 1 << 4;
const GRASS: u16 = 2 << 4;
const DIRT: u16 = 3 << 4;
const COARSE_DIRT: u16 = 3 << 4 | 1;
const PODZOL: u16 = 3 << 4 | 2;
const BEDROCK: u16 = 7 << 4;
const WATER: u16 = 9 << 4;
const SAND: u16 = 12 << 4;
const ICE: u16 = 79 << 4;
const GRAVEL: u16 = 13 << 4;
const SANDSTONE: u16 = 24 << 4;
const LILY_PAD: u16 = 111 << 4;
const STAINED_TERRACOTTA: u16 = 159 << 4;
const STAINED_WHITE: u16 = STAINED_TERRACOTTA;
const STAINED_ORANGE: u16 = 159 << 4 | 1;
const STAINED_YELLOW: u16 = 159 << 4 | 4;
const STAINED_SILVER: u16 = 159 << 4 | 8;
const STAINED_BROWN: u16 = 159 << 4 | 12;
const STAINED_RED: u16 = 159 << 4 | 14;
const TERRACOTTA: u16 = 172 << 4;
const RED_SANDSTONE: u16 = 179 << 4;

/// Writes `method_9194`'s stone/water/air lattice into the
/// `ChunkBlockStateStorage` layout the surface pass edits.
///
/// `density` is indexed `x + z * 16 + y * 256`; the storage is
/// `x << 12 | z << 8 | y`.
pub fn fill_from_density(density: &[DensityBlock18], storage: &mut [u16]) {
    assert_eq!(density.len(), CHUNK_VOLUME, "density column count");
    assert_eq!(storage.len(), STORAGE_LEN, "chunk storage length");
    for y in 0..256usize {
        for z in 0..16usize {
            for x in 0..16usize {
                storage[x << 12 | z << 8 | y] = match density[x + z * 16 + y * 256] {
                    DensityBlock18::Air => AIR,
                    DensityBlock18::Stone => STONE,
                    DensityBlock18::Water => WATER,
                };
            }
        }
    }
}

/// The mutable state `SurfaceChunkGenerator` keeps between chunks: the
/// per-chunk `Random`, the surface-noise grid, and the per-biome mesa caches.
pub struct SurfaceState18 {
    random: LegacyRandom,
    noise: Vec<f64>,
    mesas: BTreeMap<u8, MesaState18>,
}

impl Default for SurfaceState18 {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceState18 {
    pub fn new() -> Self {
        Self {
            random: LegacyRandom::new(0),
            noise: vec![0.0; 256],
            mesas: BTreeMap::new(),
        }
    }

    /// The `field_4839` surface-noise grid of the last chunk (`x + z * 16`).
    pub fn noise(&self) -> &[f64] {
        &self.noise
    }

    /// `method_6546`: sample the surface noise over the chunk and run every
    /// column's biome surface body over `storage`, which must already hold
    /// [`fill_from_density`]'s blocks.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        noises: &TerrainNoise18,
        world_seed: i64,
        biomes: &[u8],
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        storage: &mut [u16],
    ) {
        assert_eq!(biomes.len(), 256, "16x16 biome ids");
        assert_eq!(storage.len(), STORAGE_LEN, "chunk storage length");
        self.noise = noises.surface.sample_2d(
            f64::from(chunk_x * 16),
            f64::from(chunk_z * 16),
            16,
            16,
            0.0625,
            0.0625,
            1.0,
        );
        // `getChunk` reseeds the generator's `Random` before the surface pass.
        self.random = LegacyRandom::new(
            i64::from(chunk_x) * 341_873_128_712 + i64::from(chunk_z) * 132_897_987_541,
        );
        for z_local in 0..16usize {
            for x_local in 0..16usize {
                let biome_id = biomes[x_local + z_local * 16];
                // 1.8's transposed coordinates, see the module docs.
                let i = chunk_x * 16 + z_local as i32;
                let j = chunk_z * 16 + x_local as i32;
                let d = self.noise[x_local + z_local * 16];
                let terrain = biome_row(biome_id);
                let top = legacy_state(terrain.top);
                let filler = legacy_state(terrain.filler);
                match terrain.surface {
                    Surface18::Mesa => {
                        let mesa = self.mesas.entry(biome_id).or_default();
                        mesa.column(
                            world_seed,
                            &mut self.random,
                            top,
                            filler,
                            terrain.variant,
                            i,
                            j,
                            d,
                            sea_level,
                            storage,
                        );
                    }
                    Surface18::Swamp => swamp_column(
                        &mut self.random,
                        terrain,
                        top,
                        filler,
                        i,
                        j,
                        d,
                        sea_level,
                        storage,
                    ),
                    Surface18::ExtremeHills => {
                        let (top, filler) = if (d < -1.0 || d > 2.0) && terrain.variant == 2 {
                            (GRAVEL, GRAVEL)
                        } else if d > 1.0 && terrain.variant != 1 {
                            (STONE, STONE)
                        } else {
                            (top, filler)
                        };
                        plain_column(
                            &mut self.random,
                            terrain,
                            top,
                            filler,
                            i,
                            j,
                            d,
                            sea_level,
                            storage,
                        );
                    }
                    Surface18::Taiga => {
                        let (top, filler) = if terrain.variant == 1 || terrain.variant == 2 {
                            if d > 1.75 {
                                (COARSE_DIRT, DIRT)
                            } else if d > -0.95 {
                                (PODZOL, DIRT)
                            } else {
                                (GRASS, DIRT)
                            }
                        } else {
                            (top, filler)
                        };
                        plain_column(
                            &mut self.random,
                            terrain,
                            top,
                            filler,
                            i,
                            j,
                            d,
                            sea_level,
                            storage,
                        );
                    }
                    Surface18::ShatteredSavanna => {
                        let (top, filler) = if d > 1.75 {
                            (STONE, STONE)
                        } else if d > -0.5 {
                            (COARSE_DIRT, DIRT)
                        } else {
                            (GRASS, DIRT)
                        };
                        plain_column(
                            &mut self.random,
                            terrain,
                            top,
                            filler,
                            i,
                            j,
                            d,
                            sea_level,
                            storage,
                        );
                    }
                    Surface18::Plain | Surface18::Ocean => plain_column(
                        &mut self.random,
                        terrain,
                        top,
                        filler,
                        i,
                        j,
                        d,
                        sea_level,
                        storage,
                    ),
                    // `MutatedBiome.method_6420` delegates before this match.
                    Surface18::Mutated => unreachable!("mutated biomes resolve to their base"),
                }
            }
        }
    }
}

/// The 1.8.9 overworld terrain stage: `method_6544`'s density lattice, then
/// `method_9194`'s blocks, then `method_6546`'s biome surface.
///
/// One per generator thread: the density buffers and the mesa caches are
/// reused across chunks, and the mesa height noises remember the previous
/// world just as the shared `Biome` instances do.
pub struct ChunkGenerator18 {
    density: TerrainDensity18,
    surface: SurfaceState18,
    /// `LevelGeneratorType.AMPLIFIED`: doubled positive biome depth and
    /// quadrupled variation in the density blend.
    amplified: bool,
}

impl ChunkGenerator18 {
    pub fn new(seed: i64, amplified: bool) -> Self {
        Self {
            density: TerrainDensity18::new(seed),
            surface: SurfaceState18::new(),
            amplified,
        }
    }

    /// A chunk's block states in `ChunkBlockStateStorage` order (`x << 12 |
    /// z << 8 | y`, `id << 4 | meta`), with no carvers, features or
    /// structures.
    ///
    /// `river_biomes` is `method_3857`'s 10x10 density neighbourhood and
    /// `voronoi_biomes` the 16x16 Voronoi array the surface pass and the
    /// saved `Biomes` tag use.
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        &mut self,
        world_seed: i64,
        river_biomes: &[u8],
        voronoi_biomes: &[u8],
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
    ) -> Vec<u16> {
        assert_eq!(river_biomes.len(), 100, "10x10 river-mixed biomes");
        assert_eq!(voronoi_biomes.len(), 256, "16x16 Voronoi biomes");
        let density = self
            .density
            .fill(river_biomes, chunk_x, chunk_z, self.amplified)
            .to_vec();
        let mut storage = vec![0u16; STORAGE_LEN];
        fill_from_density(
            &terrain_1_8_9::fill_blocks(&density, sea_level),
            &mut storage,
        );
        self.surface.apply(
            self.density.noises(),
            world_seed,
            voronoi_biomes,
            chunk_x,
            chunk_z,
            sea_level,
            &mut storage,
        );
        storage
    }
}

/// `Block.getByBlockState`: the 1.8 block id and metadata as the storage
/// packs them. The table's metadata is already four bits.
fn legacy_state(block: LegacyBlock18) -> u16 {
    block.0 << 4 | u16::from(block.1 & 0x0F)
}

/// The biome row whose `method_6420` body a column runs: a mutated biome
/// delegates to its base row (`id - 128`), whose top, filler and temperature
/// the body reads.
fn biome_row(id: u8) -> BiomeTerrain18 {
    let terrain = biome_terrain_1_8_9::terrain(id).unwrap_or(PLAINS);
    if terrain.surface == Surface18::Mutated {
        biome_terrain_1_8_9::terrain(id.wrapping_sub(128)).unwrap_or(PLAINS)
    } else {
        terrain
    }
}

fn temperature_noise() -> &'static PerlinNoise18 {
    static NOISE: OnceLock<PerlinNoise18> = OnceLock::new();
    NOISE.get_or_init(|| PerlinNoise18::new(&mut LegacyRandom::new(1234), 1))
}

fn foliage_noise() -> &'static PerlinNoise18 {
    static NOISE: OnceLock<PerlinNoise18> = OnceLock::new();
    NOISE.get_or_init(|| PerlinNoise18::new(&mut LegacyRandom::new(2345), 1))
}

/// `Biome.getTemperature(BlockPos)`: the biome temperature, cooled with
/// altitude above y = 64 by the shared `TEMPERATURE_NOISE`.
fn biome_temperature(terrain: BiomeTerrain18, i: i32, y: i32, j: i32) -> f32 {
    if y > 64 {
        let f = (temperature_noise().sample(f64::from(i) / 8.0, f64::from(j) / 8.0) * 4.0) as f32;
        terrain.temperature - (f + y as f32 - 64.0) * 0.05 / 30.0
    } else {
        terrain.temperature
    }
}

/// `Biome.method_8590`: the plain column body.
#[allow(clippy::too_many_arguments)]
fn plain_column(
    random: &mut LegacyRandom,
    terrain: BiomeTerrain18,
    top: u16,
    filler: u16,
    i: i32,
    j: i32,
    d: f64,
    sea_level: i32,
    storage: &mut [u16],
) {
    // `n`/`o` are the transposed column coordinates this body writes.
    let z = (i & 0xF) as usize;
    let x = (j & 0xF) as usize;
    let mut block_state = Some(top);
    let mut base_state = filler;
    let mut depth: i32 = -1;
    let m = (d / 3.0 + 3.0 + random.next_double() * 0.25) as i32;
    for p in (0..=255i32).rev() {
        let index = x << 12 | z << 8 | p as usize;
        if p <= random.next_int_bound(5).expect("positive bound") {
            storage[index] = BEDROCK;
            continue;
        }
        let current = storage[index];
        if is_air(current) {
            depth = -1;
            continue;
        }
        if current != STONE {
            continue;
        }
        if depth == -1 {
            if m <= 0 {
                block_state = None;
                base_state = STONE;
            } else if p >= sea_level - 4 && p <= sea_level + 1 {
                block_state = Some(top);
                base_state = filler;
            }
            if p < sea_level && block_state.is_none() {
                block_state = Some(if biome_temperature(terrain, i, p, j) < 0.15 {
                    ICE
                } else {
                    WATER
                });
            }
            depth = m;
            if p >= sea_level - 1 {
                storage[index] = block_state.unwrap_or(AIR);
                continue;
            }
            if p < sea_level - 7 - m {
                block_state = None;
                base_state = STONE;
                storage[index] = GRAVEL;
                continue;
            }
            storage[index] = base_state;
            continue;
        }
        if depth <= 0 {
            continue;
        }
        storage[index] = base_state;
        depth -= 1;
        if depth == 0 && base_state >> 4 == SAND >> 4 {
            depth = random.next_int_bound(4).expect("positive bound") + (p - 63).max(0);
            base_state = if base_state & 0xF == 1 {
                RED_SANDSTONE
            } else {
                SANDSTONE
            };
        }
    }
}

/// `SwampBiome.method_6420`: drain the y = 62 surface and grow a lily pad
/// where the foliage noise is strong, then run the plain body.
#[allow(clippy::too_many_arguments)]
fn swamp_column(
    random: &mut LegacyRandom,
    terrain: BiomeTerrain18,
    top: u16,
    filler: u16,
    i: i32,
    j: i32,
    d: f64,
    sea_level: i32,
    storage: &mut [u16],
) {
    let e = foliage_noise().sample(f64::from(i) * 0.25, f64::from(j) * 0.25);
    if e > 0.0 {
        let z = (i & 0xF) as usize;
        let x = (j & 0xF) as usize;
        for m in (0..=255i32).rev() {
            let index = x << 12 | z << 8 | m as usize;
            if is_air(storage[index]) {
                continue;
            }
            if m != 62 || storage[index] == WATER {
                break;
            }
            storage[index] = WATER;
            if e < 0.12 {
                // 1.8 sets `m + 1`, and this branch only runs at m = 62.
                storage[index + 1] = LILY_PAD;
            }
            break;
        }
    }
    plain_column(random, terrain, top, filler, i, j, d, sea_level, storage);
}

/// The cached per-`MesaBiome`-instance state: the seed-keyed layer palette and
/// the two plateau noises.
///
/// 1.8.9 has a bug the port keeps: `initLayerBlocks` runs before `this.seed`
/// is assigned, so the height noises are built from `Random(this.seed)` (0 on
/// the first world, the previous world's seed afterwards) while the layer
/// palette and layer noise use the real world seed.
#[derive(Default)]
struct MesaState18 {
    seed: i64,
    layer_blocks: Option<[u16; 64]>,
    layer_noise: Option<PerlinNoise18>,
    height_cutoff: Option<PerlinNoise18>,
    height_noise: Option<PerlinNoise18>,
}

impl MesaState18 {
    fn ensure(&mut self, world_seed: i64) {
        if self.layer_blocks.is_none() || self.seed != world_seed {
            self.init_layer_blocks(world_seed);
        }
        if self.height_cutoff.is_none() || self.height_noise.is_none() || self.seed != world_seed {
            let mut random = LegacyRandom::new(self.seed);
            self.height_cutoff = Some(PerlinNoise18::new(&mut random, 4));
            self.height_noise = Some(PerlinNoise18::new(&mut random, 1));
        }
        self.seed = world_seed;
    }

    /// `MesaBiome.initLayerBlocks`: the 64-entry hardened-clay palette.
    fn init_layer_blocks(&mut self, seed: i64) {
        let mut blocks = [TERRACOTTA; 64];
        let mut random = LegacyRandom::new(seed);
        self.layer_noise = Some(PerlinNoise18::new(&mut random, 1));
        let mut i: i32 = 0;
        while i < 64 {
            i += random.next_int_bound(5).expect("positive bound") + 1;
            if i < 64 {
                blocks[i as usize] = STAINED_ORANGE;
            }
            i += 1;
        }
        let count = random.next_int_bound(4).expect("positive bound") + 2;
        for _ in 0..count {
            let len = random.next_int_bound(3).expect("positive bound") + 1;
            let start = random.next_int_bound(64).expect("positive bound");
            for m in 0..len {
                if start + m < 64 {
                    blocks[(start + m) as usize] = STAINED_YELLOW;
                }
            }
        }
        let count = random.next_int_bound(4).expect("positive bound") + 2;
        for _ in 0..count {
            let len = random.next_int_bound(3).expect("positive bound") + 2;
            let start = random.next_int_bound(64).expect("positive bound");
            for m in 0..len {
                if start + m < 64 {
                    blocks[(start + m) as usize] = STAINED_BROWN;
                }
            }
        }
        let count = random.next_int_bound(4).expect("positive bound") + 2;
        for _ in 0..count {
            let len = random.next_int_bound(3).expect("positive bound") + 1;
            let start = random.next_int_bound(64).expect("positive bound");
            for m in 0..len {
                if start + m < 64 {
                    blocks[(start + m) as usize] = STAINED_RED;
                }
            }
        }
        let count = random.next_int_bound(3).expect("positive bound") + 3;
        let mut at: i32 = 0;
        for _ in 0..count {
            at += random.next_int_bound(16).expect("positive bound") + 4;
            if at >= 64 {
                continue;
            }
            blocks[at as usize] = STAINED_WHITE;
            if at > 1 && random.next_int_bound(2).expect("positive bound") == 1 {
                blocks[(at - 1) as usize] = STAINED_SILVER;
            }
            if at < 63 && random.next_int_bound(2).expect("positive bound") == 1 {
                blocks[(at + 1) as usize] = STAINED_SILVER;
            }
        }
        self.layer_blocks = Some(blocks);
    }

    /// `MesaBiome.calculateLayerBlockState`: the layer noise is read at the
    /// transposed `x` coordinate twice; `z` is never used.
    fn calculate_layer_block_state(&self, x: i32, y: i32) -> u16 {
        let noise = self.layer_noise.as_ref().expect("layer noise");
        let at = f64::from(x) / 512.0;
        let offset = (noise.sample(at, at) * 2.0 + 0.5).floor() as i32;
        let blocks = self.layer_blocks.as_ref().expect("layer blocks");
        blocks[(y + offset + 64).rem_euclid(64) as usize]
    }

    /// `MesaBiome.method_6420`.
    #[allow(clippy::too_many_arguments)]
    fn column(
        &mut self,
        world_seed: i64,
        random: &mut LegacyRandom,
        top: u16,
        filler: u16,
        variant: u8,
        i: i32,
        j: i32,
        d: f64,
        sea_level: i32,
        storage: &mut [u16],
    ) {
        self.ensure(world_seed);
        let plateau = variant & 1 != 0;
        let bryce = variant & 2 != 0;
        let mut plateau_height = 0.0;
        if plateau {
            let k = (i & !0xF) + (j & 0xF);
            let l = (j & !0xF) + (i & 0xF);
            let cutoff = self.height_cutoff.as_ref().expect("height cutoff");
            let f = d
                .abs()
                .min(cutoff.sample(f64::from(k) * 0.25, f64::from(l) * 0.25));
            if f > 0.0 {
                plateau_height = f * f * 2.5;
                let g = 0.001953125;
                let height = self.height_noise.as_ref().expect("height noise");
                let h = height.sample(f64::from(k) * g, f64::from(l) * g).abs();
                let ceiling = (h * 50.0).ceil() + 14.0;
                if plateau_height > ceiling {
                    plateau_height = ceiling;
                }
                plateau_height += 64.0;
            }
        }
        let z = (i & 0xF) as usize;
        let x = (j & 0xF) as usize;
        let mut top_state = Some(STAINED_WHITE);
        let mut base_state = filler;
        let depth = (d / 3.0 + 3.0 + random.next_double() * 0.25) as i32;
        let positive = (d / 3.0 * std::f64::consts::PI).cos() > 0.0;
        let mut remaining: i32 = -1;
        let mut clay_top = false;
        for q in (0..=255i32).rev() {
            let index = x << 12 | z << 8 | q as usize;
            if is_air(storage[index]) && q < plateau_height as i32 {
                storage[index] = STONE;
            }
            if q <= random.next_int_bound(5).expect("positive bound") {
                storage[index] = BEDROCK;
                continue;
            }
            let current = storage[index];
            if is_air(current) {
                remaining = -1;
                continue;
            }
            if current != STONE {
                continue;
            }
            if remaining == -1 {
                clay_top = false;
                if depth <= 0 {
                    top_state = None;
                    base_state = STONE;
                } else if q >= sea_level - 4 && q <= sea_level + 1 {
                    top_state = Some(STAINED_WHITE);
                    base_state = filler;
                }
                if q < sea_level && top_state.is_none() {
                    top_state = Some(WATER);
                }
                remaining = depth + (q - sea_level).max(0);
                if q >= sea_level - 1 {
                    if bryce && q > 86 + depth * 2 {
                        storage[index] = if positive { COARSE_DIRT } else { GRASS };
                        continue;
                    }
                    if q > sea_level + 3 + depth {
                        storage[index] = if q < 64 || q > 127 {
                            STAINED_ORANGE
                        } else if positive {
                            TERRACOTTA
                        } else {
                            self.calculate_layer_block_state(i, q)
                        };
                        continue;
                    }
                    storage[index] = top;
                    clay_top = true;
                    continue;
                }
                storage[index] = base_state;
                if base_state >> 4 == STAINED_TERRACOTTA >> 4 {
                    storage[index] = STAINED_ORANGE;
                }
                continue;
            }
            if remaining <= 0 {
                continue;
            }
            remaining -= 1;
            if clay_top {
                storage[index] = STAINED_ORANGE;
                continue;
            }
            storage[index] = self.calculate_layer_block_state(i, q);
        }
    }
}

fn is_air(state: u16) -> bool {
    state >> 4 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::terrain_1_8_9::{SEA_LEVEL, TerrainDensity18, fill_blocks};

    struct Case {
        seed: i64,
        kind: String,
        chunk_x: i32,
        chunk_z: i32,
        biomes: Vec<u8>,
        density_biomes: Vec<u8>,
        noise: Vec<f64>,
        columns: Vec<Option<Vec<(usize, u16)>>>,
    }

    fn oracle() -> Vec<Case> {
        let text = include_str!("../../tests/fixtures/surface_1_8_9.txt");
        let mut cases: Vec<Case> = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("case=") {
                let fields: Vec<&str> = rest.split(':').collect();
                assert_eq!(fields.len(), 4, "case fields");
                cases.push(Case {
                    seed: fields[0].parse().expect("seed"),
                    kind: fields[1].to_owned(),
                    chunk_x: fields[2].parse().expect("chunk x"),
                    chunk_z: fields[3].parse().expect("chunk z"),
                    biomes: Vec::new(),
                    density_biomes: Vec::new(),
                    noise: Vec::new(),
                    columns: (0..256).map(|_| None).collect(),
                });
            } else if let Some(rest) = line.strip_prefix("biomes=") {
                cases.last_mut().expect("case first").biomes = rest
                    .split(',')
                    .map(|id| id.parse().expect("biome id"))
                    .collect();
            } else if let Some(rest) = line.strip_prefix("density_biomes=") {
                cases.last_mut().expect("case first").density_biomes = rest
                    .split(',')
                    .map(|id| id.parse().expect("density biome id"))
                    .collect();
            } else if let Some(rest) = line.strip_prefix("noise=") {
                cases.last_mut().expect("case first").noise = rest
                    .split(',')
                    .map(|bits| f64::from_bits(bits.parse::<i64>().expect("noise bits") as u64))
                    .collect();
            } else if let Some(rest) = line.strip_prefix("column=") {
                let (position, runs) = rest.split_once(';').expect("column position");
                let (x, z) = position.split_once(',').expect("column x,z");
                let x: usize = x.parse().expect("column x");
                let z: usize = z.parse().expect("column z");
                let runs = runs
                    .split(',')
                    .map(|run| {
                        let (start, id) = run.split_once(':').expect("run start:id");
                        (
                            start.parse().expect("run start"),
                            id.parse().expect("run state"),
                        )
                    })
                    .collect();
                cases.last_mut().expect("case first").columns[x + z * 16] = Some(runs);
            }
        }
        cases
    }

    fn runs(storage: &[u16], x: usize, z: usize) -> Vec<(usize, u16)> {
        let mut runs = Vec::new();
        let mut start = 0;
        let mut previous = storage[x << 12 | z << 8];
        for y in 1..256 {
            let now = storage[x << 12 | z << 8 | y];
            if now != previous {
                runs.push((start, previous));
                start = y;
                previous = now;
            }
        }
        runs.push((start, previous));
        runs
    }

    /// `method_6546` and every biome `method_6420` body against the 1.8.9 jar:
    /// the surface-noise grid by bits, then every column's block-state runs.
    #[test]
    fn surface_matches_the_one_eight_nine_jar() {
        let cases = oracle();
        assert_eq!(cases.len(), 9, "fixture cases");
        for case in &cases {
            assert_eq!(case.biomes.len(), 256, "{} biomes", case.kind);
            assert_eq!(
                case.density_biomes.len(),
                100,
                "{} density biomes",
                case.kind
            );
            assert_eq!(case.noise.len(), 256, "{} noise", case.kind);
            assert_eq!(
                case.columns.iter().filter(|c| c.is_some()).count(),
                256,
                "{} columns",
                case.kind
            );
            let mut generator = TerrainDensity18::new(case.seed);
            let density = generator
                .fill(
                    &case.density_biomes,
                    case.chunk_x,
                    case.chunk_z,
                    case.kind == "amplified",
                )
                .to_vec();
            let mut storage = vec![0u16; STORAGE_LEN];
            fill_from_density(&fill_blocks(&density, SEA_LEVEL), &mut storage);
            let mut surface = SurfaceState18::new();
            let noises = generator.noises();
            surface.apply(
                noises,
                case.seed,
                &case.biomes,
                case.chunk_x,
                case.chunk_z,
                SEA_LEVEL,
                &mut storage,
            );
            for (index, (&got, &want)) in surface.noise().iter().zip(&case.noise).enumerate() {
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "{} seed {} surface noise {index}",
                    case.kind,
                    case.seed
                );
            }
            for z in 0..16 {
                for x in 0..16 {
                    let want = case.columns[x + z * 16].as_ref().expect("column");
                    let got = runs(&storage, x, z);
                    assert_eq!(
                        got, *want,
                        "{} seed {} column {x},{z}",
                        case.kind, case.seed
                    );
                }
            }
        }
    }
}
