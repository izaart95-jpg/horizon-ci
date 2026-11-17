//! Minecraft 1.8.9's terrain density lattice.
//!
//! `SurfaceChunkGenerator.method_6544` samples four noise fields into the
//! 825-double lattice `field_7516` at 4-block steps, blending each biome's
//! `depth` and `variationModifier` over the surrounding 5x5 biome
//! neighbourhood, and `method_9194` interpolates that lattice down to one
//! block. The result is only `stone`, `water` below sea level 63 and `air`;
//! the biome's `genTerrainBlocks` pass replaces the top of every column
//! afterwards.
//!
//! Both steps are ported operation for operation: the noise scales are f32
//! divisions widened to f64, the biome blend is f32, and the height/limit
//! math is f64, because 1.8.9's world is only reproducible if every rounding
//! matches. `tests/fixtures/terrain_density_1_8_9.txt` pins all 825 doubles
//! and the 256 stone/water/air columns for four chunks from the 1.8.9 jar.

use super::noise_1_8_9::TerrainNoise18;
use crate::biome_terrain_1_8_9::{self, PLAINS};

/// 1.8's `seaLevel`: the height `method_9194` fills with water.
pub const SEA_LEVEL: i32 = 63;

/// `field_7516`'s length: the 5 x 5 x 33 density lattice.
pub const DENSITY_LEN: usize = 5 * 5 * 33;

/// One chunk's block count, `16 x 16 x 256`.
pub const CHUNK_VOLUME: usize = 16 * 16 * 256;

/// `CustomizedWorldProperties.Builder`'s defaults, the values
/// `SurfaceChunkGenerator` reads for the overworld.
const COORDINATE_SCALE: f32 = 684.412;
const HEIGHT_SCALE: f32 = 684.412;
const UPPER_LIMIT_SCALE: f32 = 512.0;
const LOWER_LIMIT_SCALE: f32 = 512.0;
const DEPTH_NOISE_SCALE_X: f32 = 200.0;
const DEPTH_NOISE_SCALE_Z: f32 = 200.0;
const MAIN_NOISE_SCALE_X: f32 = 80.0;
const MAIN_NOISE_SCALE_Y: f32 = 160.0;
const MAIN_NOISE_SCALE_Z: f32 = 80.0;
const BASE_SIZE: f32 = 8.5;
const STRETCH_Y: f32 = 12.0;

/// One block of the pre-surface 1.8.9 column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DensityBlock18 {
    Air,
    Stone,
    Water,
}

/// The noise and scratch buffers one `SurfaceChunkGenerator` owns.
///
/// The four noises are built from the seed in `SurfaceChunkGenerator`'s
/// constructor order, so the fields cannot be built separately: their
/// permutation tables share one `Random`.
pub struct TerrainDensity18 {
    noises: TerrainNoise18,
    /// `field_7517`: `10 / sqrt(dx^2 + dz^2 + 0.2)` over the 5x5 blend.
    weights: [f32; 25],
    /// `field_4828`: the 5x5 depth noise `method_121` fills.
    depth_noise: Vec<f64>,
    /// `field_7512`: the lower-limit noise, `field_4832`.
    lower_limit: Vec<f64>,
    /// `field_7513`: the upper-limit noise, `field_4833`.
    upper_limit: Vec<f64>,
    /// `field_7511`: the main noise, `field_4834`.
    main: Vec<f64>,
    /// `field_7516`: the density lattice.
    density: Vec<f64>,
}

impl TerrainDensity18 {
    pub fn new(seed: i64) -> Self {
        let mut weights = [0.0; 25];
        for i in -2i32..=2 {
            for j in -2i32..=2 {
                // `MathHelper.sqrt(float)` widens to f64, roots, and narrows.
                let squared = (i * i + j * j) as f32 + 0.2;
                weights[(i + 2 + (j + 2) * 5) as usize] = 10.0 / f64::from(squared).sqrt() as f32;
            }
        }
        Self {
            noises: TerrainNoise18::new(seed),
            weights,
            depth_noise: Vec::with_capacity(25),
            lower_limit: Vec::with_capacity(DENSITY_LEN),
            upper_limit: Vec::with_capacity(DENSITY_LEN),
            main: Vec::with_capacity(DENSITY_LEN),
            density: vec![0.0; DENSITY_LEN],
        }
    }

    pub fn noises(&self) -> &TerrainNoise18 {
        &self.noises
    }

    /// `method_6544`: fill the density lattice for the chunk at
    /// `(chunk_x, chunk_z)` from its 10x10 biome neighbourhood (x-major, as
    /// `LayeredBiomeSource.method_3857` returns it).
    ///
    /// `amplified` is `LevelGeneratorType.AMPLIFIED`, which doubles positive
    /// biome depth and quadruples its variation.
    pub fn fill(&mut self, biomes: &[u8], chunk_x: i32, chunk_z: i32, amplified: bool) -> &[f64] {
        assert_eq!(biomes.len(), 100, "10x10 biome neighbourhood");
        let at_x = chunk_x * 4;
        let at_z = chunk_z * 4;
        self.depth_noise = self.noises.scale.sample_2d(
            at_x,
            at_z,
            5,
            5,
            f64::from(DEPTH_NOISE_SCALE_X),
            f64::from(DEPTH_NOISE_SCALE_Z),
        );
        // `f / mainNoiseScaleX` is a float division widened to double.
        self.main = self.noises.main.sample_3d(
            at_x,
            0,
            at_z,
            5,
            33,
            5,
            f64::from(COORDINATE_SCALE / MAIN_NOISE_SCALE_X),
            f64::from(HEIGHT_SCALE / MAIN_NOISE_SCALE_Y),
            f64::from(COORDINATE_SCALE / MAIN_NOISE_SCALE_Z),
        );
        self.lower_limit = self.noises.lower_limit.sample_3d(
            at_x,
            0,
            at_z,
            5,
            33,
            5,
            f64::from(COORDINATE_SCALE),
            f64::from(HEIGHT_SCALE),
            f64::from(COORDINATE_SCALE),
        );
        self.upper_limit = self.noises.upper_limit.sample_3d(
            at_x,
            0,
            at_z,
            5,
            33,
            5,
            f64::from(COORDINATE_SCALE),
            f64::from(HEIGHT_SCALE),
            f64::from(COORDINATE_SCALE),
        );
        let mut depth_index = 0;
        let mut index = 0;
        for x in 0..5usize {
            for z in 0..5usize {
                let center = terrain_of(biomes[x + 2 + (z + 2) * 10]);
                let mut variation_sum = 0.0f32;
                let mut depth_sum = 0.0f32;
                let mut weight_sum = 0.0f32;
                for s in -2i32..=2 {
                    for t in -2i32..=2 {
                        let neighbour = terrain_of(
                            biomes[(x as i32 + s + 2 + (z as i32 + t + 2) * 10) as usize],
                        );
                        let mut depth = neighbour.depth;
                        let mut variation = neighbour.variation;
                        if amplified && depth > 0.0 {
                            depth = 1.0 + depth * 2.0;
                            variation = 1.0 + variation * 4.0;
                        }
                        let mut weight =
                            self.weights[(s + 2 + (t + 2) * 5) as usize] / (depth + 2.0);
                        if neighbour.depth > center.depth {
                            weight /= 2.0;
                        }
                        variation_sum += variation * weight;
                        depth_sum += depth * weight;
                        weight_sum += weight;
                    }
                }
                let mut variation = variation_sum / weight_sum;
                let mut depth = depth_sum / weight_sum;
                variation = variation * 0.9 + 0.1;
                depth = (depth * 4.0 - 1.0) / 8.0;

                let mut d = self.depth_noise[depth_index] / 8000.0;
                depth_index += 1;
                if d < 0.0 {
                    d = -d * 0.3;
                }
                d = d * 3.0 - 2.0;
                if d < 0.0 {
                    d /= 2.0;
                    if d < -1.0 {
                        d = -1.0;
                    }
                    d /= 1.4;
                    d /= 2.0;
                } else {
                    if d > 1.0 {
                        d = 1.0;
                    }
                    d /= 8.0;
                }
                let mut height = f64::from(depth);
                let base = f64::from(variation);
                height += d * 0.2;
                height = height * f64::from(BASE_SIZE) / 8.0;
                let base_height = f64::from(BASE_SIZE) + height * 4.0;
                for y in 0..33 {
                    let mut stretch =
                        (y as f64 - base_height) * f64::from(STRETCH_Y) * 128.0 / 256.0 / base;
                    if stretch < 0.0 {
                        stretch *= 4.0;
                    }
                    let lower = self.lower_limit[index] / f64::from(LOWER_LIMIT_SCALE);
                    let upper = self.upper_limit[index] / f64::from(UPPER_LIMIT_SCALE);
                    let delta = (self.main[index] / 10.0 + 1.0) / 2.0;
                    let mut density = clamped_lerp(lower, upper, delta) - stretch;
                    if y > 29 {
                        let tail = ((y as i32 - 29) as f32 / 3.0) as f64;
                        density = density * (1.0 - tail) + -10.0 * tail;
                    }
                    self.density[index] = density;
                    index += 1;
                }
            }
        }
        &self.density
    }
}

/// `MathHelper.clampedLerp`.
fn clamped_lerp(start: f64, end: f64, delta: f64) -> f64 {
    if delta < 0.0 {
        start
    } else if delta > 1.0 {
        end
    } else {
        start + (end - start) * delta
    }
}

/// The 1.8.9 terrain fields of a biome id, with the plains row for an id
/// 1.8.9 never registered (the layer chain only produces registered ids).
fn terrain_of(id: u8) -> biome_terrain_1_8_9::BiomeTerrain18 {
    biome_terrain_1_8_9::terrain(id).unwrap_or(PLAINS)
}

/// `method_9194`'s interpolation: one block per lattice cell, with stone
/// where the interpolated density is above zero, water below sea level, and
/// air otherwise.
///
/// The result is indexed `x + z * 16 + y * 256`.
pub fn fill_blocks(density: &[f64], sea_level: i32) -> Vec<DensityBlock18> {
    assert_eq!(density.len(), DENSITY_LEN, "density lattice");
    let mut blocks = vec![DensityBlock18::Air; CHUNK_VOLUME];
    for k in 0..4usize {
        let l = k * 5;
        let m = (k + 1) * 5;
        for n in 0..4usize {
            let o = (l + n) * 33;
            let p = (l + n + 1) * 33;
            let q = (m + n) * 33;
            let r = (m + n + 1) * 33;
            for s in 0..32usize {
                let d = 0.125;
                let mut e = density[o + s];
                let mut f = density[p + s];
                let mut g = density[q + s];
                let mut h = density[r + s];
                let t = (density[o + s + 1] - e) * d;
                let u = (density[p + s + 1] - f) * d;
                let v = (density[q + s + 1] - g) * d;
                let w = (density[r + s + 1] - h) * d;
                for x in 0..8usize {
                    let y_step = 0.25;
                    let mut along_x = e;
                    let mut along_x1 = f;
                    let step_x = (g - e) * y_step;
                    let step_x1 = (h - f) * y_step;
                    for ad in 0..4usize {
                        // The `-step; += step` pair is vanilla's own: it makes
                        // the first sample exactly the cell value.
                        let mut sample = along_x;
                        let step_z = (along_x1 - along_x) * 0.25;
                        sample -= step_z;
                        for ah in 0..4usize {
                            sample += step_z;
                            let block_y = s * 8 + x;
                            blocks[block_y * 256 + (n * 4 + ah) * 16 + k * 4 + ad] = if sample > 0.0
                            {
                                DensityBlock18::Stone
                            } else if block_y as i32 >= sea_level {
                                DensityBlock18::Air
                            } else {
                                DensityBlock18::Water
                            };
                        }
                        along_x += step_x;
                        along_x1 += step_x1;
                    }
                    e += t;
                    f += u;
                    g += v;
                    h += w;
                }
            }
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case {
        seed: i64,
        kind: String,
        chunk_x: i32,
        chunk_z: i32,
        biomes: Vec<u8>,
        density: Vec<u64>,
        columns: Vec<Column>,
    }

    struct Column {
        x: usize,
        z: usize,
        runs: Vec<(usize, String)>,
    }

    fn oracle() -> Vec<Case> {
        let text = include_str!("../../tests/fixtures/terrain_density_1_8_9.txt");
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
                    density: Vec::new(),
                    columns: Vec::new(),
                });
            } else if let Some(rest) = line.strip_prefix("biomes=") {
                cases.last_mut().expect("case first").biomes = rest
                    .split(',')
                    .map(|id| id.parse().expect("biome id"))
                    .collect();
            } else if let Some(rest) = line.strip_prefix("density=") {
                cases.last_mut().expect("case first").density = rest
                    .split(',')
                    .map(|bits| bits.parse::<i64>().expect("density bits") as u64)
                    .collect();
            } else if let Some(rest) = line.strip_prefix("column=") {
                let (position, runs) = rest.split_once(';').expect("column position");
                let (x, z) = position.split_once(',').expect("column x,z");
                let case = cases.last_mut().expect("case first");
                case.columns.push(Column {
                    x: x.parse().expect("column x"),
                    z: z.parse().expect("column z"),
                    runs: runs
                        .split(',')
                        .map(|run| {
                            let (start, kind) = run.split_once(':').expect("run start:kind");
                            (start.parse().expect("run start"), kind.to_owned())
                        })
                        .collect(),
                });
            }
        }
        cases
    }

    fn kind_name(block: DensityBlock18) -> &'static str {
        match block {
            DensityBlock18::Air => "air",
            DensityBlock18::Stone => "stone",
            DensityBlock18::Water => "water",
        }
    }

    /// `method_6544` and `method_9194` against the 1.8.9 jar: the 825 lattice
    /// doubles bit for bit, then the 256 columns of stone/water/air runs.
    #[test]
    fn terrain_density_matches_the_one_eight_nine_jar() {
        let cases = oracle();
        assert_eq!(cases.len(), 4, "fixture cases");
        for case in &cases {
            assert_eq!(case.biomes.len(), 100, "{} biomes", case.kind);
            assert_eq!(case.density.len(), DENSITY_LEN, "{} density", case.kind);
            assert_eq!(case.columns.len(), 256, "{} columns", case.kind);
            let mut generator = TerrainDensity18::new(case.seed);
            let density = generator.fill(
                &case.biomes,
                case.chunk_x,
                case.chunk_z,
                case.kind == "amplified",
            );
            for (index, (&got, &want)) in density.iter().zip(&case.density).enumerate() {
                assert_eq!(
                    got.to_bits(),
                    want,
                    "{} seed {} density {index}: {got} != {}",
                    case.kind,
                    case.seed,
                    f64::from_bits(want)
                );
            }
            let blocks = fill_blocks(density, SEA_LEVEL);
            for column in &case.columns {
                let mut runs = Vec::new();
                let mut start = 0;
                let mut previous = blocks[column.x + column.z * 16];
                for y in 1..256 {
                    let now = blocks[column.x + column.z * 16 + y * 256];
                    if now != previous {
                        runs.push((start, kind_name(previous).to_owned()));
                        start = y;
                        previous = now;
                    }
                }
                runs.push((start, kind_name(previous).to_owned()));
                assert_eq!(
                    runs, column.runs,
                    "{} seed {} column {},{}",
                    case.kind, case.seed, column.x, column.z
                );
            }
        }
    }
}
