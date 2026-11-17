//! Minecraft 1.8.9's natural spawning: `MobSpawnerHelper.tickSpawners`, its
//! `findSpawnLocation`, `isSpawnable` and the per-species `canSpawn`,
//! `method_2469` and `getLimitPerChunk` overrides, plus 1.8's light queries
//! (`World.method_8557` raw light, `method_8558` combined light with
//! neighbour-light blocks and ambient darkness, `recievesSunlight`).
//!
//! The game's spawn pass (`game/natural_spawning.rs`) drives the loop; this
//! module owns the rules so they can be read next to the decompiled source.
//! Everything not listed under the 1.8 feature list (generation packs for a
//! freshly generated chunk, `Biome.getMaxSpawnLimit`) stays on the 26.3 path.

use crate::blocks::{BlockFacts, Fluid};
use horizon_core::{
    geometry::Aabb,
    random::{LegacyRandom, RandomSource},
};
use horizon_world::{block_materials_1_8_9, registry::BlockStateId};

pub use horizon_world::biome_spawns_1_8_9::{Category, SpawnEntry18};

/// `EntityLocations`: where a species' natural spawn position may be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location18 {
    OnGround,
    InWater,
}

/// `EntityLocations.getLocation`: only the squid and the guardian spawn in
/// water; every other naturally spawning 1.8 mob is `ON_GROUND`.
pub fn location_of(entity: &str) -> Location18 {
    match entity {
        "minecraft:squid" | "minecraft:guardian" => Location18::InWater,
        _ => Location18::OnGround,
    }
}

/// `MobEntity.getLimitPerChunk`.
pub fn limit_per_chunk(entity: &str) -> i32 {
    match entity {
        "minecraft:wolf" => 8,
        "minecraft:horse" | "minecraft:donkey" | "minecraft:mule" => 6,
        "minecraft:ghast" => 1,
        _ => 4,
    }
}

/// What the spawn pass reads from the level.
pub trait SpawnLevel18 {
    fn state(&self, x: i32, y: i32, z: i32) -> BlockStateId;
    fn facts(&self) -> &BlockFacts;
    /// A block state property at a position (`BlockState.get`).
    fn prop(&self, x: i32, y: i32, z: i32, key: &str) -> Option<&str>;
    /// `ChunkSection.getSkyLight`: the stored sky light, before ambient
    /// darkness.
    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32;
    fn block_light(&self, x: i32, y: i32, z: i32) -> i32;
    /// `World.getSkyDarken` (`skyDarken`, 0 by day and 11 at night here).
    fn sky_darken(&self) -> i32;
    fn thundering(&self) -> bool;
    fn nether(&self) -> bool;
    fn peaceful(&self) -> bool;
    fn sea_level(&self) -> i32;
    /// `Chunk.getHighestBlockY`: one above the highest block with a non-zero
    /// 1.8 opacity (the heightmap `findSpawnLocation` and `hasDirectSunlight`
    /// read). It is not "above the highest non-air block": glass, snow
    /// layers, carpets and plants are skipped while water, ice, leaves and
    /// cobwebs count.
    fn highest_block_y(&self, x: i32, z: i32) -> i32;
    /// `LevelGeneratorType.FLAT`.
    fn is_flat(&self) -> bool;
    /// `World.getBiome(pos).biomeName`, the chunk's stored biome.
    fn biome_name(&self, x: i32, y: i32, z: i32) -> Option<&str>;
    /// `ChunkProvider.getSpawnEntries`: the biome's list for the category,
    /// with the swamp hut's and ocean monument's monster overrides when the
    /// position is inside one of those structures.
    fn spawn_entries(&self, category: Category, x: i32, y: i32, z: i32) -> &'static [SpawnEntry18];
    /// `World.isPlayerInRange(x, y, z, distance)`: a non-spectator player.
    fn player_in_range(&self, x: f64, y: f64, z: f64, distance: f64) -> bool;
    /// `World.getSpawnPos`.
    fn spawn_position(&self) -> [i32; 3];
    /// `World.getPersistentEntityCount(category.getCategoryClass())`: the
    /// loaded mobs of the category's 1.8 class that are not persistent.
    fn persistent_count(&self, category: Category) -> i32;
    /// `World.getTime`, the total world time the passive category's 400-tick
    /// gate reads.
    fn game_time(&self) -> i64;
    /// `World.getTimeOfDay`: the day time the moon phase reads (it stops
    /// with the daylight cycle, while `game_time` keeps counting).
    fn day_time(&self) -> i64;
    /// `Chunk.getRandom(salt)`.
    fn chunk_random(&self, cx: i32, cz: i32, salt: i64) -> LegacyRandom;
    /// `WorldBorder.contains(pos)`: the block is inside the border.
    fn border_contains_block(&self, x: i32, y: i32, z: i32) -> bool;
    /// `World.hasEntityIn(bounds, entity) == false`: an entity that 1.8 does
    /// not mark `inanimate` meets the box (living entities, players, vehicles,
    /// falling blocks, TNT, end crystals and the dragon).
    fn has_inanimate_collision(&self, bounds: Aabb) -> bool;
    /// `World.doesBoxCollide(entity, bounds).isEmpty()`.
    fn no_block_collision(&self, bounds: Aabb) -> bool;
    /// `World.containsFluid(bounds)`.
    fn contains_fluid(&self, bounds: Aabb) -> bool;
    /// `block.getMaterial().blocksMovement()` at a position. It is not the
    /// modern opacity: glass, snow layers, carpets and skulls block movement,
    /// while air, fluids, plants, torches, rails, redstone and flower pots do
    /// not ([`block_materials_1_8_9`]).
    fn blocks_movement(&self, x: i32, y: i32, z: i32) -> bool {
        let facts = self.facts();
        let state = facts.get(self.state(x, y, z));
        block_materials_1_8_9::blocks_movement(state.name.as_ref())
    }
    /// `block.getMaterial() == Material.FOILAGE` at a position.
    fn is_foliage(&self, x: i32, y: i32, z: i32) -> bool {
        let facts = self.facts();
        let state = facts.get(self.state(x, y, z));
        block_materials_1_8_9::is_foliage(state.name.as_ref())
    }
    /// `MobSpawnerHelper.isSpawnable` at a position (the tests' seam; the
    /// default is the rule in this module).
    fn spawnable(&self, location: Location18, x: i32, y: i32, z: i32) -> bool {
        is_spawnable(self, location, x, y, z)
    }
    /// `World.getTopPosition`: the first position above a block that blocks
    /// movement and is not foliage, scanning down from the top of the world
    /// (1.8 starts at `chunk.getHighestNonEmptySectionYOffset() + 16`, and
    /// everything above that is air). Returns -1 when nothing blocks.
    fn top_position(&self, x: i32, z: i32) -> i32 {
        top_position_at(self, x, z)
    }
}

/// The scan behind [`SpawnLevel18::top_position`].
pub fn top_position_at<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, z: i32) -> i32 {
    let mut y = WORLD_MAX_Y;
    loop {
        if y < 0 {
            return y;
        }
        let below = y - 1;
        if below >= 0 && l.blocks_movement(x, below, z) && !l.is_foliage(x, below, z) {
            return y;
        }
        y = below;
    }
}

/// 1.8's world height (16 sections of 16): `getTopPosition` scans from here.
pub const WORLD_MAX_Y: i32 = 256;

/// `MathHelper.roundUp`.
pub fn round_up(value: i32, interval: i32) -> i32 {
    if interval == 0 {
        return 0;
    }
    if value == 0 {
        return interval;
    }
    let interval = if value < 0 { -interval } else { interval };
    let rem = value % interval;
    if rem == 0 {
        value
    } else {
        value + interval - rem
    }
}

/// `Dimension.MOON_PHASE_TO_SIZE`.
pub const MOON_PHASE_TO_SIZE: [f32; 8] = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];

/// `World.getMoonSize`.
pub fn moon_size<L: SpawnLevel18 + ?Sized>(l: &L) -> f32 {
    let phase = (l.day_time() / 24_000).rem_euclid(8) as usize;
    MOON_PHASE_TO_SIZE[phase]
}

/// `Chunk.getRandom(salt)`: the chunk's own random, from `World.getSeed` and
/// the chunk coordinates (the `cx*cx*4987142` term is 32-bit arithmetic).
pub fn chunk_random(seed: i64, cx: i32, cz: i32, salt: i64) -> LegacyRandom {
    let a = cx.wrapping_mul(cx).wrapping_mul(4_987_142) as i64;
    let b = cx.wrapping_mul(5_947_611) as i64;
    let c = (cz.wrapping_mul(cz) as i64).wrapping_mul(4_392_871);
    let d = cz.wrapping_mul(389_711) as i64;
    let seed = seed
        .wrapping_add(a)
        .wrapping_add(b)
        .wrapping_add(c)
        .wrapping_add(d);
    LegacyRandom::new(seed ^ salt)
}

/// `Block.usesNeighbourLight`: stairs, slabs, farmland, the air/plant/portal
/// materials, and every block whose 1.8 opacity is 0 (glass, snow layers,
/// carpets, barriers, plants...). The exported `dampening` is that opacity.
pub fn uses_neighbor_light(f: &BlockFacts, state: BlockStateId) -> bool {
    let state = f.get(state);
    let name = &*state.name;
    if matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    ) {
        return false;
    }
    name.ends_with("_stairs")
        || name.ends_with("_slab")
        || name == "minecraft:farmland"
        || name == "minecraft:barrier"
        || state.dampening == 0
}

/// `Block.isFullCube`: an opaque normal-rendering block that emits no
/// redstone power. Of the blocks that emit power only the redstone block is
/// also an opaque normal cube, so it is the one exception.
pub fn is_full_cube(f: &BlockFacts, state: BlockStateId) -> bool {
    let state = f.get(state);
    full_cube(state.can_occlude, &state.name)
}

fn full_cube(can_occlude: bool, name: &str) -> bool {
    can_occlude && name != "minecraft:redstone_block"
}

/// `World.isOpaque`: an opaque normal-rendering block, or a top-half stair or
/// slab, a hopper, or the tallest snow layer.
pub fn is_opaque<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, y: i32, z: i32) -> bool {
    let f = l.facts();
    let state = f.get(l.state(x, y, z));
    if state.can_occlude {
        return true;
    }
    let name = &*state.name;
    if name.ends_with("_stairs") {
        return l.prop(x, y, z, "half") == Some("top");
    }
    if name.ends_with("_slab") {
        return l.prop(x, y, z, "type") == Some("top");
    }
    if name == "minecraft:hopper" {
        return true;
    }
    if name == "minecraft:snow" {
        return l.prop(x, y, z, "layers") == Some("7");
    }
    false
}

/// `World.method_8557`: the raw light, `max(sky light, block light)`.
pub fn raw_light<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, y: i32, z: i32) -> i32 {
    if y < 0 {
        return 0;
    }
    let y = y.min(255);
    l.sky_light(x, y, z).max(l.block_light(x, y, z))
}

/// 1.8's `MathHelper.SINE_TABLE`, built once from `Math.sin`.
fn sine_table() -> &'static [f32] {
    static TABLE: std::sync::OnceLock<Box<[f32]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        (0..65536)
            .map(|i| ((i as f64) * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32)
            .collect()
    })
}

/// `MathHelper.cos`: the 65536-entry sine table, indexed through 1.8's float
/// stride (`10430.378`, a quarter-turn offset).
pub fn mathhelper_cos(value: f32) -> f32 {
    let index = (value * 10430.378 + 16384.0) as i32 & 0xFFFF;
    sine_table()[index as usize]
}

/// `Dimension.getSkyAngle(timeOfDay, tickDelta)`: the smoothed day fraction.
pub fn sky_angle(day_time: i64, tick_delta: f32) -> f32 {
    let i = (day_time % 24_000) as i32;
    let mut f = (i as f32 + tick_delta) / 24_000.0 - 0.25;
    if f < 0.0 {
        f += 1.0;
    }
    if f > 1.0 {
        f -= 1.0;
    }
    let g = f;
    // Java's `(float)((Math.cos((double)f * Math.PI) + 1.0) / 2.0)`.
    let half = ((f as f64 * std::f64::consts::PI).cos() + 1.0) / 2.0;
    f = 1.0 - half as f32;
    g + (f - g) / 3.0
}

/// `World.method_3597(1.0)`: the ambient sky darkness. A clear day is 0, a
/// clear night 11; the rain and thunder gradients (the values
/// `getRainGradient(1.0)` and `getThunderGradient(1.0)` return, the latter
/// already scaled by rain) lift the daytime floor toward 3 and 5.
pub fn sky_darken(day_time: i64, rain_gradient: f32, thunder_gradient: f32) -> i32 {
    let g = sky_angle(day_time, 1.0);
    let mut h = 1.0 - (mathhelper_cos(g * (std::f32::consts::PI * 2.0)) * 2.0 + 0.5);
    h = h.clamp(0.0, 1.0);
    h = 1.0 - h;
    h = (h as f64 * (1.0 - (rain_gradient * 5.0) as f64 / 16.0)) as f32;
    h = (h as f64 * (1.0 - (thunder_gradient * 5.0) as f64 / 16.0)) as f32;
    h = 1.0 - h;
    (h * 11.0) as i32
}

/// `World.method_8558` / `method_8543(pos, true)`: the combined light after
/// ambient darkness, or the highest of the five neighbours' combined light
/// when the block uses neighbour light.
pub fn combined_light<L: SpawnLevel18 + ?Sized>(
    l: &L,
    x: i32,
    y: i32,
    z: i32,
    darkening: i32,
) -> i32 {
    combined_light_from(l, x, y, z, darkening, true)
}

/// `method_8543`: `expand` is the `bl` argument. The neighbour reads pass
/// `false` in vanilla, so they never expand again, but they still apply the
/// ambient darkness.
fn combined_light_from<L: SpawnLevel18 + ?Sized>(
    l: &L,
    x: i32,
    y: i32,
    z: i32,
    darkening: i32,
    expand: bool,
) -> i32 {
    if expand && uses_neighbor_light(l.facts(), l.state(x, y, z)) {
        let mut light = combined_light_from(l, x, y + 1, z, darkening, false);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            light = light.max(combined_light_from(l, x + dx, y, z + dz, darkening, false));
        }
        return light;
    }
    if y < 0 {
        return 0;
    }
    let y = y.min(255);
    (l.sky_light(x, y, z) - darkening).max(l.block_light(x, y, z))
}

/// `Dimension.lightLevelToBrightness` with the dimension's ambient light.
pub fn brightness_from_light(level: i32, nether: bool) -> f32 {
    let f = level.clamp(0, 15) as f32 / 15.0;
    let base = f / (4.0 - 3.0 * f);
    if nether { 0.1 + base * 0.9 } else { base }
}

/// `World.getBrightness(pos)`: the brightness of `method_8558` at a block.
pub fn brightness<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, y: i32, z: i32) -> f32 {
    let level = combined_light(l, x, y, z, l.sky_darken());
    brightness_from_light(level, l.nether())
}

/// `Chunk.getHighestBlockY` from the exported 1.8 opacity: one above the
/// highest block whose light dampening is non-zero. Full cubes export as 15
/// and the partial blocks that 1.8 gave an explicit opacity (water, ice,
/// leaves, cobwebs, farmland, slabs, stairs) as 1; glass, snow layers,
/// carpets and plants export as 0.
pub fn highest_block_y<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, z: i32) -> i32 {
    for y in (0..=255).rev() {
        if l.facts().get(l.state(x, y, z)).dampening != 0 {
            return y + 1;
        }
    }
    0
}

/// `Chunk.hasDirectSunlight`: the block is at or above the heightmap.
pub fn has_direct_sunlight<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, y: i32, z: i32) -> bool {
    y >= l.highest_block_y(x, z)
}

/// `World.recievesSunlight`: direct sunlight at or above sea level; below it,
/// the sea-level column is directly lit and no opaque non-fluid block stands
/// between the position and sea level.
pub fn receives_sunlight<L: SpawnLevel18 + ?Sized>(l: &L, x: i32, y: i32, z: i32) -> bool {
    let sea = l.sea_level();
    if y >= sea {
        return has_direct_sunlight(l, x, y, z);
    }
    if !has_direct_sunlight(l, x, sea, z) {
        return false;
    }
    let mut yy = sea - 1;
    while yy > y {
        let f = l.facts().get(l.state(x, yy, z));
        if f.dampening > 0 && f.fluid == Fluid::None {
            return false;
        }
        yy -= 1;
    }
    true
}

/// `HostileEntity.method_3087`: the sky light roll, then `method_8558` under
/// the usual ambient darkness (10 while thundering) against `rand(8)`.
pub fn dark_enough<L: SpawnLevel18 + ?Sized>(
    l: &L,
    random: &mut impl RandomSource,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    if l.sky_light(x, y, z) > random.next_int_bound(32).unwrap_or(0) {
        return false;
    }
    let darkening = if l.thundering() { 10 } else { l.sky_darken() };
    combined_light(l, x, y, z, darkening) <= random.next_int_bound(8).unwrap_or(0)
}

/// `MobSpawnerHelper.isSpawnable`.
pub fn is_spawnable<L: SpawnLevel18 + ?Sized>(
    l: &L,
    location: Location18,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    if !l.border_contains_block(x, y, z) {
        return false;
    }
    let facts = l.facts();
    let here = l.state(x, y, z);
    if location == Location18::InWater {
        return facts.get(here).fluid != Fluid::None
            && facts.get(l.state(x, y - 1, z)).fluid != Fluid::None
            && !is_full_cube(facts, l.state(x, y + 1, z));
    }
    if !is_opaque(l, x, y - 1, z) {
        return false;
    }
    let below = &*facts.get(l.state(x, y - 1, z)).name;
    let solid = below != "minecraft:bedrock" && below != "minecraft:barrier";
    solid
        && !is_full_cube(facts, here)
        && facts.get(here).fluid == Fluid::None
        && !is_full_cube(facts, l.state(x, y + 1, z))
}

/// `Weighting.rand(random, list)`: one weighted entry, or none for an empty
/// list (the only case `getSpawnEntries` can return).
pub fn pick_weighted<'a>(
    list: &'a [SpawnEntry18],
    random: &mut impl RandomSource,
) -> Option<&'a SpawnEntry18> {
    let total: i32 = list.iter().map(|e| e.weight).sum();
    if total <= 0 {
        return None;
    }
    let mut pick = random.next_int_bound(total).ok()?;
    for entry in list {
        pick -= entry.weight;
        if pick < 0 {
            return Some(entry);
        }
    }
    None
}

/// `Weighting.rand` over `SpawnLevel18::spawn_entries`.
pub fn pick_entry<L: SpawnLevel18 + ?Sized>(
    l: &L,
    category: Category,
    x: i32,
    y: i32,
    z: i32,
    random: &mut impl RandomSource,
) -> Option<&'static SpawnEntry18> {
    pick_weighted(l.spawn_entries(category, x, y, z), random)
}

/// `MobSpawnerHelper.findSpawnLocation`: a random column of the chunk, then a
/// random height below the surface rounded up to a section.
pub fn find_spawn_location<L: SpawnLevel18 + ?Sized>(
    l: &L,
    cx: i32,
    cz: i32,
    random: &mut impl RandomSource,
) -> [i32; 3] {
    let x = cx * 16 + random.next_int_bound(16).unwrap_or(0);
    let z = cz * 16 + random.next_int_bound(16).unwrap_or(0);
    // `MathHelper.roundUp(chunk.getHighestBlockY(pos) + 1, 16)`.
    let height = round_up(l.highest_block_y(x, z) + 1, 16);
    let y = random.next_int_bound(height.max(1)).unwrap_or(0);
    [x, y, z]
}

/// `Biome.getMaxSpawnLimit`: every 1.8 biome returns 0.1 (`MutatedBiome`
/// delegates to its base, which does too).
pub const MAX_SPAWN_LIMIT: f32 = 0.1;

/// What chunk generation spawning did (for differential tests).
#[derive(Clone, Debug, PartialEq)]
pub enum GenEvent18 {
    /// A pack attempt's `getTopPosition` lookup.
    Top(i32, i32, i32),
    /// A pack placed a mob: entity, position and yaw.
    Spawn(&'static str, [f64; 3], f32),
}

/// `MobSpawnerHelper.spawnMobs`: the passive pack a chunk's decoration
/// places, from the biome at the chunk's `(cx*16+16, 0, cz*16+16)` corner.
///
/// `random` is the chunk provider's decoration random (pack loop, group
/// size, offsets, drifts and yaw); `world_random` is `World.random` (the
/// weighted picks). 1.8's `initialize` also draws from the world random for
/// a few species; the 26.3-shaped finalizer does not reproduce those draws
/// (see the debt in `docs/mc-1.8.9.md`). Every mob is `isSpawnable`-checked
/// only: generation does not run `canSpawn`/`method_2469`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_mobs_for_chunk_generation<
    L: SpawnLevel18 + ?Sized,
    R: RandomSource,
    W: RandomSource,
>(
    l: &L,
    cx: i32,
    cz: i32,
    random: &mut R,
    world_random: &mut W,
    spawn: &mut dyn FnMut(&L, &'static str, [f64; 3], f32, &mut W) -> bool,
    trace: &mut dyn FnMut(GenEvent18),
) {
    // `SurfaceChunkGenerator.decorateChunk` reads the biome at the chunk
    // corner (one block into the next chunk) before decorating it.
    let list = l.spawn_entries(Category::Creature, cx * 16 + 16, 0, cz * 16 + 16);
    if list.is_empty() {
        return;
    }
    while random.next_float() < MAX_SPAWN_LIMIT {
        let Some(entry) = pick_weighted(list, world_random) else {
            return;
        };
        let count = entry.min
            + random
                .next_int_bound(1 + entry.max - entry.min)
                .unwrap_or(0);
        let mut x = cx * 16 + 8 + random.next_int_bound(16).unwrap_or(0);
        let mut z = cz * 16 + 8 + random.next_int_bound(16).unwrap_or(0);
        let (origin_x, origin_z) = (x, z);
        for _ in 0..count {
            let mut placed = false;
            for _ in 0..4 {
                if placed {
                    break;
                }
                let y = l.top_position(x, z);
                trace(GenEvent18::Top(x, y, z));
                if l.spawnable(Location18::OnGround, x, y, z) {
                    let yaw = random.next_float() * 360.0;
                    // The mob is finalized inside the callback, and 1.8's
                    // sheep colour and villager profession roll on the
                    // world's random, so it goes along.
                    placed = spawn(
                        l,
                        entry.entity,
                        [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
                        yaw,
                        world_random,
                    );
                    if placed {
                        trace(GenEvent18::Spawn(
                            entry.entity,
                            [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
                            yaw,
                        ));
                    }
                }
                // The drift and clamp run even on the attempt that placed a
                // mob; vanilla draws them before the loop condition rechecks.
                let drift = |random: &mut R| {
                    random.next_int_bound(5).unwrap_or(0) - random.next_int_bound(5).unwrap_or(0)
                };
                x += drift(random);
                z += drift(random);
                while x < cx * 16 + 8
                    || x >= cx * 16 + 8 + 16
                    || z < cz * 16 + 8
                    || z >= cz * 16 + 8 + 16
                {
                    x = origin_x + drift(random);
                    z = origin_z + drift(random);
                }
            }
        }
    }
}

/// `AnimalEntity.canSpawn`: the spawn block stands on `on` and the raw light
/// is above 8.
fn animal<L: SpawnLevel18 + ?Sized>(l: &L, on: &str, x: i32, y: i32, z: i32) -> bool {
    &*l.facts().get(l.state(x, y - 1, z)).name == on && raw_light(l, x, y, z) > 8
}

/// `SlimeEntity.canSpawn`: superflat's 1-in-4, the swamp roll (surface height,
/// half chance, moon size, light against `rand(8)`), or the slime-chunk roll
/// below y 40. `MobEntity.canSpawn` is the remaining super.
fn slime<L: SpawnLevel18 + ?Sized>(
    l: &L,
    x: i32,
    y: i32,
    z: i32,
    y_f64: f64,
    random: &mut impl RandomSource,
) -> bool {
    if l.is_flat() && random.next_int_bound(4).unwrap_or(0) != 1 {
        return false;
    }
    if l.peaceful() {
        return false;
    }
    if l.biome_name(x, y, z) == Some("minecraft:swamp")
        && y_f64 > 50.0
        && y_f64 < 70.0
        && random.next_float() < 0.5
        && random.next_float() < moon_size(l)
        && combined_light(l, x, y, z, l.sky_darken()) <= random.next_int_bound(8).unwrap_or(0)
    {
        return true;
    }
    random.next_int_bound(10).unwrap_or(0) == 0
        && l.chunk_random(x >> 4, z >> 4, 987_234_911)
            .next_int_bound(10)
            .unwrap_or(0)
            == 0
        && y_f64 < 40.0
}

/// `BatEntity.canSpawn`: below sea level, Halloween or a coin flip, then
/// combined light against `rand(4)` (`rand(7)` on Halloween).
fn bat<L: SpawnLevel18 + ?Sized>(
    l: &L,
    x: i32,
    y: i32,
    z: i32,
    random: &mut impl RandomSource,
) -> bool {
    if y >= l.sea_level() {
        return false;
    }
    let light = combined_light(l, x, y, z, l.sky_darken());
    let bound = if crate::mobs::finalize::is_halloween() {
        7
    } else {
        if random.next_boolean() {
            return false;
        }
        4
    };
    light <= random.next_int_bound(bound).unwrap_or(0)
}

/// A mob's `canSpawn` for the 1.8 spawn lists. `world_random` is the level's
/// random (the ocelot's roll reads it); the others read the entity's own
/// random, whose draws vanilla does not tie to the level's sequence.
pub fn can_spawn<L: SpawnLevel18 + ?Sized>(
    l: &L,
    entity: &str,
    entity_random: &mut impl RandomSource,
    world_random: &mut impl RandomSource,
    x: i32,
    y: i32,
    z: i32,
    y_f64: f64,
) -> bool {
    match entity {
        "minecraft:chicken" | "minecraft:cow" | "minecraft:pig" | "minecraft:rabbit"
        | "minecraft:sheep" | "minecraft:wolf" | "minecraft:horse" => {
            animal(l, "minecraft:grass_block", x, y, z)
        }
        // `MooshroomEntity` only changes the block under it (mycelium); its
        // `AnimalEntity.canSpawn` is the same grass-or-mycelium and raw
        // light check as the other animals.
        "minecraft:mooshroom" => animal(l, "minecraft:mycelium", x, y, z),
        "minecraft:ocelot" => world_random.next_int_bound(3).unwrap_or(0) != 0,
        "minecraft:bat" => bat(l, x, y, z, entity_random),
        "minecraft:squid" => y_f64 > 45.0 && y_f64 < f64::from(l.sea_level()),
        "minecraft:ghast" => entity_random.next_int_bound(20).unwrap_or(0) == 0 && !l.peaceful(),
        "minecraft:guardian" => {
            (entity_random.next_int_bound(20).unwrap_or(0) == 0 || !receives_sunlight(l, x, y, z))
                && !l.peaceful()
                && dark_enough(l, entity_random, x, y, z)
        }
        "minecraft:zombified_piglin" | "minecraft:magma_cube" => !l.peaceful(),
        "minecraft:slime" => slime(l, x, y, z, y_f64, entity_random),
        _ => !l.peaceful() && dark_enough(l, entity_random, x, y, z),
    }
}

/// `MobEntity.method_2469` and its overrides: clear of entities 1.8 treats as
/// spawn blockers and, except for water creatures, clear of block collisions
/// and fluid; the ocelot also needs grass or leaves under it at or above sea
/// level.
pub fn method_2469<L: SpawnLevel18 + ?Sized>(l: &L, entity: &str, bounds: Aabb) -> bool {
    if l.has_inanimate_collision(bounds) {
        return false;
    }
    match entity {
        "minecraft:squid" => true,
        "minecraft:guardian" => l.no_block_collision(bounds),
        "minecraft:ocelot" => {
            if !l.no_block_collision(bounds) || l.contains_fluid(bounds) {
                return false;
            }
            let min = bounds.min_position();
            let (x, y, z) = (
                min.x.floor() as i32,
                min.y.floor() as i32,
                min.z.floor() as i32,
            );
            if y < l.sea_level() {
                return false;
            }
            let name = &*l.facts().get(l.state(x, y - 1, z)).name;
            name == "minecraft:grass_block" || name.ends_with("_leaves")
        }
        _ => l.no_block_collision(bounds) && !l.contains_fluid(bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fast_hash::FxHashMap;
    use horizon_core::random::PositionalRandom;
    use std::collections::VecDeque;

    #[test]
    fn round_up_matches_math_helper() {
        assert_eq!(round_up(0, 16), 16);
        assert_eq!(round_up(1, 16), 16);
        assert_eq!(round_up(16, 16), 16);
        assert_eq!(round_up(17, 16), 32);
        assert_eq!(round_up(31, 16), 32);
        assert_eq!(round_up(32, 16), 32);
        assert_eq!(round_up(65, 16), 80);
        assert_eq!(round_up(9, 0), 0);
    }

    #[test]
    fn limits_are_the_1_8_overrides() {
        assert_eq!(limit_per_chunk("minecraft:wolf"), 8);
        assert_eq!(limit_per_chunk("minecraft:horse"), 6);
        assert_eq!(limit_per_chunk("minecraft:ghast"), 1);
        for other in [
            "minecraft:chicken",
            "minecraft:zombie",
            "minecraft:squid",
            "minecraft:guardian",
            "minecraft:slime",
        ] {
            assert_eq!(limit_per_chunk(other), 4, "{other}");
        }
    }

    #[test]
    fn sky_darken_follows_the_1_8_ambient_curve() {
        // Expected values from running 1.8.9's Dimension.getSkyAngle and
        // World.method_3597 (with MathHelper's sine table) on the 1.8 jar.
        assert_eq!(sky_angle(6000, 0.0), 0.0);
        assert_eq!(sky_angle(18000, 0.0), 0.5);
        assert_eq!(mathhelper_cos(0.0), 1.0);
        assert_eq!(sky_darken(6000, 0.0, 0.0), 0);
        assert_eq!(sky_darken(18000, 0.0, 0.0), 11);
        // Noon under a full rain gradient, then rain and thunder.
        assert_eq!(sky_darken(6000, 1.0, 0.0), 3);
        assert_eq!(sky_darken(6000, 1.0, 1.0), 5);
        // Half rain at noon reaches 1, a quarter does not reach 1 yet.
        assert_eq!(sky_darken(6000, 0.5, 0.0), 1);
        assert_eq!(sky_darken(6000, 0.25, 0.0), 0);
        // Dusk: the sunlight-burn boundary of the day track and just after.
        assert_eq!(sky_darken(12542, 0.0, 0.0), 4);
        assert_eq!(sky_darken(13000, 0.0, 0.0), 6);
        assert_eq!(sky_darken(23460, 0.0, 0.0), 3);
        // Negative day times wrap the same way java's % does.
        assert_eq!(sky_darken(-1000, 0.0, 0.0), 6);
    }

    #[test]
    fn spawn_requires_the_world_border() {
        // The fake border rejects everything, so is_spawnable stops before
        // it would read a block state.
        let l = FakeLevel::default();
        assert!(!is_spawnable(&l, Location18::OnGround, 0, 64, 0));
        assert!(!is_spawnable(&l, Location18::InWater, 0, 64, 0));
    }

    #[test]
    fn only_squid_and_guardian_spawn_in_water() {
        assert_eq!(location_of("minecraft:squid"), Location18::InWater);
        assert_eq!(location_of("minecraft:guardian"), Location18::InWater);
        for other in [
            "minecraft:chicken",
            "minecraft:bat",
            "minecraft:zombie",
            "minecraft:horse",
            "minecraft:magma_cube",
        ] {
            assert_eq!(location_of(other), Location18::OnGround, "{other}");
        }
    }

    #[test]
    fn chunk_random_uses_the_1_8_terms() {
        // The Java expression with cx = 3, cz = -7 and salt 987234911.
        let mut java = LegacyRandom::new(
            (0x1234_5678_i64)
                .wrapping_add((3 * 3 * 4_987_142) as i64)
                .wrapping_add((3 * 5_947_611) as i64)
                .wrapping_add(((7 * 7) as i64).wrapping_mul(4_392_871))
                .wrapping_add((-7 * 389_711) as i64)
                ^ 987_234_911,
        );
        let mut ours = chunk_random(0x1234_5678, 3, -7, 987_234_911);
        for _ in 0..8 {
            assert_eq!(ours.next_int(), java.next_int());
        }
    }

    #[test]
    fn a_redstone_block_is_not_a_full_cube() {
        // `Block.isFullCube` excludes blocks that emit redstone power; the
        // redstone block is the only opaque normal cube that does.
        assert!(full_cube(true, "minecraft:stone"));
        assert!(!full_cube(true, "minecraft:redstone_block"));
        assert!(!full_cube(false, "minecraft:glass"));
    }

    #[test]
    fn moon_phase_sizes_follow_the_1_8_table() {
        let l = FakeLevel {
            time: 24_000 * 4,
            ..FakeLevel::default()
        };
        assert_eq!(moon_size(&l), 0.0);
        let l = FakeLevel {
            time: 24_000,
            ..FakeLevel::default()
        };
        assert_eq!(moon_size(&l), 0.75);
    }

    #[derive(Default)]
    struct FakeLevel {
        time: i64,
        lists: FxHashMap<(usize, i32, i32, i32), &'static [SpawnEntry18]>,
        top: i32,
        pass: bool,
        column: FxHashMap<i32, (bool, bool)>,
        /// Use the real `getTopPosition` scan over `column` instead of `top`.
        scan: bool,
    }
    impl SpawnLevel18 for FakeLevel {
        fn state(&self, _: i32, _: i32, _: i32) -> BlockStateId {
            unimplemented!()
        }
        fn facts(&self) -> &BlockFacts {
            unimplemented!()
        }
        fn prop(&self, _: i32, _: i32, _: i32, _: &str) -> Option<&str> {
            unimplemented!()
        }
        fn sky_light(&self, _: i32, _: i32, _: i32) -> i32 {
            unimplemented!()
        }
        fn block_light(&self, _: i32, _: i32, _: i32) -> i32 {
            unimplemented!()
        }
        fn sky_darken(&self) -> i32 {
            0
        }
        fn thundering(&self) -> bool {
            false
        }
        fn nether(&self) -> bool {
            false
        }
        fn peaceful(&self) -> bool {
            false
        }
        fn sea_level(&self) -> i32 {
            63
        }
        fn highest_block_y(&self, _: i32, _: i32) -> i32 {
            unimplemented!()
        }
        fn is_flat(&self) -> bool {
            false
        }
        fn biome_name(&self, _: i32, _: i32, _: i32) -> Option<&str> {
            None
        }
        fn spawn_entries(
            &self,
            category: Category,
            x: i32,
            y: i32,
            z: i32,
        ) -> &'static [SpawnEntry18] {
            self.lists
                .get(&(category.index(), x, y, z))
                .copied()
                .unwrap_or(&[])
        }
        fn player_in_range(&self, _: f64, _: f64, _: f64, _: f64) -> bool {
            false
        }
        fn spawn_position(&self) -> [i32; 3] {
            [0, 64, 0]
        }
        fn persistent_count(&self, _: Category) -> i32 {
            0
        }
        fn game_time(&self) -> i64 {
            self.time
        }
        fn day_time(&self) -> i64 {
            self.time
        }
        fn chunk_random(&self, cx: i32, cz: i32, salt: i64) -> LegacyRandom {
            chunk_random(1, cx, cz, salt)
        }
        fn border_contains_block(&self, _: i32, _: i32, _: i32) -> bool {
            false
        }
        fn has_inanimate_collision(&self, _: Aabb) -> bool {
            false
        }
        fn no_block_collision(&self, _: Aabb) -> bool {
            true
        }
        fn contains_fluid(&self, _: Aabb) -> bool {
            false
        }
        fn top_position(&self, x: i32, z: i32) -> i32 {
            if self.scan {
                top_position_at(self, x, z)
            } else {
                self.top
            }
        }
        fn spawnable(&self, _: Location18, _: i32, _: i32, _: i32) -> bool {
            self.pass
        }
        fn blocks_movement(&self, _: i32, y: i32, _: i32) -> bool {
            self.column.get(&y).is_some_and(|&(blocks, _)| blocks)
        }
        fn is_foliage(&self, _: i32, y: i32, _: i32) -> bool {
            self.column.get(&y).is_some_and(|&(_, foliage)| foliage)
        }
    }

    const ENTRIES: &[SpawnEntry18] = &[
        SpawnEntry18 {
            entity: "minecraft:sheep",
            weight: 12,
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

    fn pick_with(seed: i64) -> &'static str {
        let mut l = FakeLevel::default();
        l.lists.insert((1, 0, 0, 0), ENTRIES);
        let mut random = LegacyRandom::new(seed);
        pick_entry(&l, Category::Creature, 0, 0, 0, &mut random)
            .unwrap()
            .entity
    }

    #[test]
    fn weighted_picks_follow_weighting_rand() {
        // The exact sequence of java.util.Random(7) over [0, 30): 12 + 8 + 10.
        let mut random = LegacyRandom::new(7);
        let mut expected = Vec::new();
        for _ in 0..6 {
            let mut pick = random.next_int_bound(30).unwrap();
            expected.push(if pick < 12 {
                "minecraft:sheep"
            } else {
                pick -= 12;
                if pick < 8 {
                    "minecraft:cow"
                } else {
                    "minecraft:chicken"
                }
            });
        }
        let mut random = LegacyRandom::new(7);
        let mut l = FakeLevel::default();
        l.lists.insert((1, 0, 0, 0), ENTRIES);
        for want in expected {
            let got = pick_entry(&l, Category::Creature, 0, 0, 0, &mut random)
                .unwrap()
                .entity;
            assert_eq!(got, want);
        }
        let _ = pick_with(7);
    }

    #[test]
    fn an_empty_list_draws_nothing() {
        let l = FakeLevel::default();
        let mut random = LegacyRandom::new(7);
        assert!(pick_entry(&l, Category::Monster, 0, 0, 0, &mut random).is_none());
        let mut untouched = LegacyRandom::new(7);
        assert_eq!(random.next_int(), untouched.next_int());
    }

    #[test]
    fn spawn_location_draws_column_then_height() {
        // world_surface is stubbed to panic, so only the first two draws are
        // reachable through a level that supplies one.
        struct Flat(FakeLevel);
        impl SpawnLevel18 for Flat {
            fn state(&self, x: i32, y: i32, z: i32) -> BlockStateId {
                self.0.state(x, y, z)
            }
            fn facts(&self) -> &BlockFacts {
                self.0.facts()
            }
            fn prop(&self, x: i32, y: i32, z: i32, k: &str) -> Option<&str> {
                self.0.prop(x, y, z, k)
            }
            fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
                self.0.sky_light(x, y, z)
            }
            fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
                self.0.block_light(x, y, z)
            }
            fn sky_darken(&self) -> i32 {
                self.0.sky_darken()
            }
            fn thundering(&self) -> bool {
                self.0.thundering()
            }
            fn nether(&self) -> bool {
                self.0.nether()
            }
            fn peaceful(&self) -> bool {
                self.0.peaceful()
            }
            fn sea_level(&self) -> i32 {
                self.0.sea_level()
            }
            fn highest_block_y(&self, _: i32, _: i32) -> i32 {
                64
            }
            fn is_flat(&self) -> bool {
                self.0.is_flat()
            }
            fn biome_name(&self, x: i32, y: i32, z: i32) -> Option<&str> {
                self.0.biome_name(x, y, z)
            }
            fn spawn_entries(
                &self,
                c: Category,
                x: i32,
                y: i32,
                z: i32,
            ) -> &'static [SpawnEntry18] {
                self.0.spawn_entries(c, x, y, z)
            }
            fn player_in_range(&self, x: f64, y: f64, z: f64, d: f64) -> bool {
                self.0.player_in_range(x, y, z, d)
            }
            fn spawn_position(&self) -> [i32; 3] {
                self.0.spawn_position()
            }
            fn persistent_count(&self, c: Category) -> i32 {
                self.0.persistent_count(c)
            }
            fn game_time(&self) -> i64 {
                self.0.game_time()
            }
            fn day_time(&self) -> i64 {
                self.0.day_time()
            }
            fn chunk_random(&self, cx: i32, cz: i32, salt: i64) -> LegacyRandom {
                self.0.chunk_random(cx, cz, salt)
            }
            fn border_contains_block(&self, x: i32, y: i32, z: i32) -> bool {
                self.0.border_contains_block(x, y, z)
            }
            fn has_inanimate_collision(&self, b: Aabb) -> bool {
                self.0.has_inanimate_collision(b)
            }
            fn no_block_collision(&self, b: Aabb) -> bool {
                self.0.no_block_collision(b)
            }
            fn contains_fluid(&self, b: Aabb) -> bool {
                self.0.contains_fluid(b)
            }
        }
        let l = Flat(FakeLevel::default());
        let mut random = LegacyRandom::new(11);
        let mut expect = LegacyRandom::new(11);
        let x = 5 * 16 + expect.next_int_bound(16).unwrap();
        let z = -2 * 16 + expect.next_int_bound(16).unwrap();
        // Heightmap 64: roundUp(65, 16) == 80.
        let y = expect.next_int_bound(80).unwrap();
        assert_eq!(
            find_spawn_location(&l, 5, -2, &mut random),
            [x, y, z],
            "16 draws for x and z, then rand(round_up(heightmap + 1, 16))"
        );
    }

    /// A random whose floats and integer bounds come from a script, so a test
    /// can pin the exact draw order and still use a real `java.util.Random`
    /// for anything the script does not cover.
    struct Scripted {
        floats: VecDeque<f32>,
        ints: VecDeque<(i32, i32)>,
        inner: LegacyRandom,
        floats_used: usize,
        ints_used: usize,
    }
    impl Default for Scripted {
        fn default() -> Self {
            Self {
                floats: VecDeque::new(),
                ints: VecDeque::new(),
                inner: LegacyRandom::new(0),
                floats_used: 0,
                ints_used: 0,
            }
        }
    }
    impl Scripted {
        fn new(floats: &[f32], ints: &[(i32, i32)]) -> Self {
            Self {
                floats: floats.iter().copied().collect(),
                ints: ints.iter().copied().collect(),
                ..Self::default()
            }
        }
        fn empty(&self) -> bool {
            self.floats.is_empty() && self.ints.is_empty()
        }
    }
    struct ScriptedPositional;
    impl PositionalRandom for ScriptedPositional {
        type Source = Scripted;
        fn at(&self, _: horizon_core::position::BlockPos) -> Scripted {
            Scripted::default()
        }
        fn from_seed(&self, _: i64) -> Scripted {
            Scripted::default()
        }
        fn from_hash_of(&self, _: &str) -> Scripted {
            Scripted::default()
        }
        fn from_hash_of_utf16(&self, _: &[u16]) -> Scripted {
            Scripted::default()
        }
    }
    impl RandomSource for Scripted {
        type Positional = ScriptedPositional;
        fn set_seed(&mut self, seed: i64) {
            self.inner.set_seed(seed);
        }
        fn next_int(&mut self) -> i32 {
            self.inner.next_int()
        }
        fn next_int_bound(
            &mut self,
            bound: i32,
        ) -> Result<i32, horizon_core::random::InvalidBound> {
            self.ints_used += 1;
            if let Some((want, value)) = self.ints.pop_front() {
                assert_eq!(want, bound, "a draw asked for a different bound");
                return Ok(value);
            }
            self.inner.next_int_bound(bound)
        }
        fn next_long(&mut self) -> i64 {
            self.inner.next_long()
        }
        fn next_boolean(&mut self) -> bool {
            self.inner.next_boolean()
        }
        fn next_float(&mut self) -> f32 {
            self.floats_used += 1;
            self.floats
                .pop_front()
                .unwrap_or_else(|| self.inner.next_float())
        }
        fn next_double(&mut self) -> f64 {
            self.inner.next_double()
        }
        fn next_gaussian(&mut self) -> f64 {
            self.inner.next_gaussian()
        }
        fn fork_positional(&mut self) -> Self::Positional {
            ScriptedPositional
        }
        fn fork(&mut self) -> Self {
            Scripted::default()
        }
    }

    const ONE_SHEEP: &[SpawnEntry18] = &[SpawnEntry18 {
        entity: "minecraft:sheep",
        weight: 1,
        min: 1,
        max: 1,
    }];

    fn pack_level(top: i32, pass: bool) -> FakeLevel {
        let mut l = FakeLevel {
            top,
            pass,
            ..FakeLevel::default()
        };
        l.lists
            .insert((Category::Creature.index(), 16, 0, 16), ONE_SHEEP);
        l
    }

    #[test]
    fn generation_pack_draws_in_vanilla_order() {
        // One pack: loop float, world-random pick, group size, x/z, then the
        // attempt (top, yaw) and the drift that vanilla still draws after a
        // successful placement. A skipped or reordered draw leaves the script
        // unconsumed.
        let l = pack_level(64, true);
        let mut random = Scripted::new(
            &[0.05, 0.25, 0.5],
            &[(1, 0), (16, 15), (16, 0), (5, 1), (5, 4), (5, 2), (5, 0)],
        );
        let mut world = Scripted::new(&[], &[(1, 0)]);
        let mut events = Vec::new();
        let mut placed = Vec::new();
        spawn_mobs_for_chunk_generation(
            &l,
            0,
            0,
            &mut random,
            &mut world,
            &mut |_, entity, position, yaw, _| {
                placed.push((entity, position, yaw));
                true
            },
            &mut |event| events.push(event),
        );
        assert_eq!(placed, vec![("minecraft:sheep", [23.5, 64.0, 8.5], 90.0)]);
        assert_eq!(
            events,
            vec![
                GenEvent18::Top(23, 64, 8),
                GenEvent18::Spawn("minecraft:sheep", [23.5, 64.0, 8.5], 90.0),
            ]
        );
        assert!(
            random.empty() && world.empty(),
            "every scripted draw was used"
        );
    }

    #[test]
    fn generation_pack_clamps_drift_into_the_chunk() {
        let l = pack_level(64, false);
        // One pack of one: the pick, the group size, the pack origin, then
        // attempt 1 (top, drift to x = 27, outside the chunk) and its clamp
        // back to (20, 11). Attempts 2-4 drift by zero.
        let mut ints = vec![
            (1, 0),
            (16, 15),
            (16, 0),
            (5, 4),
            (5, 0),
            (5, 0),
            (5, 0),
            (5, 1),
            (5, 4),
            (5, 3),
            (5, 0),
        ];
        ints.extend(std::iter::repeat_n((5, 0), 12));
        let mut random = Scripted::new(&[0.05, 0.5], &ints);
        let mut world = Scripted::new(&[], &[(1, 0)]);
        let mut events = Vec::new();
        spawn_mobs_for_chunk_generation(
            &l,
            0,
            0,
            &mut random,
            &mut world,
            &mut |_, _, _, _, _| false,
            &mut |event| events.push(event),
        );
        assert_eq!(
            events,
            vec![
                GenEvent18::Top(23, 64, 8),
                GenEvent18::Top(20, 64, 11),
                GenEvent18::Top(20, 64, 11),
                GenEvent18::Top(20, 64, 11),
            ],
            "four attempts per mob, starting at the clamped column"
        );
        assert!(random.empty() && world.empty());
    }

    #[test]
    fn generation_pack_reads_the_corner_biome() {
        // The list sits on the chunk's own corner, not (16,0,16): 1.8 reads
        // `getBiome(blockPos.add(16, 0, 16))`.
        let mut l = FakeLevel::default();
        l.lists
            .insert((Category::Creature.index(), 0, 0, 0), ONE_SHEEP);
        l.pass = true;
        let mut random = Scripted::new(&[0.05], &[(16, 0), (16, 0), (1, 0)]);
        let mut world = Scripted::new(&[], &[(1, 0)]);
        spawn_mobs_for_chunk_generation(
            &l,
            0,
            0,
            &mut random,
            &mut world,
            &mut |_, _, _, _, _| true,
            &mut |_| {},
        );
        assert_eq!(random.floats_used, 0, "the list is empty, so no pack");
        assert_eq!(random.ints_used, 0);
    }

    #[test]
    fn empty_biome_list_returns_without_drawing() {
        let l = FakeLevel::default();
        let mut random = Scripted::default();
        let mut world = Scripted::default();
        spawn_mobs_for_chunk_generation(
            &l,
            0,
            0,
            &mut random,
            &mut world,
            &mut |_, _, _, _, _| true,
            &mut |_| {},
        );
        assert_eq!((random.floats_used, random.ints_used), (0, 0));
        assert_eq!((world.floats_used, world.ints_used), (0, 0));
    }

    #[test]
    fn top_position_skips_non_blocking_materials_and_foliage() {
        let mut l = FakeLevel {
            scan: true,
            ..FakeLevel::default()
        };
        l.column.insert(63, (true, false));
        assert_eq!(l.top_position(0, 0), 64);
        l.column.insert(70, (true, true));
        assert_eq!(l.top_position(0, 0), 64, "leaves do not stop the scan");
        l.column.insert(64, (true, false));
        assert_eq!(l.top_position(0, 0), 65, "glass blocks movement");
        l.column.insert(64, (false, false));
        assert_eq!(l.top_position(0, 0), 64, "water does not");
        let empty = FakeLevel {
            scan: true,
            ..FakeLevel::default()
        };
        assert_eq!(empty.top_position(0, 0), -1);
        let mut high = FakeLevel {
            scan: true,
            ..FakeLevel::default()
        };
        high.column.insert(255, (true, false));
        assert_eq!(high.top_position(0, 0), 256);
        let mut floor = FakeLevel {
            scan: true,
            ..FakeLevel::default()
        };
        floor.column.insert(0, (true, false));
        assert_eq!(floor.top_position(0, 0), 1);
    }

    #[test]
    fn movement_materials_are_the_1_8_ones() {
        use horizon_world::block_materials_1_8_9::{
            FOLIAGE, NON_BLOCKING, blocks_movement, is_foliage,
        };
        // `Material.blocksMovement` is false for more than modern opacity
        // zero: snow layers, carpets and flower pots have a collision box but
        // a non-blocking material.
        for name in [
            "minecraft:air",
            "minecraft:water",
            "minecraft:lava",
            "minecraft:tall_grass",
            "minecraft:short_grass",
            "minecraft:snow",
            "minecraft:white_carpet",
            "minecraft:cobweb",
            "minecraft:rail",
            "minecraft:torch",
            "minecraft:flower_pot",
            "minecraft:potted_poppy",
            "minecraft:wither_skeleton_skull",
            "minecraft:portal",
        ] {
            assert!(!blocks_movement(name), "{name} blocks movement");
        }
        for name in [
            "minecraft:stone",
            "minecraft:glass",
            "minecraft:snow_block",
            "minecraft:oak_log",
            "minecraft:oak_leaves",
        ] {
            assert!(blocks_movement(name), "{name} does not block movement");
        }
        assert!(is_foliage("minecraft:dark_oak_leaves"));
        assert!(!is_foliage("minecraft:oak_log"));
        assert!(NON_BLOCKING.windows(2).all(|w| w[0] < w[1]));
        assert!(FOLIAGE.windows(2).all(|w| w[0] < w[1]));
    }
}
