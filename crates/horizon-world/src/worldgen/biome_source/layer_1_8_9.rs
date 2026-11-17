//! The 1.8.9 overworld's biome source: `WorldChunkManager` over the layer
//! chain.
//!
//! 1.8 answers a block's biome from `BiomeCache.Block.biomes`, the 16x16
//! array `WorldChunkManager.getBiomesForGeneration(null, chunkX << 4,
//! chunkZ << 4, 16, 16)` returns. [`BiomeLayers18::chunk_biomes`] is that
//! array. Horizon stores one biome per 4x4x4 quart, so a quart takes the
//! biome of its first block and every query inside the quart rounds there.
//!
//! The chain is stateful per query (`GenLayer.method_143` seeds each cell
//! from the world seed), so the source keeps one chain behind a lock and
//! caches the chunk it last answered, as `BiomeCache` does. Neighbouring
//! quart samples stay in one chunk, so the cache hit rate is high.

use crate::biome_layer_1_8_9::BiomeLayers18;
use std::sync::Mutex;

/// The overworld biome source of a 1.8.9 world.
pub struct LayerBiomeSource18 {
    cache: Mutex<Cache>,
}
struct Cache {
    layers: BiomeLayers18,
    /// The last chunk asked about: `BiomeCache.Block`'s `biomes` array in
    /// vanilla, which also holds one chunk at a time.
    last: Option<((i32, i32), Box<[u8; 256]>)>,
}
impl LayerBiomeSource18 {
    pub fn new(seed: i64) -> Self {
        Self::from_layers(BiomeLayers18::new(seed))
    }
    /// `WorldType.LARGE_BIOMES`: the same chain with `biomeSize = 6`.
    pub fn large_biomes(seed: i64) -> Self {
        Self::from_layers(BiomeLayers18::large_biomes(seed))
    }
    fn from_layers(layers: BiomeLayers18) -> Self {
        Self {
            cache: Mutex::new(Cache { layers, last: None }),
        }
    }
    /// The two arrays the 1.8 terrain pass reads for chunk `(chunk_x,
    /// chunk_z)`: `method_3857`'s 10x10 river-mixed density neighbourhood
    /// (x-major) and `getBiomesForGeneration`'s 16x16 Voronoi array. The
    /// Voronoi array is also the chunk `BiomeCache` holds afterwards.
    pub fn terrain_biomes(&self, chunk_x: i32, chunk_z: i32) -> ([u8; 100], [u8; 256]) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let ids = cache.layers.river_mixed(
            chunk_x.wrapping_mul(4).wrapping_sub(2),
            chunk_z.wrapping_mul(4).wrapping_sub(2),
            10,
            10,
        );
        let mut river = [0u8; 100];
        for (biome, id) in river.iter_mut().zip(ids) {
            debug_assert!((0..=255).contains(&id));
            *biome = id as u8;
        }
        let biomes = cache.layers.chunk_biomes(chunk_x, chunk_z);
        cache.last = Some(((chunk_x, chunk_z), Box::new(biomes)));
        (river, biomes)
    }

    /// The biome of the block a quart starts at (`quart << 2`).
    pub fn biome_at_quart(&self, quart: [i32; 3]) -> u8 {
        self.biome_at_block(quart[0].wrapping_shl(2), quart[2].wrapping_shl(2))
    }
    /// `WorldChunkManager.getBiomeGenAt` for one block.
    pub fn biome_at_block(&self, x: i32, z: i32) -> u8 {
        let chunk = (x >> 4, z >> 4);
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if cache.last.as_ref().map(|(at, _)| *at) != Some(chunk) {
            let biomes = cache.layers.chunk_biomes(chunk.0, chunk.1);
            cache.last = Some((chunk, Box::new(biomes)));
        }
        cache.last.as_ref().expect("the chunk was just filled").1
            [((x & 15) + ((z & 15) << 4)) as usize]
    }
}
impl std::fmt::Debug for LayerBiomeSource18 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LayerBiomeSource18")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every quart takes the biome of its first block in the Voronoi layer's
    /// 16x16 array, for every cell of a chunk and its neighbours.
    fn source_matches_chunk_biomes(
        make_source: impl Fn(i64) -> LayerBiomeSource18,
        make_layers: impl Fn(i64) -> BiomeLayers18,
    ) {
        for seed in [0, 12345, -987_654_321] {
            let source = make_source(seed);
            let mut layers = make_layers(seed);
            for (chunk_x, chunk_z) in [(0, 0), (-1, 3), (32, -48)] {
                let expected = layers.chunk_biomes(chunk_x, chunk_z);
                for qx in 0..4 {
                    for qz in 0..4 {
                        let quart = [chunk_x * 4 + qx, 0, chunk_z * 4 + qz];
                        let at = source.biome_at_quart(quart);
                        assert_eq!(
                            at,
                            expected[(qx * 4 + (qz * 4) * 16) as usize],
                            "seed {seed} chunk ({chunk_x}, {chunk_z}) quart {quart:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_quart_reads_the_first_block_of_its_voronoi_cell() {
        source_matches_chunk_biomes(LayerBiomeSource18::new, BiomeLayers18::new);
    }

    /// `WorldType.LARGE_BIOMES` is the same source over the `biomeSize = 6`
    /// chain, not the default one.
    #[test]
    fn large_biomes_is_the_size_six_chain() {
        source_matches_chunk_biomes(
            LayerBiomeSource18::large_biomes,
            BiomeLayers18::large_biomes,
        );
        let default = LayerBiomeSource18::new(424242);
        let large = LayerBiomeSource18::large_biomes(424242);
        // The first difference for this seed is chunk (0, 7).
        let mut differs = false;
        'search: for cx in 0..8 {
            for cz in 0..8 {
                for qx in 0..4 {
                    for qz in 0..4 {
                        let quart = [cx * 4 + qx, 0, cz * 4 + qz];
                        if default.biome_at_quart(quart) != large.biome_at_quart(quart) {
                            differs = true;
                            break 'search;
                        }
                    }
                }
            }
        }
        assert!(differs, "large biomes must use the wider chain");
    }

    /// Consecutive samples inside one chunk reuse the cached array, and a
    /// neighbouring chunk is rebuilt for its own cells.
    #[test]
    fn the_cache_follows_the_chunk_it_last_answered() {
        let source = LayerBiomeSource18::new(7);
        assert_eq!(
            source.biome_at_block(0, 0),
            source.biome_at_quart([0, 0, 0])
        );
        assert_eq!(
            source.biome_at_block(15, 15),
            source.biome_at_quart([3, 0, 3])
        );
        let mut layers = BiomeLayers18::new(7);
        let east = layers.chunk_biomes(1, 0);
        assert_eq!(source.biome_at_block(16, 1), east[16]);
    }
}
