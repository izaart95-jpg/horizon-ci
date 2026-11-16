//! The game's side of `NaturalSpawner` (`mobs::spawn_selection`): the level
//! it reads, the spawning chunks and caps of `ServerChunkCache.tickChunks`,
//! and the mobs it adds (finalized with the game's `finalizeSpawn`).
use super::{Dim, Game};
use horizon_client::vanillagen::VanillaTerrain;
use horizon_client::{
    biome::{BiomeInfo, SpawnEntry},
    blocks::{BlockFacts, Fluid},
    collisions::{SceneCollisions, no_collision_empty},
    mobs::{
        Mob, MobWorld, Species,
        data::{self, MobData},
        seen_grid::SeenGrid,
        spawn_1_8_9::{self as spawn18, Category as Category18, SpawnEntry18, SpawnLevel18},
        spawn_rules::{Dimension, SpawnLevel},
        spawn_selection::{
            self, Category, Counted, Heightmap, NaturalLevel, SpawnCost, SpawnPlayer, SpawnRequest,
            SpawnState, Spawned, StructureAt, StructureSpawns,
        },
    },
};
use horizon_core::{
    geometry::{Aabb, Vec3},
    random::{LegacyRandom, RandomSource, WorldgenRandom, XoroshiroRandom},
};
use horizon_world::{
    biome_spawns_1_8_9,
    nbt::{Compound, Tag},
    registry::BlockStateId,
};
use std::collections::HashMap;

/// Reused buffers for one pass over the spawning chunks: the candidate chunks
/// and the set that keeps each one once. The buffers keep their capacity
/// between ticks.
#[derive(Default)]
pub(super) struct SpawnScratch {
    pub(super) candidates: Vec<(i32, i32)>,
    pub(super) seen: horizon_client::fast_hash::FxHashSet<(i32, i32)>,
}

/// A player in the level in place, as spawning and despawning see it.
pub(super) struct LevelPlayer {
    pub(super) position: Vec3,
    pub(super) bounds: Aabb,
    pub(super) alive: bool,
    pub(super) creative: bool,
    pub(super) spectator: bool,
}
impl LevelPlayer {
    fn distance_sqr(&self, x: f64, y: f64, z: f64) -> f64 {
        (self.position.x - x).powi(2)
            + (self.position.y - y).powi(2)
            + (self.position.z - z).powi(2)
    }
    /// `Level.hasNearbyAlivePlayer`'s test of one player (any distance for a
    /// negative range).
    fn alive_near(&self, x: f64, y: f64, z: f64, range: f64) -> bool {
        self.alive && !self.spectator && (range < 0.0 || self.distance_sqr(x, y, z) < range * range)
    }
}
/// `ChunkMap.euclideanDistanceSquared` below 128 blocks
/// (`playerIsCloseEnoughForSpawning`).
fn close_enough_for_spawning(p: &LevelPlayer, (cx, cz): (i32, i32)) -> bool {
    let (x, z) = (
        f64::from(cx * 16 + 8) - p.position.x,
        f64::from(cz * 16 + 8) - p.position.z,
    );
    !p.spectator && x * x + z * z < 16384.0
}
/// `EntityGetter.getNearestPlayer(x, y, z, -1, false)`: the nearest
/// non-spectator, alive or not.
pub(super) fn nearest_player(
    players: &[LevelPlayer],
    x: f64,
    y: f64,
    z: f64,
) -> Option<&LevelPlayer> {
    let mut best: Option<(&LevelPlayer, f64)> = None;
    for p in players.iter().filter(|p| !p.spectator) {
        let d = p.distance_sqr(x, y, z);
        if best.is_none_or(|(_, b)| d < b) {
            best = Some((p, d));
        }
    }
    best.map(|(p, _)| p)
}

/// The game level as natural spawning reads it.
pub(super) struct GameLevel<'g, 'a> {
    pub(super) game: &'g Game<'a>,
    pub(super) w: MobWorld<'g>,
    /// Every player in the level ([`Game::level_players`]).
    pub(super) players: Vec<LevelPlayer>,
    /// Where the non-spectators among them are (`getNearestPlayer`).
    pub(super) nearest: Vec<[f64; 3]>,
    /// Boxes of the mobs this pass created, not yet in the game's list.
    pub(super) pending: std::cell::RefCell<Vec<Aabb>>,
    /// The types and boxes of what a spawner created this tick.
    pub(super) pending_ids: std::cell::RefCell<Vec<(String, Aabb)>>,
    /// The game's mobs by box, built once a pass has asked more than
    /// [`SCANNED_QUERIES`] box queries (the mobs do not change while it
    /// holds the game); the first ones scan, as the build costs a few scans.
    pub(super) boxes: std::cell::OnceCell<MobBoxes>,
    pub(super) box_queries: std::cell::Cell<u32>,
}
const SCANNED_QUERIES: u32 = 8;
/// A mob not removed: its index in `Game::mobs`, box, alive, and whether
/// it is a living shulker.
type MobBox = (usize, Aabb, bool, bool);
fn mob_box(i: usize, m: &Mob) -> MobBox {
    let alive = m.alive();
    (
        i,
        m.bounds(),
        alive,
        alive && m.species() == Species::Shulker,
    )
}
/// The boxes of the game's mobs not removed, and a grid over them.
pub(super) struct MobBoxes {
    entries: Vec<MobBox>,
    grid: SeenGrid,
}
impl MobBoxes {
    fn new(mobs: &[Mob]) -> Self {
        let entries: Vec<_> = mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.removed)
            .map(|(i, m)| mob_box(i, m))
            .collect();
        let grid = SeenGrid::new(entries.iter().map(|e| e.1));
        Self { entries, grid }
    }
    /// The entries whose boxes meet `area` (and possibly a few more).
    fn near(&self, area: Aabb) -> impl Iterator<Item = MobBox> + '_ {
        let mut near = Vec::new();
        let indexed = self.grid.candidates(area, &mut near);
        let all = (!indexed).then(|| self.entries.iter());
        near.into_iter()
            .map(|i| self.entries[i as usize])
            .chain(all.into_iter().flatten().copied())
    }
}
impl GameLevel<'_, '_> {
    /// The mobs not removed whose boxes may meet `area` (callers test them).
    fn mobs_near(&self, area: Aabb) -> Box<dyn Iterator<Item = MobBox> + '_> {
        let asked = self.box_queries.get() + 1;
        self.box_queries.set(asked);
        if asked <= SCANNED_QUERIES && self.boxes.get().is_none() {
            return Box::new(
                self.game
                    .mobs
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| !m.removed)
                    .map(|(i, m)| mob_box(i, m)),
            );
        }
        Box::new(
            self.boxes
                .get_or_init(|| MobBoxes::new(&self.game.mobs))
                .near(area),
        )
    }
    fn biome(&self, x: i32, y: i32, z: i32) -> Option<&BiomeInfo> {
        let chunk = self.game.world.chunks.get(&(x >> 4, z >> 4))?;
        Some(world_biomes!(self.game).get(chunk.biome(x, y, z)))
    }
    /// The block whose noise cell `BiomeManager.getBiome` reads.
    fn zoomed(&self, x: i32, y: i32, z: i32) -> [i32; 3] {
        let zoom = self.game.streaming.biome_zoom;
        let [qx, qy, qz] = zoom.quart_at_block(horizon_core::position::BlockPos::new(x, y, z));
        [qx << 2, qy << 2, qz << 2]
    }
    fn zoomed_biome(&self, x: i32, y: i32, z: i32) -> Option<&BiomeInfo> {
        let [x, y, z] = self.zoomed(x, y, z);
        self.biome(x, y, z)
    }
    /// `EntityMob`'s constructor plus `canSpawn`, `method_2469`, `initialize`
    /// and the second `method_2469` of `MobSpawnerHelper.tickSpawners`. The
    /// mob is not added to the level when any check fails. Returns the
    /// entities `onInitialSpawn` spawned with it (the ocelot's kittens) in
    /// `extra`.
    fn build_mob_1_8_9(
        &self,
        entity: &str,
        position: [f64; 3],
        yaw: f32,
        world_random: &mut LegacyRandom,
        extra: &mut Vec<Mob>,
    ) -> Option<Mob> {
        let kind = self.game.res.mob_types().get(entity)?.clone();
        let [x, y, z] = position;
        let mut mob = Mob::new(0, kind, Vec3::new(x, y, z), yaw);
        let (bx, by, bz) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
        if !spawn18::can_spawn(self, entity, &mut mob.random, world_random, bx, by, bz, y) {
            return None;
        }
        if !spawn18::method_2469(self, entity, mob.bounds()) {
            return None;
        }
        let with = self.game.finalize_mob(&mut mob, world_random);
        if !spawn18::method_2469(self, entity, mob.bounds()) {
            return None;
        }
        extra.extend(with);
        Some(mob)
    }
}
impl SpawnLevel for GameLevel<'_, '_> {
    fn state(&self, x: i32, y: i32, z: i32) -> BlockStateId {
        self.w.state(x, y, z)
    }
    fn facts(&self) -> &BlockFacts {
        self.w.facts
    }
    fn mob_data(&self) -> &MobData {
        self.w.data
    }
    fn block_tag(&self, tag: &str, block: &str) -> bool {
        self.w.block_tag(tag, block)
    }
    // Level.getBiome: the zoomed biome.
    fn biome_tag(&self, tag: &str, x: i32, y: i32, z: i32) -> bool {
        let [x, y, z] = self.zoomed(x, y, z);
        self.w.biome_tag(tag, x, y, z)
    }
    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        self.w.sky_light(x, y, z)
    }
    fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        self.w.block_light(x, y, z)
    }
    fn sky_darken(&self) -> i32 {
        SpawnLevel::sky_darken(&self.w)
    }
    fn thundering(&self) -> bool {
        self.w.thundering
    }
    fn sea_level(&self) -> i32 {
        self.w.sea_level()
    }
    fn world_surface(&self, x: i32, z: i32) -> i32 {
        self.w.world_surface(x, z)
    }
    fn peaceful(&self) -> bool {
        self.w.peaceful()
    }
    fn dimension(&self) -> Dimension {
        self.w.dimension()
    }
    fn surface_slime_chance(&self, x: i32, y: i32, z: i32) -> f32 {
        self.w.surface_slime_chance(x, y, z)
    }
    fn world_seed(&self) -> i64 {
        self.w.world_seed
    }
    /// `getNearestPlayer(x, y, z, distance, true)`, over every player.
    fn player_near(&self, x: f64, y: f64, z: f64, distance: f64) -> bool {
        self.players.iter().any(|p| {
            p.alive && !p.creative && !p.spectator && p.distance_sqr(x, y, z) < distance * distance
        })
    }
}
impl spawn18::SpawnLevel18 for GameLevel<'_, '_> {
    fn state(&self, x: i32, y: i32, z: i32) -> BlockStateId {
        SpawnLevel::state(&self.w, x, y, z)
    }
    fn facts(&self) -> &BlockFacts {
        self.w.facts
    }
    fn prop(&self, x: i32, y: i32, z: i32, key: &str) -> Option<&str> {
        self.w.prop(x, y, z, key)
    }
    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        SpawnLevel::sky_light(&self.w, x, y, z)
    }
    fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        SpawnLevel::block_light(&self.w, x, y, z)
    }
    fn sky_darken(&self) -> i32 {
        let (rain, thunder) = self.game.world.weather.levels(1.0);
        spawn18::sky_darken(self.w.day_time, rain, thunder)
    }
    fn thundering(&self) -> bool {
        // `World.isThundering`: the thunder gradient (already scaled by the
        // rain gradient) is above 0.9, not the level's pending weather flag.
        self.game.world.weather.levels(1.0).1 > 0.9
    }
    fn nether(&self) -> bool {
        self.game.dim == Dim::Nether
    }
    fn peaceful(&self) -> bool {
        SpawnLevel::peaceful(&self.w)
    }
    fn sea_level(&self) -> i32 {
        SpawnLevel::sea_level(&self.w)
    }
    fn highest_block_y(&self, x: i32, z: i32) -> i32 {
        spawn18::highest_block_y(self, x, z)
    }
    fn is_flat(&self) -> bool {
        self.game.end.locator().is_some_and(|t| t.is_flat())
    }
    /// `World.getBiome(pos).biomeName`: the chunk's stored biome, without
    /// the noise-cell zoom the 26.3 rules read.
    fn biome_name(&self, x: i32, y: i32, z: i32) -> Option<&str> {
        self.biome(x, y, z).map(|b| b.name.as_str())
    }
    fn spawn_entries(
        &self,
        category: Category18,
        x: i32,
        y: i32,
        z: i32,
    ) -> &'static [SpawnEntry18] {
        // `ChunkProvider.getSpawnEntries`: the swamp hut's and the ocean
        // monument's monster lists replace the biome's list inside their
        // start's bounding box; only monsters are overridden.
        if category == Category18::Monster {
            let mut hut = false;
            let mut monument = false;
            for (id, _) in self.game.structures_reaching_where([x, y, z], &|id| {
                id == "minecraft:swamp_hut" || id == "minecraft:ocean_monument"
            }) {
                match id.as_str() {
                    "minecraft:swamp_hut" => hut = true,
                    "minecraft:ocean_monument" => monument = true,
                    _ => {}
                }
            }
            if hut {
                return biome_spawns_1_8_9::SWAMP_HUT_MONSTERS;
            }
            if monument {
                return biome_spawns_1_8_9::OCEAN_MONUMENT_MONSTERS;
            }
        }
        let Some(biome) = self.biome(x, y, z) else {
            return &[];
        };
        biome_spawns_1_8_9::for_name(&biome.name).map_or(&[], |list| list.lists[category.index()])
    }
    fn player_in_range(&self, x: f64, y: f64, z: f64, distance: f64) -> bool {
        self.players
            .iter()
            .any(|p| !p.spectator && p.distance_sqr(x, y, z) < distance * distance)
    }
    fn spawn_position(&self) -> [i32; 3] {
        self.game.level.spawn.map_or([0, 64, 0], |s| {
            [
                s[0].floor() as i32,
                s[1].floor() as i32,
                s[2].floor() as i32,
            ]
        })
    }
    /// `World.getPersistentEntityCount`: the loaded mobs of the category's
    /// 1.8 class that are not persistent. The 1.8 classes map onto the
    /// engine's modern categories.
    fn persistent_count(&self, category: Category18) -> i32 {
        self.game
            .mobs
            .iter()
            .filter(|m| {
                !m.removed && !m.persistent && category_of_1_8_9(&m.kind.category) == Some(category)
            })
            .count() as i32
    }
    fn game_time(&self) -> i64 {
        self.w.game_time
    }
    fn day_time(&self) -> i64 {
        self.w.day_time
    }
    fn chunk_random(&self, cx: i32, cz: i32, salt: i64) -> LegacyRandom {
        spawn18::chunk_random(self.w.world_seed, cx, cz, salt)
    }
    /// `WorldBorder.contains(BlockPos)`: the block cell overlaps the border's
    /// interior; the Y is not part of the test.
    fn border_contains_block(&self, x: i32, _y: i32, z: i32) -> bool {
        let [min_x, min_z, max_x, max_z] = self.game.border.bounds();
        f64::from(x) + 1.0 > min_x
            && f64::from(x) < max_x
            && f64::from(z) + 1.0 > min_z
            && f64::from(z) < max_z
    }
    /// `World.hasEntityIn(bounds, null) == false`: 1.8 only skips entities
    /// with `inanimate = true` (items, XP orbs, arrows, item frames and
    /// paintings), so living entities, players, armour stands, vehicles,
    /// falling blocks, TNT, end crystals and the dragon meet the box.
    fn has_inanimate_collision(&self, bounds: Aabb) -> bool {
        self.players.iter().any(|p| p.bounds.intersects(bounds))
            || self.pending.borrow().iter().any(|b| b.intersects(bounds))
            || self
                .mobs_near(bounds)
                .any(|(_, b, _, _)| b.intersects(bounds))
            || self
                .game
                .armor_stands
                .iter()
                .any(|a| a.bounds().intersects(bounds))
            || self
                .game
                .boats
                .iter()
                .any(|b| b.bounds().intersects(bounds))
            || self
                .game
                .carts
                .iter()
                .any(|c| c.bounds().intersects(bounds))
            || self.game.crystals.iter().any(|c| {
                Aabb::new(
                    Vec3::new(c.position.x - 1.0, c.position.y, c.position.z - 1.0),
                    Vec3::new(c.position.x + 1.0, c.position.y + 2.0, c.position.z + 1.0),
                )
                .intersects(bounds)
            })
            || self.game.tnts.iter().any(|t| t.bounds().intersects(bounds))
            || self
                .game
                .falling
                .iter()
                .any(|f| horizon_client::falling_block::bounds(f.position).intersects(bounds))
            || self
                .game
                .dragon
                .as_ref()
                .is_some_and(|d| d.alive() && (0..8).any(|i| d.part_box(i).intersects(bounds)))
    }
    /// `World.doesBoxCollide(entity, bounds).isEmpty()`.
    fn no_block_collision(&self, bounds: Aabb) -> bool {
        let mut scene = SceneCollisions {
            world: &self.game.world,
            binding: self.w.collision,
            subject: None,
        };
        no_collision_empty(&mut scene, bounds)
    }
    /// `World.containsFluid(bounds)`: any fluid block in the box's floor
    /// range, with no fluid-height test.
    fn contains_fluid(&self, bounds: Aabb) -> bool {
        let (min, max) = (bounds.min_position(), bounds.max_position());
        let (x0, x1) = (min.x.floor() as i32, max.x.floor() as i32);
        let (y0, y1) = (min.y.floor() as i32, max.y.floor() as i32);
        let (z0, z1) = (min.z.floor() as i32, max.z.floor() as i32);
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    if spawn18::SpawnLevel18::facts(self)
                        .get(spawn18::SpawnLevel18::state(self, x, y, z))
                        .fluid
                        != Fluid::None
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// The 1.8 `EntityCategory` an engine mob category belongs to.
fn category_of_1_8_9(category: &str) -> Option<Category18> {
    match category {
        "monster" => Some(Category18::Monster),
        "creature" => Some(Category18::Creature),
        "ambient" => Some(Category18::Ambient),
        "water_creature" => Some(Category18::Water),
        _ => None,
    }
}

/// `MobSpawnerHelper.tickSpawners` for one category and one chunk: three
/// attempts of up to four tries each. Returns the chunk's spawned count and
/// the debug count vanilla accumulates across the tries.
fn spawn_chunk_1_8_9(
    l: &GameLevel<'_, '_>,
    category: Category18,
    cx: i32,
    cz: i32,
    random: &mut LegacyRandom,
    pending: &mut Vec<Mob>,
) -> (i32, i32) {
    let [base_x, base_y, base_z] = spawn18::find_spawn_location(l, cx, cz, random);
    // A picked full-cube block skips the whole chunk.
    if spawn18::is_full_cube(
        spawn18::SpawnLevel18::facts(l),
        spawn18::SpawnLevel18::state(l, base_x, base_y, base_z),
    ) {
        return (0, 0);
    }
    let spawn = l.spawn_position();
    let mut t = 0;
    let mut o = 0;
    'attempts: for _ in 0..3 {
        let (mut x, mut y, mut z) = (base_x, base_y, base_z);
        // The entry is picked once per attempt, never per try.
        let mut entry: Option<&'static SpawnEntry18> = None;
        for _ in 0..4 {
            x += random.next_int_bound(6).unwrap_or(0) - random.next_int_bound(6).unwrap_or(0);
            y += random.next_int_bound(1).unwrap_or(0) - random.next_int_bound(1).unwrap_or(0);
            z += random.next_int_bound(6).unwrap_or(0) - random.next_int_bound(6).unwrap_or(0);
            let fx = f64::from(x) + 0.5;
            let fy = f64::from(y);
            let fz = f64::from(z) + 0.5;
            if l.player_in_range(fx, fy, fz, 24.0) {
                continue;
            }
            let (dx, dy, dz) = (
                fx - f64::from(spawn[0]),
                fy - f64::from(spawn[1]),
                fz - f64::from(spawn[2]),
            );
            if dx * dx + dy * dy + dz * dz < 576.0 {
                continue;
            }
            let picked = match entry {
                Some(e) => e,
                None => match spawn18::pick_entry(l, category, x, y, z, random) {
                    Some(e) => {
                        entry = Some(e);
                        e
                    }
                    // No entry at the first try abandons the attempt.
                    None => continue 'attempts,
                },
            };
            // `World.method_10753`: the drifted position must still list the
            // picked entry.
            if !l.spawn_entries(category, x, y, z).contains(picked) {
                continue;
            }
            if !spawn18::is_spawnable(l, spawn18::location_of(picked.entity), x, y, z) {
                continue;
            }
            let yaw = random.next_float() * 360.0;
            let mut kittens = Vec::new();
            let Some(mob) =
                l.build_mob_1_8_9(picked.entity, [fx, fy, fz], yaw, random, &mut kittens)
            else {
                // The failed try still contributes the running count below,
                // as vanilla's `o += t` does.
                o += t;
                continue;
            };
            let bounds = mob.bounds();
            l.pending.borrow_mut().push(bounds);
            pending.push(mob);
            // `onInitialSpawn` spawned them with the mob, at its position.
            for kitten in kittens {
                l.pending.borrow_mut().push(kitten.bounds());
                pending.push(kitten);
            }
            t += 1;
            if t >= spawn18::limit_per_chunk(picked.entity) {
                // `continue block6`: the next chunk, without the final
                // `o += t`.
                break 'attempts;
            }
            o += t;
        }
    }
    (t, o)
}

impl NaturalLevel for GameLevel<'_, '_> {
    fn min_y(&self) -> i32 {
        self.w.world.bottom_y()
    }
    fn max_y(&self) -> i32 {
        self.w.world.top_y() - 1
    }
    fn ambient_light(&self) -> f32 {
        match self.game.dim {
            Dim::Nether => 0.1,
            Dim::End => 0.25,
            _ => 0.0,
        }
    }
    fn has_ceiling(&self) -> bool {
        self.game.dim == Dim::Nether
    }
    fn mob_type(&self, entity: &str) -> Option<(Species, f32, f32, Category)> {
        let t = self.game.res.mob_types().get(entity)?;
        Some((
            t.species,
            t.width as f32,
            t.height as f32,
            Category::from_name(&t.category),
        ))
    }
    // The `natural_mob_spawns` attribute reads the zoomed biome
    // (fullResolutionBiomes), the spawn probability the noise cell.
    fn biome_spawns(&self, x: i32, y: i32, z: i32) -> Option<&HashMap<String, Vec<SpawnEntry>>> {
        self.zoomed_biome(x, y, z).map(|b| &b.spawns)
    }
    fn spawn_cost(&self, entity: &str, x: i32, y: i32, z: i32) -> Option<SpawnCost> {
        self.zoomed_biome(x, y, z)?.spawn_costs.get(entity).copied()
    }
    fn creature_probability(&self, x: i32, y: i32, z: i32) -> f32 {
        self.biome(x, y, z).map_or(0.1, |b| b.creature_probability)
    }
    fn structures_at(
        &self,
        x: i32,
        y: i32,
        z: i32,
        wanted: &dyn Fn(&str) -> bool,
    ) -> Vec<StructureAt> {
        self.game
            .structures_reaching_where([x, y, z], wanted)
            .into_iter()
            .map(|(id, in_piece)| StructureAt {
                id,
                in_piece,
                in_bounds: true,
            })
            .collect()
    }
    fn structure_spawns(&self) -> &StructureSpawns {
        StructureSpawns::shared(&self.game.options.resources)
    }
    fn nearest_player_positions(&self) -> &[[f64; 3]] {
        &self.nearest
    }
    fn respawn(&self) -> Option<[i32; 3]> {
        // The world spawn point lives in the overworld.
        if self.game.dim != Dim::Overworld {
            return None;
        }
        self.game.level.spawn.map(|s| {
            [
                s[0].floor() as i32,
                s[1].floor() as i32,
                s[2].floor() as i32,
            ]
        })
    }
    fn can_spawn_in_chunk(&self, cx: i32, cz: i32) -> bool {
        self.game.world.chunks.contains_key(&(cx, cz))
    }
    fn no_collision(&self, bounds: Aabb) -> bool {
        let mut scene = SceneCollisions {
            world: &self.game.world,
            binding: self.w.collision,
            subject: None,
        };
        // getEntityCollisions(null, box): living shulkers are solid.
        let grown = bounds.inflate(Vec3::new(1e-7, 1e-7, 1e-7));
        no_collision_empty(&mut scene, bounds)
            && !self
                .mobs_near(grown)
                .any(|(_, b, _, shulker)| shulker && b.intersects(grown))
            && !self
                .pending_ids
                .borrow()
                .iter()
                .any(|(id, b)| id == "minecraft:shulker" && b.intersects(grown))
    }
    fn unobstructed(&self, bounds: Aabb) -> bool {
        let player = self
            .players
            .iter()
            .any(|p| p.alive && !p.spectator && p.bounds.intersects(bounds));
        !player
            && !self.pending.borrow().iter().any(|b| b.intersects(bounds))
            && !self
                .mobs_near(bounds)
                .any(|(_, b, alive, _)| alive && b.intersects(bounds))
    }
    fn spawn_mobs(&self) -> bool {
        self.game.ui.rules.bool("spawn_mobs")
    }
    /// WORLD_SURFACE (every spawn attempt's) is the chunk's kept heightmap.
    /// The others are scanned down the column: sections of air alone are
    /// skipped (air counts for no heightmap). An unloaded column reads as air.
    fn heightmap(&self, kind: Heightmap, x: i32, z: i32) -> i32 {
        let world = self.w.world;
        if kind == Heightmap::WorldSurface {
            return self.w.world_surface(x, z);
        }
        world
            .chunks
            .get(&(x >> 4, z >> 4))
            .and_then(|chunk| {
                chunk.highest(world.air.0, |y| {
                    spawn_selection::heightmap_blocks(self, kind, x, y, z)
                })
            })
            .map_or(self.w.world.bottom_y(), |y| y + 1)
    }
    fn pathfindable_land(&self, x: i32, y: i32, z: i32) -> bool {
        self.w.data.state(SpawnLevel::state(self, x, y, z)).flags & data::LAND != 0
    }
}
impl horizon_client::mobs::spawner::SpawnerLevel for GameLevel<'_, '_> {
    /// `Level.hasNearbyAlivePlayer`, over every player.
    fn alive_player_near(&self, x: f64, y: f64, z: f64, range: f64) -> bool {
        self.players.iter().any(|p| p.alive_near(x, y, z, range))
    }
    fn spawner_blocks_work(&self) -> bool {
        self.game.ui.rules.bool("spawner_blocks_work")
    }
    fn entity_exists(&self, id: &str) -> bool {
        self.game.res.mob_types().contains_key(id) || self.game.entity_type_exists(id)
    }
    fn count_entities(&self, id: &str, bounds: Aabb) -> usize {
        let mobs = self
            .mobs_near(bounds)
            .filter(|&(i, b, ..)| self.game.mobs[i].kind.id == id && b.intersects(bounds))
            .count();
        let pending = self
            .pending_ids
            .borrow()
            .iter()
            .filter(|(k, b)| k == id && b.intersects(bounds))
            .count();
        mobs + pending
    }
}
impl<'a> Game<'a> {
    pub(super) fn natural_level(&self) -> GameLevel<'_, 'a> {
        let players = self.level_players();
        let nearest = players
            .iter()
            .filter(|p| !p.spectator)
            .map(|p| [p.position.x, p.position.y, p.position.z])
            .collect();
        GameLevel {
            game: self,
            w: self.mob_world_base(&[]),
            players,
            nearest,
            pending: Default::default(),
            pending_ids: Default::default(),
            boxes: Default::default(),
            box_queries: Default::default(),
        }
    }
    /// The game's `finalizeSpawn` for a mob the spawner created: biome facts
    /// at its feet and the local difficulty. 1.8's `onInitialSpawn` also
    /// reads the level's random (a sheep's fleece colour and a villager's
    /// profession), can turn the mob into another species, and can spawn
    /// entities with it (the ocelot's kittens), which are returned.
    fn finalize_mob(&self, mob: &mut Mob, world_random: &mut impl RandomSource) -> Vec<Mob> {
        let p = mob.position;
        let difficulty = self.local_difficulty([p.x, p.y, p.z]);
        if horizon_core::target::is_1_8_9() {
            let out = horizon_client::mobs::finalize_1_8_9::finalize(
                mob,
                world_random,
                &difficulty,
                self.dim == Dim::Nether,
            );
            if let Some(id) = out.convert
                && let Some(kind) = self.res.mob_types().get(id)
            {
                mob.kind = kind.clone();
                // `EntityHorse` became a donkey: its own jump strength,
                // health and 0.175 speed follow from the new type.
                if mob.species() == Species::Donkey {
                    horizon_client::mobs::equine::finalize(mob);
                    horizon_client::mobs::finalize::set_base(
                        mob,
                        horizon_client::mobs::finalize::Attr::MovementSpeed,
                        0.175,
                    );
                }
            }
            return out.extra;
        }
        let facts = self.biome_facts(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        horizon_client::mobs::spawning::finalize(mob, &facts, &difficulty);
        Vec::new()
    }
    /// Creates a mob the spawner accepted, finalized the game's way; the
    /// caller adds it with [`Game::add_spawned`].
    fn finalize_spawned(&self, r: &SpawnRequest, pending: &mut Vec<Mob>) -> Option<Spawned> {
        let kind = self.res.mob_types().get(r.entity)?.clone();
        let [x, y, z] = r.position;
        let mut mob = Mob::new(0, kind, Vec3::new(x, y, z), r.yaw);
        // 26.3's finalize draws from the mob's own random only.
        let _ = self.finalize_mob(&mut mob, &mut LegacyRandom::new(0));
        horizon_client::mobs::raider::finalize(
            &mut mob,
            horizon_client::mobs::raider::Reason::Natural,
        );
        let bounds = mob.bounds();
        pending.push(mob);
        Some(Spawned {
            bounds,
            school: true,
        })
    }
    /// Adds the mobs a spawn pass created: ids, enchanted gear and jockeys.
    fn add_spawned(&mut self, pending: Vec<Mob>, persistent: bool) {
        for mut mob in pending {
            self.next_entity += 1;
            mob.id = self.next_entity;
            mob.persistent = persistent;
            let id = mob.id;
            self.mobs.push(mob);
            self.native_mobs_joined(self.mobs.len() - 1, true);
            self.enchant_spawn_equipment(id);
            // 1.8 rolls the pumpkin at the end of `onInitialSpawn`.
            if !horizon_core::target::is_1_8_9() {
                self.halloween_spawn_head(id);
            }
            if !persistent {
                self.spawn_jockeys(id, true);
            }
        }
    }
    /// Every player in the level in place: the local player, and the LAN
    /// guests or server players in it.
    pub(super) fn level_players(&self) -> Vec<LevelPlayer> {
        self.level_players_iter().collect()
    }
    /// `level_players` one at a time, without collecting them.
    fn level_players_iter(&self) -> impl Iterator<Item = LevelPlayer> + '_ {
        let local = self.player_view().map(|p| LevelPlayer {
            position: p.position,
            bounds: p.bounds,
            alive: p.alive,
            creative: p.creative,
            spectator: p.spectator,
        });
        let guests = self.lan.iter().flat_map(|h| {
            h.hub
                .guests
                .iter()
                .filter(|g| h.hub.here(g))
                .map(|g| LevelPlayer {
                    position: Vec3::new(g.position[0], g.position[1], g.position[2]),
                    bounds: super::lan_mobs::guest_bounds(g),
                    alive: !g.dead,
                    creative: g.creative(),
                    spectator: g.spectator(),
                })
        });
        local.into_iter().chain(guests)
    }
    /// `Level.hasNearbyAlivePlayer` before a level is built for the spawn
    /// code: what a spawner asks first, every tick.
    pub(super) fn alive_player_near(&self, x: f64, y: f64, z: f64, range: f64) -> bool {
        self.level_players_iter()
            .any(|p| p.alive_near(x, y, z, range))
    }
    /// `ChunkMap.collectSpawningChunks`: the natural spawn chunk tracker's
    /// chunks (within 8 chunks of a player, `getSpawnCandidateChunks`) that
    /// are ticking and within 128 blocks of a non-spectator. Also returns how
    /// many chunks the tracker holds (`getNaturalSpawnChunkCount`).
    pub(super) fn spawning_chunks(
        &self,
        players: &[LevelPlayer],
        scratch: &mut SpawnScratch,
    ) -> i32 {
        let candidates = &mut scratch.candidates;
        candidates.clear();
        candidates.reserve(289 * players.len());
        let seen = &mut scratch.seen;
        seen.clear();
        seen.reserve(289 * players.len());
        for p in players {
            let (pcx, pcz) = (
                p.position.x.floor() as i32 >> 4,
                p.position.z.floor() as i32 >> 4,
            );
            for dx in -8..=8 {
                for dz in -8..=8 {
                    let key = (pcx + dx, pcz + dz);
                    if seen.insert(key) {
                        candidates.push(key);
                    }
                }
            }
        }
        let count = candidates.len() as i32;
        let levels = self.tickets.levels();
        candidates.retain(|&(cx, cz)| {
            levels.loaded_ticking(cx, cz)
                && super::ticking_chunk(&self.world, cx, cz)
                && players
                    .iter()
                    .any(|p| close_enough_for_spawning(p, (cx, cz)))
        });
        count
    }
    /// `MobSpawnerHelper.tickSpawners`' chunk gathering: every non-spectator
    /// player's 17 by 17 chunks. The border chunks and the chunks outside the
    /// world border are counted (`i`, the cap's denominator) but never
    /// spawned in, and the set only deduplicates the accepted ones.
    pub(super) fn spawning_chunks_1_8_9(
        &self,
        players: &[LevelPlayer],
        scratch: &mut SpawnScratch,
    ) -> i32 {
        let candidates = &mut scratch.candidates;
        candidates.clear();
        let seen = &mut scratch.seen;
        seen.clear();
        let [min_x, min_z, max_x, max_z] = self.border.bounds();
        let mut counted = 0;
        for p in players.iter().filter(|p| !p.spectator) {
            let (pcx, pcz) = (
                p.position.x.floor() as i32 >> 4,
                p.position.z.floor() as i32 >> 4,
            );
            for dx in -8..=8 {
                for dz in -8..=8 {
                    let key = (pcx + dx, pcz + dz);
                    if seen.contains(&key) {
                        continue;
                    }
                    counted += 1;
                    let edge = dx == -8 || dx == 8 || dz == -8 || dz == 8;
                    let (x0, z0) = (key.0 * 16, key.1 * 16);
                    let inside = f64::from(x0 + 15) > min_x
                        && f64::from(x0) < max_x
                        && f64::from(z0 + 15) > min_z
                        && f64::from(z0) < max_z;
                    if edge || !inside {
                        continue;
                    }
                    seen.insert(key);
                    candidates.push(key);
                }
            }
        }
        // `World.getChunk` loads a missing chunk; this engine only reads the
        // chunks already loaded.
        candidates.retain(|&(cx, cz)| self.world.chunks.contains_key(&(cx, cz)));
        counted
    }
    /// `MobSpawnerHelper.tickSpawners` against the 1.8 rules: the category
    /// caps over the loaded mobs, then one chunk at a time.
    pub(super) fn spawn_natural_1_8_9(&mut self) {
        // `ServerWorld.tick` skips a debug world and `doMobSpawning == false`.
        if self.world_debug || !self.ui.rules.bool("spawn_mobs") {
            return;
        }
        let players = self.level_players();
        if players.is_empty() {
            return;
        }
        let mut scratch = std::mem::take(&mut self.spawn_scratch);
        let counted = self.spawning_chunks_1_8_9(&players, &mut scratch);
        let mut random = std::mem::replace(&mut self.world.random, LegacyRandom::new(0));
        let mut pending = Vec::new();
        let mut counts = [0i32; 8];
        {
            let level = self.natural_level();
            // Single-player `ServerWorld.tick`: `spawnAnimals` is
            // difficulty != PEACEFUL, `spawnMonsters` is always true, and
            // the breedable gate is `world time % 400 == 0`.
            let spawn_animals = !spawn18::SpawnLevel18::peaceful(&level);
            let breedable = level.game_time() % 400 == 0;
            for category in Category18::ALL {
                if (!category.hostile() && !spawn_animals) || (category.breedable() && !breedable) {
                    continue;
                }
                let n = level.persistent_count(category);
                if n > category.cap() * counted / 289 {
                    continue;
                }
                let slot = match category {
                    Category18::Monster => 0,
                    Category18::Creature => 1,
                    Category18::Ambient => 2,
                    Category18::Water => 5,
                };
                counts[slot] = n;
                for &(cx, cz) in &scratch.candidates {
                    spawn_chunk_1_8_9(&level, category, cx, cz, &mut random, &mut pending);
                }
            }
        }
        self.world.random = random;
        self.spawn_counts.insert(self.dim, (counted, counts));
        self.add_spawned(pending, false);
        self.spawn_scratch = scratch;
    }
    /// `ServerChunkCache.tickChunks`: the spawn state over the loaded mobs,
    /// the categories under their global caps (creatures every 400 ticks),
    /// then `tickSpawningChunk` in the shuffled spawning chunks of every
    /// player: inhabited time, then `spawnForChunk`.
    pub(super) fn spawn_natural(&mut self) {
        if horizon_core::target::is_1_8_9() {
            return self.spawn_natural_1_8_9();
        }
        // `tickChunks` does nothing in a Debug world.
        if self.world_debug {
            return;
        }
        let players = self.level_players();
        if players.is_empty() {
            return;
        }
        let mut scratch = std::mem::take(&mut self.spawn_scratch);
        let spawnable = self.spawning_chunks(&players, &mut scratch);
        for &key in &scratch.candidates {
            *self.world.inhabited.entry(key).or_insert(0) += 1;
        }
        let spawn_mobs = self.ui.rules.bool("spawn_mobs");
        let spawn_enemies = self.ui.rules.bool("spawn_monsters");
        let mut random = std::mem::replace(&mut self.world.random, LegacyRandom::new(0));
        let mut pending = Vec::new();
        let last_state;
        {
            let level = self.natural_level();
            let counted: Vec<Counted> = self
                .mobs
                .iter()
                .map(|m| Counted {
                    entity: &m.kind.id,
                    category: Category::from_name(&m.kind.category),
                    block: [
                        m.position.x.floor() as i32,
                        m.position.y.floor() as i32,
                        m.position.z.floor() as i32,
                    ],
                    is_mob: true,
                    persistent: m.persistent
                        || horizon_client::mobs::ai::requires_custom_persistence(m),
                })
                .collect();
            let players = players
                .iter()
                .map(|p| SpawnPlayer {
                    position: [p.position.x, p.position.y, p.position.z],
                    spectator: p.spectator,
                })
                .collect();
            let mut state = SpawnState::new(&level, spawnable, players, &counted, |cx, cz| {
                self.world.chunks.contains_key(&(cx, cz))
            });
            last_state = state.counts();
            let categories = if spawn_mobs {
                state.filtered_categories(spawn_enemies, self.world.game_time % 400 == 0)
            } else {
                Vec::new()
            };
            // Util.shuffle.
            for i in (1..scratch.candidates.len()).rev() {
                let j = random.next_int_bound(i as i32 + 1).unwrap_or(0) as usize;
                scratch.candidates.swap(i, j);
            }
            if !categories.is_empty() {
                for chunk in scratch.candidates.drain(..) {
                    let mut spawn = |l: &GameLevel, r: &SpawnRequest| {
                        let spawned = l.game.finalize_spawned(r, &mut pending);
                        if let Some(s) = &spawned {
                            l.pending.borrow_mut().push(s.bounds);
                        }
                        spawned
                    };
                    spawn_selection::spawn_for_chunk(
                        &level,
                        &mut random,
                        &mut state,
                        &categories,
                        chunk,
                        &mut spawn,
                        &mut |_| {},
                    );
                }
            }
        }
        self.world.random = random;
        // ServerChunkCache.lastSpawnState, for the debug overlay.
        self.spawn_counts.insert(self.dim, last_state);
        self.add_spawned(pending, false);
        self.spawn_scratch = scratch;
    }
    /// `NoiseBasedChunkGenerator.spawnOriginalMobs`: creature packs for a
    /// freshly generated chunk, from the biome at its corner column's top,
    /// with the chunk's decoration-seeded random, finalized with the
    /// region's random as generation adds them (mobs are not generated in
    /// the End).
    pub(super) fn spawn_generation_creatures(&mut self, cx: i32, cz: i32) {
        if horizon_core::target::is_1_8_9() {
            return self.spawn_generation_creatures_1_8_9(cx, cz);
        }
        if !self.world.chunks.contains_key(&(cx, cz)) || self.dim == Dim::End {
            return;
        }
        let Some(terrain) = self.end.locator() else {
            return;
        };
        // `FlatLevelSource.spawnOriginalMobs` is empty: a superflat world places no packs.
        if terrain.is_flat() {
            return;
        }
        let mut region = terrain.region_random(cx, cz);
        let any = region.as_any();
        let added = if let Some(r) = any.downcast_mut::<LegacyRandom>() {
            self.generation_creatures(cx, cz, &terrain, r)
        } else if let Some(r) = any.downcast_mut::<XoroshiroRandom>() {
            self.generation_creatures(cx, cz, &terrain, r)
        } else {
            return;
        };
        for nbt in added {
            self.load_mob(Tag::Compound(nbt), None);
        }
    }
    fn generation_creatures<Q: RandomSource + 'static>(
        &self,
        cx: i32,
        cz: i32,
        terrain: &VanillaTerrain,
        region: &mut Q,
    ) -> Vec<Compound> {
        let mut random = WorldgenRandom::new(LegacyRandom::new(0));
        random.set_decoration_seed(self.level.seed, cx * 16, cz * 16);
        let mut added = Vec::new();
        let mut group = None;
        let level = self.natural_level();
        let source = [cx * 16, level.max_y(), cz * 16];
        let mut spawn = |l: &GameLevel, r: &SpawnRequest, region: &mut Q| {
            // Each pack starts without group data.
            if r.group_index == 0 {
                group = None;
            }
            let [x, y, z] = r.position;
            let (bx, by, bz) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
            let biome = l.zoomed_biome(bx, by, bz).map_or("", |b| b.name.as_str());
            let at = Vec3::new(x, y, z);
            match terrain.spawn_generated(r.entity, at, r.yaw, biome, region, &mut group) {
                Some(nbt) => {
                    added.push(nbt);
                    true
                }
                None => false,
            }
        };
        spawn_selection::spawn_mobs_for_chunk_generation(
            &level,
            source,
            (cx, cz),
            &mut random,
            region,
            &mut spawn,
            &mut |_| {},
        );
        added
    }
    /// `SurfaceChunkGenerator.decorateChunk`'s `MobSpawnerHelper.spawnMobs`:
    /// the passive pack a freshly generated overworld chunk gets. 1.8 seeds
    /// the provider's random with the chunk coordinates (unlike 26.3, which
    /// seeds with the chunk's first block), picks from the biome at the
    /// chunk's far corner, and finalizes every mob with the world random.
    fn spawn_generation_creatures_1_8_9(&mut self, cx: i32, cz: i32) {
        // The nether and End generators have no `spawnMobs` call, and the
        // flat and debug generators decorate with no packs.
        if self.dim != Dim::Overworld || self.world_debug {
            return;
        }
        if !self.world.chunks.contains_key(&(cx, cz)) {
            return;
        }
        let Some(terrain) = self.end.locator() else {
            return;
        };
        if terrain.is_flat() {
            return;
        }
        let mut random = WorldgenRandom::new(LegacyRandom::new(0));
        random.set_decoration_seed_1_8_9(self.level.seed, cx, cz);
        let mut world_random = std::mem::replace(&mut self.world.random, LegacyRandom::new(0));
        let mut pending = Vec::new();
        {
            let level = self.natural_level();
            let mut spawn = |_: &GameLevel,
                             entity: &'static str,
                             position: [f64; 3],
                             yaw: f32,
                             world_random: &mut LegacyRandom| {
                let Some(kind) = self.res.mob_types().get(entity).cloned() else {
                    return false;
                };
                let mut mob = Mob::new(
                    0,
                    kind,
                    Vec3::new(position[0], position[1], position[2]),
                    yaw,
                );
                let kittens = self.finalize_mob(&mut mob, world_random);
                pending.push(mob);
                pending.extend(kittens);
                true
            };
            spawn18::spawn_mobs_for_chunk_generation(
                &level,
                cx,
                cz,
                &mut random,
                &mut world_random,
                &mut spawn,
                &mut |_| {},
            );
        }
        self.world.random = world_random;
        self.add_spawned(pending, false);
    }
}
