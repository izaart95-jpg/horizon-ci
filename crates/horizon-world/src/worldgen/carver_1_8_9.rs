//! Minecraft 1.8.9's cave and ravine carvers.
//!
//! `SurfaceChunkGenerator.getChunk` carves after the density fill and the
//! biome surface pass, caves first and ravines second. Both carvers walk a
//! 17x17 chunk neighbourhood: `Carver.carveRegion` draws two longs from
//! `Random(world seed)`, then reseeds the carver's own `Random` per neighbour
//! with `(j * l) ^ (k * m) ^ world seed` and simulates that neighbour's
//! caves. Every write is clamped to the main chunk's storage, so a cave that
//! starts in a neighbour still reaches into this chunk; the whole
//! neighbourhood has to run for every chunk.
//!
//! The simulated cave is a chain of ellipsoids: `carveCave` walks a branch
//! from the start point, widening and turning with the `Random(seed)` of that
//! one cave, and recurses into two side branches partway along. Ravines are a
//! single chain with a per-height horizontal stretch factor, and only one
//! chunk in fifty rolls one.
//!
//! Both carvers reproduce 1.8's block rules exactly: stone, dirt (any
//! metadata), grass, hardened and stained clay, both sandstones, mycelium and
//! snow layers are carved; sand and gravel only when the block above is not
//! water. Below y = 11 the cave leaves still lava, the ravine flowing lava.
//! A carved grass or mycelium block with dirt below turns that dirt into the
//! biome's `topBlock` — the block's default state for caves, the full 1.8
//! state for ravines.
//!
//! `tests/fixtures/carver_1_8_9.txt` pins chunks from the 1.8.9 jar after
//! both carvers, including a mesa, a swamp and chunks cut by ravines.

use horizon_core::{
    geometry::trig::{cos_1_8_9, sin_1_8_9},
    random::{LegacyRandom, RandomSource},
};

use crate::biome_terrain_1_8_9::{self, PLAINS};

/// One chunk's block states, `id << 4 | meta`, indexed
/// `x << 12 | z << 8 | y` as `ChunkBlockStateStorage` indexes them.
const STORAGE_LEN: usize = 16 * 16 * 256;

/// `Carver.complexity`: the chunk radius `carveRegion` simulates.
const COMPLEXITY: i32 = 8;

const AIR: u16 = 0;
const STONE: u16 = 1 << 4;
const GRASS: u16 = 2 << 4;
const DIRT: u16 = 3 << 4;
const FLOWING_LAVA: u16 = 10 << 4;
const LAVA: u16 = 11 << 4;
const SANDSTONE: u16 = 24 << 4;
const SNOW_LAYER: u16 = 78 << 4;
const MYCELIUM: u16 = 110 << 4;
const STAINED_TERRACOTTA: u16 = 159 << 4;
const TERRACOTTA: u16 = 172 << 4;
const RED_SANDSTONE: u16 = 179 << 4;

fn at(x: i32, y: i32, z: i32) -> usize {
    ((x as usize) << 12) | ((z as usize) << 8) | (y as usize)
}

fn get(storage: &[u16], x: i32, y: i32, z: i32) -> u16 {
    storage[at(x, y, z)]
}

fn set(storage: &mut [u16], x: i32, y: i32, z: i32, state: u16) {
    storage[at(x, y, z)] = state;
}

/// `MathHelper.floor(double)`: `(int)Math.floor(value)`.
fn floor(value: f64) -> i32 {
    value.floor() as i32
}

fn block(state: u16) -> u16 {
    state >> 4
}

fn is_water(state: u16) -> bool {
    matches!(block(state), 8 | 9)
}

/// `CaveCarver.canCarveBlock`. `above` is the block at `y + 1`.
fn can_carve_cave(state: u16, above: u16) -> bool {
    match block(state) {
        1 | 2 | 3 => true,
        // `Blocks.DIRT` is one block for dirt, coarse dirt and podzol.
        24 | 78 | 110 | 159 | 172 | 179 => true,
        // Sand and gravel stay when they hold back water.
        12 | 13 => !is_water(above),
        _ => false,
    }
}

/// `RavineCarver` carves only stone, dirt and grass.
fn can_carve_ravine(state: u16) -> bool {
    matches!(block(state), 1 | 2 | 3)
}

/// `Biome.topBlock.getBlock().getDefaultState()` (the cave carver).
fn biome_top_default(biome: u8) -> u16 {
    let (id, _) = biome_terrain_1_8_9::terrain(biome).unwrap_or(PLAINS).top;
    id << 4
}

/// `Biome.topBlock` (the ravine carver keeps its metadata).
fn biome_top_state(biome: u8) -> u16 {
    let (id, meta) = biome_terrain_1_8_9::terrain(biome).unwrap_or(PLAINS).top;
    (id << 4) | u16::from(meta & 0xF)
}

/// The two 1.8.9 carvers, in `getChunk`'s order, with the `Random` state each
/// keeps between chunks.
pub struct Carver18 {
    caves: CaveCarver18,
    ravines: RavineCarver18,
}

impl Default for Carver18 {
    fn default() -> Self {
        Self::new()
    }
}

impl Carver18 {
    pub fn new() -> Self {
        Self {
            caves: CaveCarver18 {
                random: LegacyRandom::new(0),
            },
            ravines: RavineCarver18 {
                random: LegacyRandom::new(0),
                height_stretch: [0.0; 256],
            },
        }
    }

    /// `CaveCarver.carveRegion` then `RavineCarver.carveRegion`.
    ///
    /// `biomes` is the chunk's 16x16 Voronoi array (`x + z * 16`): the carvers
    /// ask it for the biome `topBlock` of a carved grass column, and every
    /// such column lies in the main chunk.
    pub fn carve(
        &mut self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[u8; 256],
        storage: &mut [u16],
    ) {
        assert_eq!(storage.len(), STORAGE_LEN, "chunk storage length");
        self.caves
            .carve_region(world_seed, chunk_x, chunk_z, biomes, storage);
        self.ravines
            .carve_region(world_seed, chunk_x, chunk_z, biomes, storage);
    }
}

struct CaveCarver18 {
    random: LegacyRandom,
}

impl CaveCarver18 {
    /// `Carver.carveRegion`.
    fn carve_region(
        &mut self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[u8; 256],
        storage: &mut [u16],
    ) {
        self.random.set_seed(world_seed);
        let l = self.random.next_long();
        let m = self.random.next_long();
        let base_x = chunk_x.wrapping_sub(COMPLEXITY);
        let base_z = chunk_z.wrapping_sub(COMPLEXITY);
        for i in 0..=COMPLEXITY * 2 {
            let j = base_x.wrapping_add(i);
            for k in 0..=COMPLEXITY * 2 {
                let k = base_z.wrapping_add(k);
                let n = i64::from(j).wrapping_mul(l);
                let o = i64::from(k).wrapping_mul(m);
                self.random.set_seed(n ^ o ^ world_seed);
                self.carve(j, k, chunk_x, chunk_z, biomes, storage);
            }
        }
    }

    /// `CaveCarver.carve`: the per-neighbour cave count and entry points.
    #[allow(clippy::too_many_arguments)]
    fn carve(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        main_chunk_x: i32,
        main_chunk_z: i32,
        biomes: &[u8; 256],
        storage: &mut [u16],
    ) {
        let bound = self.random.next_int_bound(15).expect("positive bound") + 1;
        let bound = self.random.next_int_bound(bound).expect("positive bound") + 1;
        let mut count = self.random.next_int_bound(bound).expect("positive bound");
        if self.random.next_int_bound(7).expect("positive bound") != 0 {
            count = 0;
        }
        for _ in 0..count {
            let x = f64::from(
                chunk_x
                    .wrapping_mul(16)
                    .wrapping_add(self.random.next_int_bound(16).expect("positive bound")),
            );
            let bound = self.random.next_int_bound(120).expect("positive bound") + 8;
            let y = f64::from(self.random.next_int_bound(bound).expect("positive bound"));
            let z = f64::from(
                chunk_z
                    .wrapping_mul(16)
                    .wrapping_add(self.random.next_int_bound(16).expect("positive bound")),
            );
            let mut branches = 1;
            if self.random.next_int_bound(4).expect("positive bound") == 0 {
                let seed = self.random.next_long();
                self.carve_cave_default(biomes, storage, seed, main_chunk_x, main_chunk_z, x, y, z);
                branches += self.random.next_int_bound(4).expect("positive bound");
            }
            for _ in 0..branches {
                let xz_angle = self.random.next_float() * std::f32::consts::PI * 2.0;
                let y_angle = (self.random.next_float() - 0.5) * 2.0 / 8.0;
                let mut width = self.random.next_float() * 2.0 + self.random.next_float();
                if self.random.next_int_bound(10).expect("positive bound") == 0 {
                    width *= self.random.next_float() * self.random.next_float() * 3.0 + 1.0;
                }
                let seed = self.random.next_long();
                self.carve_cave(
                    biomes,
                    storage,
                    seed,
                    main_chunk_x,
                    main_chunk_z,
                    x,
                    y,
                    z,
                    width,
                    xz_angle,
                    y_angle,
                    0,
                    0,
                    1.0,
                );
            }
        }
    }

    /// The eight-argument `carveCave`: the entry point's width and a full
    /// branch range.
    #[allow(clippy::too_many_arguments)]
    fn carve_cave_default(
        &mut self,
        biomes: &[u8; 256],
        storage: &mut [u16],
        seed: i64,
        main_chunk_x: i32,
        main_chunk_z: i32,
        x: f64,
        y: f64,
        z: f64,
    ) {
        let width = 1.0 + self.random.next_float() * 6.0;
        self.carve_cave(
            biomes,
            storage,
            seed,
            main_chunk_x,
            main_chunk_z,
            x,
            y,
            z,
            width,
            0.0,
            0.0,
            -1,
            -1,
            0.5,
        );
    }

    /// `CaveCarver.carveCave`: walk one cave branch along its ellipsoids.
    #[allow(clippy::too_many_arguments)]
    fn carve_cave(
        &mut self,
        biomes: &[u8; 256],
        storage: &mut [u16],
        seed: i64,
        main_chunk_x: i32,
        main_chunk_z: i32,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        base_width: f32,
        mut xz_angle: f32,
        mut y_angle: f32,
        branch: i32,
        branch_count: i32,
        height_width_ratio: f64,
    ) {
        let d = f64::from(main_chunk_x.wrapping_mul(16).wrapping_add(8));
        let e = f64::from(main_chunk_z.wrapping_mul(16).wrapping_add(8));
        let mut f = 0.0f32;
        let mut g = 0.0f32;
        let mut random = LegacyRandom::new(seed);
        let mut branch = branch;
        let mut branch_count = branch_count;
        if branch_count <= 0 {
            let i = COMPLEXITY * 16 - 16;
            branch_count = i - random.next_int_bound(i / 4).expect("positive bound");
        }
        let mut start = false;
        if branch == -1 {
            branch = branch_count / 2;
            start = true;
        }
        let split = random
            .next_int_bound(branch_count / 2)
            .expect("positive bound")
            + branch_count / 4;
        let shorten = random.next_int_bound(6).expect("positive bound") == 0;
        while branch < branch_count {
            let h = 1.5
                + f64::from(
                    sin_1_8_9(branch as f32 * std::f32::consts::PI / branch_count as f32)
                        * base_width,
                );
            let k = h * height_width_ratio;
            let l = cos_1_8_9(y_angle);
            let m = sin_1_8_9(y_angle);
            x += f64::from(cos_1_8_9(xz_angle) * l);
            y += f64::from(m);
            z += f64::from(sin_1_8_9(xz_angle) * l);
            if shorten {
                y_angle *= 0.92;
            } else {
                y_angle *= 0.7;
            }
            y_angle += g * 0.1;
            xz_angle += f * 0.1;
            g *= 0.9;
            f *= 0.75;
            g += (random.next_float() - random.next_float()) * random.next_float() * 2.0;
            f += (random.next_float() - random.next_float()) * random.next_float() * 4.0;
            if !start && branch == split && base_width > 1.0 && branch_count > 0 {
                let seed = random.next_long();
                let width = random.next_float() * 0.5 + 0.5;
                self.carve_cave(
                    biomes,
                    storage,
                    seed,
                    main_chunk_x,
                    main_chunk_z,
                    x,
                    y,
                    z,
                    width,
                    xz_angle - 1.570_796_4,
                    y_angle / 3.0,
                    branch,
                    branch_count,
                    1.0,
                );
                let seed = random.next_long();
                let width = random.next_float() * 0.5 + 0.5;
                self.carve_cave(
                    biomes,
                    storage,
                    seed,
                    main_chunk_x,
                    main_chunk_z,
                    x,
                    y,
                    z,
                    width,
                    xz_angle + 1.570_796_4,
                    y_angle / 3.0,
                    branch,
                    branch_count,
                    1.0,
                );
                return;
            }
            if start || random.next_int_bound(4).expect("positive bound") != 0 {
                let n = x - d;
                let o = z - e;
                let p = f64::from(branch_count - branch);
                let q = f64::from(base_width) + 2.0 + 16.0;
                if n * n + o * o - p * p > q * q {
                    return;
                }
                if !(x < d - 16.0 - h * 2.0
                    || z < e - 16.0 - h * 2.0
                    || x > d + 16.0 + h * 2.0
                    || z > e + 16.0 + h * 2.0)
                {
                    let mut r = floor(x - h).wrapping_sub(main_chunk_x.wrapping_mul(16)) - 1;
                    let mut s = floor(x + h).wrapping_sub(main_chunk_x.wrapping_mul(16)) + 1;
                    let mut t = floor(y - k) - 1;
                    let mut u = floor(y + k) + 1;
                    let mut v = floor(z - h).wrapping_sub(main_chunk_z.wrapping_mul(16)) - 1;
                    let mut w = floor(z + h).wrapping_sub(main_chunk_z.wrapping_mul(16)) + 1;
                    if r < 0 {
                        r = 0;
                    }
                    if s > 16 {
                        s = 16;
                    }
                    if t < 1 {
                        t = 1;
                    }
                    if u > 248 {
                        u = 248;
                    }
                    if v < 0 {
                        v = 0;
                    }
                    if w > 16 {
                        w = 16;
                    }
                    let water = touches_water(storage, r, s, t, u, v, w);
                    if !water {
                        for ab in r..s {
                            let ad = (f64::from(ab.wrapping_add(main_chunk_x.wrapping_mul(16)))
                                + 0.5
                                - x)
                                / h;
                            for ae in v..w {
                                let af =
                                    (f64::from(ae.wrapping_add(main_chunk_z.wrapping_mul(16)))
                                        + 0.5
                                        - z)
                                        / h;
                                let mut grass = false;
                                if ad * ad + af * af >= 1.0 {
                                    continue;
                                }
                                let mut ag = u;
                                while ag > t {
                                    let ah = (f64::from(ag - 1) + 0.5 - y) / k;
                                    if ah > -0.7 && ad * ad + ah * ah + af * af < 1.0 {
                                        let state = get(storage, ab, ag, ae);
                                        let above = get(storage, ab, ag + 1, ae);
                                        if block(state) == 2 || block(state) == 110 {
                                            grass = true;
                                        }
                                        if can_carve_cave(state, above) {
                                            if ag - 1 < 10 {
                                                set(storage, ab, ag, ae, LAVA);
                                            } else {
                                                set(storage, ab, ag, ae, AIR);
                                                if block(above) == 12 {
                                                    set(
                                                        storage,
                                                        ab,
                                                        ag + 1,
                                                        ae,
                                                        if above & 0xF == 1 {
                                                            RED_SANDSTONE
                                                        } else {
                                                            SANDSTONE
                                                        },
                                                    );
                                                }
                                                if grass && block(get(storage, ab, ag - 1, ae)) == 3
                                                {
                                                    let biome = biomes[(ab + ae * 16) as usize];
                                                    set(
                                                        storage,
                                                        ab,
                                                        ag - 1,
                                                        ae,
                                                        biome_top_default(biome),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    ag -= 1;
                                }
                            }
                        }
                        if start {
                            break;
                        }
                    }
                }
            }
            branch += 1;
        }
    }
}

/// The carvers' water probe: scan the ellipsoid's bounding box for water.
///
/// 1.8 checks the top and bottom of each column and, on the box's x/z walls,
/// every Y in between; the middle of an interior column cannot hold water
/// that the walls would not.
fn touches_water(storage: &[u16], r: i32, s: i32, t: i32, u: i32, v: i32, w: i32) -> bool {
    for x in r..s {
        for z in v..w {
            let mut y = u + 1;
            while y >= t - 1 {
                if (0..256).contains(&y) {
                    if is_water(get(storage, x, y, z)) {
                        return true;
                    }
                    if !(y == t - 1 || x == r || x == s - 1 || z == v || z == w - 1) {
                        y = t;
                    }
                }
                y -= 1;
            }
        }
    }
    false
}

struct RavineCarver18 {
    random: LegacyRandom,
    /// `RavineCarver.heightToHorizontalStretchFactor`, rebuilt per ravine.
    height_stretch: [f32; 256],
}

impl RavineCarver18 {
    /// `Carver.carveRegion`.
    fn carve_region(
        &mut self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[u8; 256],
        storage: &mut [u16],
    ) {
        self.random.set_seed(world_seed);
        let l = self.random.next_long();
        let m = self.random.next_long();
        let base_x = chunk_x.wrapping_sub(COMPLEXITY);
        let base_z = chunk_z.wrapping_sub(COMPLEXITY);
        for i in 0..=COMPLEXITY * 2 {
            let j = base_x.wrapping_add(i);
            for k in 0..=COMPLEXITY * 2 {
                let k = base_z.wrapping_add(k);
                let n = i64::from(j).wrapping_mul(l);
                let o = i64::from(k).wrapping_mul(m);
                self.random.set_seed(n ^ o ^ world_seed);
                self.carve(j, k, chunk_x, chunk_z, biomes, storage);
            }
        }
    }

    /// `RavineCarver.carve`: one ravine in fifty chunks.
    #[allow(clippy::too_many_arguments)]
    fn carve(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        main_chunk_x: i32,
        main_chunk_z: i32,
        biomes: &[u8; 256],
        storage: &mut [u16],
    ) {
        if self.random.next_int_bound(50).expect("positive bound") != 0 {
            return;
        }
        let x = f64::from(
            chunk_x
                .wrapping_mul(16)
                .wrapping_add(self.random.next_int_bound(16).expect("positive bound")),
        );
        let bound = self.random.next_int_bound(40).expect("positive bound") + 8;
        let y = f64::from(self.random.next_int_bound(bound).expect("positive bound") + 20);
        let z = f64::from(
            chunk_z
                .wrapping_mul(16)
                .wrapping_add(self.random.next_int_bound(16).expect("positive bound")),
        );
        let xz_angle = self.random.next_float() * std::f32::consts::PI * 2.0;
        let y_angle = (self.random.next_float() - 0.5) * 2.0 / 8.0;
        let width = (self.random.next_float() * 2.0 + self.random.next_float()) * 2.0;
        let seed = self.random.next_long();
        self.carve_ravine(
            biomes,
            storage,
            seed,
            main_chunk_x,
            main_chunk_z,
            x,
            y,
            z,
            width,
            xz_angle,
            y_angle,
            0,
            0,
            3.0,
        );
    }

    /// `RavineCarver.carveRavine`: walk the ravine's single chain.
    #[allow(clippy::too_many_arguments)]
    fn carve_ravine(
        &mut self,
        biomes: &[u8; 256],
        storage: &mut [u16],
        seed: i64,
        main_chunk_x: i32,
        main_chunk_z: i32,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        base_width: f32,
        mut xz_angle: f32,
        mut y_angle: f32,
        branch: i32,
        branch_count: i32,
        height_width_ratio: f64,
    ) {
        let mut random = LegacyRandom::new(seed);
        let d = f64::from(main_chunk_x.wrapping_mul(16).wrapping_add(8));
        let e = f64::from(main_chunk_z.wrapping_mul(16).wrapping_add(8));
        let mut f = 0.0f32;
        let mut g = 0.0f32;
        let mut branch = branch;
        let mut branch_count = branch_count;
        if branch_count <= 0 {
            let i = COMPLEXITY * 16 - 16;
            branch_count = i - random.next_int_bound(i / 4).expect("positive bound");
        }
        let mut start = false;
        if branch == -1 {
            branch = branch_count / 2;
            start = true;
        }
        let mut h = 1.0f32;
        for j in 0..256usize {
            if j == 0 || random.next_int_bound(3).expect("positive bound") == 0 {
                h = 1.0 + random.next_float() * random.next_float() * 1.0;
            }
            self.height_stretch[j] = h * h;
        }
        while branch < branch_count {
            let mut k = 1.5
                + f64::from(
                    sin_1_8_9(branch as f32 * std::f32::consts::PI / branch_count as f32)
                        * base_width,
                );
            let mut l = k * height_width_ratio;
            k *= f64::from(random.next_float()) * 0.25 + 0.75;
            l *= f64::from(random.next_float()) * 0.25 + 0.75;
            let m = cos_1_8_9(y_angle);
            let n = sin_1_8_9(y_angle);
            x += f64::from(cos_1_8_9(xz_angle) * m);
            y += f64::from(n);
            z += f64::from(sin_1_8_9(xz_angle) * m);
            y_angle *= 0.7;
            y_angle += g * 0.05;
            xz_angle += f * 0.05;
            g *= 0.8;
            f *= 0.5;
            g += (random.next_float() - random.next_float()) * random.next_float() * 2.0;
            f += (random.next_float() - random.next_float()) * random.next_float() * 4.0;
            if start || random.next_int_bound(4).expect("positive bound") != 0 {
                let o = x - d;
                let p = z - e;
                let q = f64::from(branch_count - branch);
                let r = f64::from(base_width) + 2.0 + 16.0;
                if o * o + p * p - q * q > r * r {
                    return;
                }
                if !(x < d - 16.0 - k * 2.0
                    || z < e - 16.0 - k * 2.0
                    || x > d + 16.0 + k * 2.0
                    || z > e + 16.0 + k * 2.0)
                {
                    let mut s = floor(x - k).wrapping_sub(main_chunk_x.wrapping_mul(16)) - 1;
                    let mut t = floor(x + k).wrapping_sub(main_chunk_x.wrapping_mul(16)) + 1;
                    let mut u = floor(y - l) - 1;
                    let mut v = floor(y + l) + 1;
                    let mut w = floor(z - k).wrapping_sub(main_chunk_z.wrapping_mul(16)) - 1;
                    let mut aa = floor(z + k).wrapping_sub(main_chunk_z.wrapping_mul(16)) + 1;
                    if s < 0 {
                        s = 0;
                    }
                    if t > 16 {
                        t = 16;
                    }
                    if u < 1 {
                        u = 1;
                    }
                    if v > 248 {
                        v = 248;
                    }
                    if w < 0 {
                        w = 0;
                    }
                    if aa > 16 {
                        aa = 16;
                    }
                    let water = touches_water(storage, s, t, u, v, w, aa);
                    if !water {
                        for ac in s..t {
                            let ae = (f64::from(ac.wrapping_add(main_chunk_x.wrapping_mul(16)))
                                + 0.5
                                - x)
                                / k;
                            for af in w..aa {
                                let ag =
                                    (f64::from(af.wrapping_add(main_chunk_z.wrapping_mul(16)))
                                        + 0.5
                                        - z)
                                        / k;
                                let mut grass = false;
                                if ae * ae + ag * ag >= 1.0 {
                                    continue;
                                }
                                let mut ah = v;
                                while ah > u {
                                    let ai = (f64::from(ah - 1) + 0.5 - y) / l;
                                    if (ae * ae + ag * ag)
                                        * f64::from(self.height_stretch[(ah - 1) as usize])
                                        + ai * ai / 6.0
                                        < 1.0
                                    {
                                        let state = get(storage, ac, ah, af);
                                        if block(state) == 2 {
                                            grass = true;
                                        }
                                        if can_carve_ravine(state) {
                                            if ah - 1 < 10 {
                                                set(storage, ac, ah, af, FLOWING_LAVA);
                                            } else {
                                                set(storage, ac, ah, af, AIR);
                                                if grass && block(get(storage, ac, ah - 1, af)) == 3
                                                {
                                                    let biome = biomes[(ac + af * 16) as usize];
                                                    set(
                                                        storage,
                                                        ac,
                                                        ah - 1,
                                                        af,
                                                        biome_top_state(biome),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    ah -= 1;
                                }
                            }
                        }
                        if start {
                            break;
                        }
                    }
                }
            }
            branch += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::worldgen::surface_1_8_9::ChunkGenerator18;
    use crate::worldgen::terrain_1_8_9::SEA_LEVEL;

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
        let text = include_str!("../../tests/fixtures/carver_1_8_9.txt");
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

    /// The 1.8.9 terrain, surface pass, caves and ravines against the jar:
    /// every column's block-state runs.
    #[test]
    fn carvers_match_the_one_eight_nine_jar() {
        let cases = oracle();
        assert_eq!(cases.len(), 10, "fixture cases");
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
            let mut generator = ChunkGenerator18::new(case.seed, case.kind == "amplified");
            let storage = generator.generate(
                case.seed,
                &case.density_biomes,
                &case.biomes,
                case.chunk_x,
                case.chunk_z,
                SEA_LEVEL,
            );
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(
                        runs(&storage, x, z),
                        *case.columns[x + z * 16].as_ref().expect("column"),
                        "{} chunk ({}, {}) column ({x}, {z})",
                        case.kind,
                        case.chunk_x,
                        case.chunk_z
                    );
                }
            }
        }
    }
}
