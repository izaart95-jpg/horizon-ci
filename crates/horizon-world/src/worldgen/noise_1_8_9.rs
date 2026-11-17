//! Minecraft 1.8.9's terrain noise (`NoiseGenerator` and
//! `PerlinNoiseGenerator`).
//!
//! `SurfaceChunkGenerator` builds seven generators from one `Random(seed)`:
//! four octave noises, the improved-Perlin surface noise and two more octave
//! noises that only the carvers and features use. Their construction order is
//! part of the seed: every generator draws its own permutation from the same
//! stream, so [`TerrainNoise18`] must build all seven even where terrain only
//! samples some of them.
//!
//! The octave noise is the classic `NoiseGeneratorOctaves` pair (Yarn
//! `NoiseGenerator` + `class_1779`): per-octave gradients on a 16-value table,
//! trilinear `fade` interpolation and the 1/amplitude accumulation. The
//! surface noise is `NoiseGeneratorImproved` (Yarn `NoiseSampler`): the
//! simplex-style grid with a 12-entry gradient table, wrapped by
//! `PerlinNoiseGenerator`'s octave sum.
//!
//! Java's `double` arithmetic is IEEE 754, so every operation here is the same
//! `f64` operation in the same order; [`tests`] pins the results bit for bit
//! against the 1.8.9 jar.

use horizon_core::random::{LegacyRandom, RandomSource};

/// The terrain noise of one 1.8.9 world seed, as `SurfaceChunkGenerator`
/// constructs it.
pub struct TerrainNoise18 {
    /// `field_4832`: the lower-limit noise (`lowerLimitScale`).
    pub lower_limit: OctaveNoise18,
    /// `field_4833`: the upper-limit noise (`upperLimitScale`).
    pub upper_limit: OctaveNoise18,
    /// `field_4834`: the main density noise.
    pub main: OctaveNoise18,
    /// `field_7514`: the surface noise `Biome.genTerrainBlocks` reads.
    pub surface: PerlinNoise18,
    /// `field_4821`: the depth noise the carvers use.
    pub depth: OctaveNoise18,
    /// `field_4822`: the 5x5 depth offset `method_6544` blends.
    pub scale: OctaveNoise18,
    /// `field_4823`: built for the seed but unused by the overworld terrain.
    pub unused: OctaveNoise18,
}

impl TerrainNoise18 {
    pub fn new(seed: i64) -> Self {
        let mut random = LegacyRandom::new(seed);
        Self {
            lower_limit: OctaveNoise18::new(&mut random, 16),
            upper_limit: OctaveNoise18::new(&mut random, 16),
            main: OctaveNoise18::new(&mut random, 8),
            surface: PerlinNoise18::new(&mut random, 4),
            depth: OctaveNoise18::new(&mut random, 10),
            scale: OctaveNoise18::new(&mut random, 16),
            unused: OctaveNoise18::new(&mut random, 8),
        }
    }
}

/// `NoiseGenerator`: `octaves` Perlin octaves summed with halving amplitude.
pub struct OctaveNoise18 {
    octaves: Vec<PerlinOctave18>,
}

impl OctaveNoise18 {
    pub fn new(random: &mut LegacyRandom, octaves: usize) -> Self {
        Self {
            octaves: (0..octaves).map(|_| PerlinOctave18::new(random)).collect(),
        }
    }

    /// `NoiseGenerator.method_122`: a `width x height x depth` lattice starting
    /// at `(x, y, z)`, laid out x-major, then z, then y fastest.
    #[allow(clippy::too_many_arguments)]
    pub fn sample_3d(
        &self,
        x: i32,
        y: i32,
        z: i32,
        width: usize,
        height: usize,
        depth: usize,
        scale_x: f64,
        scale_y: f64,
        scale_z: f64,
    ) -> Vec<f64> {
        let mut values = vec![0.0; width * height * depth];
        let mut amplitude = 1.0;
        for octave in &self.octaves {
            let mut at_x = f64::from(x) * amplitude * scale_x;
            let at_y = f64::from(y) * amplitude * scale_y;
            let mut at_z = f64::from(z) * amplitude * scale_z;
            // `MathHelper.lfloor`, then the fractional part plus the floored
            // cell reduced modulo 2^24 (Java's signed `%`).
            let floor_x = at_x.floor() as i64;
            let floor_z = at_z.floor() as i64;
            at_x -= floor_x as f64;
            at_z -= floor_z as f64;
            octave.fill(
                &mut values,
                at_x + (floor_x % 0x100_0000) as f64,
                at_y,
                at_z + (floor_z % 0x100_0000) as f64,
                width,
                height,
                depth,
                scale_x * amplitude,
                scale_y * amplitude,
                scale_z * amplitude,
                amplitude,
            );
            amplitude /= 2.0;
        }
        values
    }

    /// `NoiseGenerator.method_121`: a `width x depth` slice with `y = 0`.
    pub fn sample_2d(
        &self,
        x: i32,
        z: i32,
        width: usize,
        depth: usize,
        scale_x: f64,
        scale_z: f64,
    ) -> Vec<f64> {
        self.sample_3d(x, 10, z, width, 1, depth, scale_x, 1.0, scale_z)
    }
}

/// `class_1779`: one octave of the classic Perlin noise.
struct PerlinOctave18 {
    permutations: [i32; 512],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
}

impl PerlinOctave18 {
    fn new(random: &mut LegacyRandom) -> Self {
        let offset_x = random.next_double() * 256.0;
        let offset_y = random.next_double() * 256.0;
        let offset_z = random.next_double() * 256.0;
        let mut permutations = [0; 512];
        for (i, value) in permutations[..256].iter_mut().enumerate() {
            *value = i as i32;
        }
        for i in 0..256 {
            let bound = 256 - i as i32;
            let j = random.next_int_bound(bound).expect("positive bound") as usize + i;
            permutations.swap(i, j);
            permutations[i + 256] = permutations[i];
        }
        Self {
            permutations,
            offset_x,
            offset_y,
            offset_z,
        }
    }

    /// `method_6577`: adds one octave's values into `values`, divided by
    /// `divisor` (the octave amplitude).
    #[allow(clippy::too_many_arguments, unused_assignments)]
    fn fill(
        &self,
        values: &mut [f64],
        at_x: f64,
        at_y: f64,
        at_z: f64,
        width: usize,
        height: usize,
        depth: usize,
        step_x: f64,
        step_y: f64,
        step_z: f64,
        divisor: f64,
    ) {
        if height == 1 {
            self.fill_2d(values, at_x, at_z, width, depth, step_x, step_z, divisor);
            return;
        }
        let inverse = 1.0 / divisor;
        let mut index = 0;
        // The `method_6544` 3D branch caches the z/y corner values for one
        // y-cell; the game almost never hits it with these steps, but vanilla
        // only recomputes when the integer y cell changes.
        let mut cached_y_cell = -1;
        let (mut corner_x0, mut corner_x1) = (0, 0);
        let (mut corner_z0, mut corner_z1) = (0, 0);
        let (mut low, mut high_y0, mut high_y1, mut high) = (0.0, 0.0, 0.0, 0.0);
        for ix in 0..width {
            let mut x = at_x + ix as f64 * step_x + self.offset_x;
            let mut cell_x = x as i32;
            if x < f64::from(cell_x) {
                cell_x -= 1;
            }
            let x_cell = (cell_x & 0xFF) as usize;
            x -= f64::from(cell_x);
            let fade_x = x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
            for iz in 0..depth {
                let mut z = at_z + iz as f64 * step_z + self.offset_z;
                let mut cell_z = z as i32;
                if z < f64::from(cell_z) {
                    cell_z -= 1;
                }
                let z_cell = (cell_z & 0xFF) as usize;
                z -= f64::from(cell_z);
                let fade_z = z * z * z * (z * (z * 6.0 - 15.0) + 10.0);
                for iy in 0..height {
                    let mut y = at_y + iy as f64 * step_y + self.offset_y;
                    let mut cell_y = y as i32;
                    if y < f64::from(cell_y) {
                        cell_y -= 1;
                    }
                    let y_cell = (cell_y & 0xFF) as i32;
                    y -= f64::from(cell_y);
                    let fade_y = y * y * y * (y * (y * 6.0 - 15.0) + 10.0);
                    if iy == 0 || y_cell != cached_y_cell {
                        cached_y_cell = y_cell;
                        let base_x = self.permutations[x_cell] + y_cell;
                        corner_x0 = self.permutations[base_x as usize] + z_cell as i32;
                        corner_x1 = self.permutations[base_x as usize + 1] + z_cell as i32;
                        let base_z = self.permutations[x_cell + 1] + y_cell;
                        corner_z0 = self.permutations[base_z as usize] + z_cell as i32;
                        corner_z1 = self.permutations[base_z as usize + 1] + z_cell as i32;
                        low = lerp(
                            fade_x,
                            gradient3(self.permutations[corner_x0 as usize], x, y, z),
                            gradient3(self.permutations[corner_z0 as usize], x - 1.0, y, z),
                        );
                        high_y0 = lerp(
                            fade_x,
                            gradient3(self.permutations[corner_x1 as usize], x, y - 1.0, z),
                            gradient3(self.permutations[corner_z1 as usize], x - 1.0, y - 1.0, z),
                        );
                        high_y1 = lerp(
                            fade_x,
                            gradient3(self.permutations[corner_x0 as usize + 1], x, y, z - 1.0),
                            gradient3(
                                self.permutations[corner_z0 as usize + 1],
                                x - 1.0,
                                y,
                                z - 1.0,
                            ),
                        );
                        high = lerp(
                            fade_x,
                            gradient3(
                                self.permutations[corner_x1 as usize + 1],
                                x,
                                y - 1.0,
                                z - 1.0,
                            ),
                            gradient3(
                                self.permutations[corner_z1 as usize + 1],
                                x - 1.0,
                                y - 1.0,
                                z - 1.0,
                            ),
                        );
                    }
                    let near = lerp(fade_y, low, high_y0);
                    let far = lerp(fade_y, high_y1, high);
                    values[index] += lerp(fade_z, near, far) * inverse;
                    index += 1;
                }
            }
        }
    }

    /// The `height == 1` branch of `method_6577`.
    #[allow(clippy::too_many_arguments)]
    fn fill_2d(
        &self,
        values: &mut [f64],
        at_x: f64,
        at_z: f64,
        width: usize,
        depth: usize,
        step_x: f64,
        step_z: f64,
        divisor: f64,
    ) {
        let inverse = 1.0 / divisor;
        let mut index = 0;
        for ix in 0..width {
            let mut x = at_x + ix as f64 * step_x + self.offset_x;
            let mut cell_x = x as i32;
            if x < f64::from(cell_x) {
                cell_x -= 1;
            }
            let x_cell = (cell_x & 0xFF) as usize;
            x -= f64::from(cell_x);
            let fade_x = x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
            for iz in 0..depth {
                let mut z = at_z + iz as f64 * step_z + self.offset_z;
                let mut cell_z = z as i32;
                if z < f64::from(cell_z) {
                    cell_z -= 1;
                }
                let z_cell = (cell_z & 0xFF) as usize;
                z -= f64::from(cell_z);
                let fade_z = z * z * z * (z * (z * 6.0 - 15.0) + 10.0);
                let base_x = self.permutations[x_cell];
                let low = self.permutations[base_x as usize] as usize + z_cell;
                let high =
                    self.permutations[self.permutations[x_cell + 1] as usize] as usize + z_cell;
                let near = lerp(
                    fade_x,
                    gradient2(self.permutations[low], x, z),
                    gradient3(self.permutations[high], x - 1.0, 0.0, z),
                );
                let far = lerp(
                    fade_x,
                    gradient3(self.permutations[low + 1], x, 0.0, z - 1.0),
                    gradient3(self.permutations[high + 1], x - 1.0, 0.0, z - 1.0),
                );
                values[index] += lerp(fade_z, near, far) * inverse;
                index += 1;
            }
        }
    }
}

/// `PerlinNoiseGenerator`: `samplers` improved-Perlin octaves.
pub struct PerlinNoise18 {
    samplers: Vec<ImprovedNoise18>,
}

impl PerlinNoise18 {
    pub fn new(random: &mut LegacyRandom, samplers: usize) -> Self {
        Self {
            samplers: (0..samplers)
                .map(|_| ImprovedNoise18::new(random))
                .collect(),
        }
    }

    /// `method_6580`: a `width x depth` grid whose step shrinks by half per
    /// octave and whose amplitude grows by `0.55 / m`.
    #[allow(clippy::too_many_arguments)]
    pub fn sample_2d(
        &self,
        x: f64,
        z: f64,
        width: usize,
        depth: usize,
        step_x: f64,
        step_z: f64,
        frequency: f64,
    ) -> Vec<f64> {
        let mut values = vec![0.0; width * depth];
        let mut frequency_scale = 1.0;
        let mut amplitude_scale = 1.0;
        for sampler in &self.samplers {
            sampler.fill_grid(
                &mut values,
                x,
                z,
                width,
                depth,
                step_x * frequency_scale * amplitude_scale,
                step_z * frequency_scale * amplitude_scale,
                0.55 / amplitude_scale,
            );
            frequency_scale *= frequency;
            amplitude_scale *= 0.5;
        }
        values
    }

    /// `NoiseGeneratorImproved.sample` / `PerlinNoiseGenerator.noise` for one
    /// point.
    pub fn sample(&self, x: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut frequency = 1.0;
        for sampler in &self.samplers {
            value += sampler.sample(x * frequency, z * frequency) / frequency;
            frequency /= 2.0;
        }
        value
    }
}

/// `NoiseSampler`: the improved Perlin noise behind the surface noise.
struct ImprovedNoise18 {
    permutations: [i32; 512],
    offset_x: f64,
    offset_y: f64,
    /// Drawn by the constructor, so it changes the permutation shuffle, but
    /// 1.8's `NoiseSampler` only has 2D entry points and never adds it.
    #[allow(dead_code)]
    offset_z: f64,
}

/// `NoiseSampler.field_7566`: `0.5 * (SQRT3 - 1)`.
fn skew() -> f64 {
    0.5 * (3.0_f64.sqrt() - 1.0)
}

/// `NoiseSampler.field_7567`: `(3 - SQRT3) / 6`.
fn unskew() -> f64 {
    (3.0 - 3.0_f64.sqrt()) / 6.0
}

impl ImprovedNoise18 {
    fn new(random: &mut LegacyRandom) -> Self {
        let offset_x = random.next_double() * 256.0;
        let offset_y = random.next_double() * 256.0;
        let offset_z = random.next_double() * 256.0;
        let mut permutations = [0; 512];
        for (i, value) in permutations[..256].iter_mut().enumerate() {
            *value = i as i32;
        }
        for i in 0..256 {
            let bound = 256 - i as i32;
            let j = random.next_int_bound(bound).expect("positive bound") as usize + i;
            permutations.swap(i, j);
            permutations[i + 256] = permutations[i];
        }
        Self {
            permutations,
            offset_x,
            offset_y,
            offset_z,
        }
    }

    /// `NoiseSampler.sample(x, y)`: one 2D point.
    fn sample(&self, x: f64, z: f64) -> f64 {
        let (skew, unskew) = (skew(), unskew());
        let skew_offset = (x + z) * skew;
        let cell_x = floor(x + skew_offset);
        let cell_z = floor(z + skew_offset);
        let origin_x = f64::from(cell_x) - f64::from(cell_x + cell_z) * unskew;
        let local_x = x - origin_x;
        let local_z = z - (f64::from(cell_z) - f64::from(cell_x + cell_z) * unskew);
        let (step_x, step_z) = if local_x > local_z { (1, 0) } else { (0, 1) };
        let mid_x = local_x - step_x as f64 + unskew;
        let mid_z = local_z - step_z as f64 + unskew;
        let far_x = local_x - 1.0 + 2.0 * unskew;
        let far_z = local_z - 1.0 + 2.0 * unskew;
        let x_cell = (cell_x & 0xFF) as usize;
        let z_cell = (cell_z & 0xFF) as usize;
        let corner = self.permutations[x_cell + self.permutations[z_cell] as usize] % 12;
        let middle =
            self.permutations[x_cell + step_x + self.permutations[z_cell + step_z] as usize] % 12;
        let opposite = self.permutations[x_cell + 1 + self.permutations[z_cell + 1] as usize] % 12;
        let mut near = 0.5 - local_x * local_x - local_z * local_z;
        let near = if near < 0.0 {
            0.0
        } else {
            near *= near;
            near * near * improved_gradient(corner, local_x, local_z)
        };
        let mut middle_value = 0.5 - mid_x * mid_x - mid_z * mid_z;
        let middle_value = if middle_value < 0.0 {
            0.0
        } else {
            middle_value *= middle_value;
            middle_value * middle_value * improved_gradient(middle, mid_x, mid_z)
        };
        let mut far = 0.5 - far_x * far_x - far_z * far_z;
        let far = if far < 0.0 {
            0.0
        } else {
            far *= far;
            far * far * improved_gradient(opposite, far_x, far_z)
        };
        70.0 * (near + middle_value + far)
    }

    /// `NoiseSampler.method_6584`: adds a `width x depth` grid into `values`.
    #[allow(clippy::too_many_arguments)]
    fn fill_grid(
        &self,
        values: &mut [f64],
        at_x: f64,
        at_z: f64,
        width: usize,
        depth: usize,
        step_x: f64,
        step_z: f64,
        amplitude: f64,
    ) {
        let (skew, unskew) = (skew(), unskew());
        let mut index = 0;
        for iz in 0..depth {
            // `NoiseSampler` is 2D: its second coordinate reads the y offset
            // (`field_7562`), not the unused `field_7563`.
            let z = (at_z + iz as f64) * step_z + self.offset_y;
            for ix in 0..width {
                let x = (at_x + ix as f64) * step_x + self.offset_x;
                let skew_offset = (x + z) * skew;
                let cell_x = floor(x + skew_offset);
                let cell_z = floor(z + skew_offset);
                let origin_x = f64::from(cell_x) - f64::from(cell_x + cell_z) * unskew;
                let local_x = x - origin_x;
                let local_z = z - (f64::from(cell_z) - f64::from(cell_x + cell_z) * unskew);
                let (step_x, step_z) = if local_x > local_z { (1, 0) } else { (0, 1) };
                let mid_x = local_x - step_x as f64 + unskew;
                let mid_z = local_z - step_z as f64 + unskew;
                let far_x = local_x - 1.0 + 2.0 * unskew;
                let far_z = local_z - 1.0 + 2.0 * unskew;
                let x_cell = (cell_x & 0xFF) as usize;
                let z_cell = (cell_z & 0xFF) as usize;
                let corner = self.permutations[x_cell + self.permutations[z_cell] as usize] % 12;
                let middle = self.permutations
                    [x_cell + step_x + self.permutations[z_cell + step_z] as usize]
                    % 12;
                let opposite =
                    self.permutations[x_cell + 1 + self.permutations[z_cell + 1] as usize] % 12;
                let mut near = 0.5 - local_x * local_x - local_z * local_z;
                let near = if near < 0.0 {
                    0.0
                } else {
                    near *= near;
                    near * near * improved_gradient(corner, local_x, local_z)
                };
                let mut middle_value = 0.5 - mid_x * mid_x - mid_z * mid_z;
                let middle_value = if middle_value < 0.0 {
                    0.0
                } else {
                    middle_value *= middle_value;
                    middle_value * middle_value * improved_gradient(middle, mid_x, mid_z)
                };
                let mut far = 0.5 - far_x * far_x - far_z * far_z;
                let far = if far < 0.0 {
                    0.0
                } else {
                    far *= far;
                    far * far * improved_gradient(opposite, far_x, far_z)
                };
                values[index] += 70.0 * (near + middle_value + far) * amplitude;
                index += 1;
            }
        }
    }
}

/// `MathHelper.lfloor`: floor as an `i32` (Java's `(int)` cast saturates).
fn floor(value: f64) -> i32 {
    if value > 0.0 {
        value as i32
    } else {
        (value as i32).wrapping_sub(1)
    }
}

/// `class_1779.method_6578` and the improved noise's lerp.
fn lerp(delta: f64, start: f64, end: f64) -> f64 {
    start + delta * (end - start)
}

/// The two-dimensional gradient of `class_1779.method_6575`
/// (`field_7555`/`field_7556`), also the improved noise's 12 gradients
/// evaluated at `y = 0`.
fn gradient2(index: i32, x: f64, z: f64) -> f64 {
    const X: [f64; 16] = [
        1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
    ];
    const Z: [f64; 16] = [
        0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0, 0.0, -1.0,
    ];
    let i = (index & 0xF) as usize;
    X[i] * x + Z[i] * z
}

/// `NoiseSampler.field_7564` evaluated at `y = 0`: the improved noise's
/// twelve gradients (`method_6585` reads the first two components).
fn improved_gradient(index: i32, x: f64, z: f64) -> f64 {
    const GRADIENTS: [[f64; 3]; 12] = [
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0],
        [-1.0, -1.0, 0.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
        [1.0, 0.0, -1.0],
        [-1.0, 0.0, -1.0],
        [0.0, 1.0, 1.0],
        [0.0, -1.0, 1.0],
        [0.0, 1.0, -1.0],
        [0.0, -1.0, -1.0],
    ];
    let gradient = GRADIENTS[(index % 12) as usize];
    gradient[0] * x + gradient[1] * z
}

/// The three-dimensional gradient of `class_1779.method_6576`.
fn gradient3(index: i32, x: f64, y: f64, z: f64) -> f64 {
    const X: [f64; 16] = [
        1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
    ];
    const Y: [f64; 16] = [
        1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0,
    ];
    const Z: [f64; 16] = [
        0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0, 0.0, -1.0,
    ];
    let i = (index & 0xF) as usize;
    X[i] * x + Y[i] * y + Z[i] * z
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case {
        noise: String,
        kind: String,
        seed: i64,
        x: i32,
        y: i32,
        z: i32,
        width: usize,
        height: usize,
        depth: usize,
        scale_x: f64,
        scale_y: f64,
        scale_z: f64,
        frequency: f64,
        bits: Vec<i64>,
    }

    fn oracle() -> Vec<Case> {
        include_str!("../../tests/fixtures/terrain_noise_1_8_9.txt")
            .lines()
            .filter(|line| line.starts_with("noise="))
            .map(|line| {
                let mut case = Case {
                    noise: String::new(),
                    kind: String::new(),
                    seed: 0,
                    x: 0,
                    y: 0,
                    z: 0,
                    width: 0,
                    height: 0,
                    depth: 0,
                    scale_x: 0.0,
                    scale_y: 0.0,
                    scale_z: 0.0,
                    frequency: 1.0,
                    bits: Vec::new(),
                };
                for field in line.split(';') {
                    let (key, value) = field.split_once('=').expect("key=value");
                    match key {
                        "noise" => case.noise = value.to_owned(),
                        "kind" => case.kind = value.to_owned(),
                        "seed" => case.seed = value.parse().expect("seed"),
                        "x" => case.x = value.parse().expect("x"),
                        "y" => case.y = value.parse().expect("y"),
                        "z" => case.z = value.parse().expect("z"),
                        "w" => case.width = value.parse().expect("w"),
                        "h" => case.height = value.parse().expect("h"),
                        "d" => case.depth = value.parse().expect("d"),
                        "sx" => case.scale_x = value.parse().expect("sx"),
                        "sy" => case.scale_y = value.parse().expect("sy"),
                        "sz" => case.scale_z = value.parse().expect("sz"),
                        "f" => case.frequency = value.parse().expect("f"),
                        // `method_121` takes the depth-noise exponent but
                        // `NoiseGenerator` ignores it in 1.8.9.
                        "e" => {}
                        "bits" => {
                            case.bits = value
                                .split(',')
                                .map(|bits| bits.parse().expect("bits"))
                                .collect()
                        }
                        other => panic!("unexpected field {other}"),
                    }
                }
                case
            })
            .collect()
    }

    fn compare(case: &Case, values: &[f64]) {
        assert_eq!(
            values.len(),
            case.bits.len(),
            "{} seed {} length",
            case.noise,
            case.seed
        );
        for (index, (value, &want)) in values.iter().zip(&case.bits).enumerate() {
            let got = value.to_bits() as i64;
            assert_eq!(
                got,
                want,
                "{} {} seed {} value {index}: {} != {}",
                case.noise,
                case.kind,
                case.seed,
                value,
                f64::from_bits(want as u64)
            );
        }
    }

    /// Every value of the 1.8.9 jar's terrain noise, bit for bit: the four
    /// octave noises, the improved-Perlin surface noise, and the depth noise
    /// `method_121` samples as a 2D slice at y = 10.
    #[test]
    fn terrain_noise_matches_the_one_eight_nine_jar() {
        let cases = oracle();
        assert_eq!(cases.len(), 20, "fixture cases");
        for case in &cases {
            let noises = TerrainNoise18::new(case.seed);
            let values = match case.noise.as_str() {
                "main" => noises.main.sample_3d(
                    case.x,
                    case.y,
                    case.z,
                    case.width,
                    case.height,
                    case.depth,
                    case.scale_x,
                    case.scale_y,
                    case.scale_z,
                ),
                "min_limit" => noises.lower_limit.sample_3d(
                    case.x,
                    case.y,
                    case.z,
                    case.width,
                    case.height,
                    case.depth,
                    case.scale_x,
                    case.scale_y,
                    case.scale_z,
                ),
                "max_limit" => noises.upper_limit.sample_3d(
                    case.x,
                    case.y,
                    case.z,
                    case.width,
                    case.height,
                    case.depth,
                    case.scale_x,
                    case.scale_y,
                    case.scale_z,
                ),
                // The 2D handlers emit their second extent as `h`, the
                // value `method_121` and `method_6580` receive as depth.
                "depth" => noises.depth.sample_2d(
                    case.x,
                    case.z,
                    case.width,
                    case.height,
                    case.scale_x,
                    case.scale_z,
                ),
                "scale" => noises.scale.sample_2d(
                    case.x,
                    case.z,
                    case.width,
                    case.height,
                    case.scale_x,
                    case.scale_z,
                ),
                "surface" => noises.surface.sample_2d(
                    f64::from(case.x),
                    f64::from(case.z),
                    case.width,
                    case.height,
                    case.scale_x,
                    case.scale_z,
                    case.frequency,
                ),
                other => panic!("unknown noise {other}"),
            };
            compare(case, &values);
        }
    }
}
