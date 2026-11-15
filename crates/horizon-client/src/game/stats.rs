//! Player statistics (`ServerStatsCounter`): the custom counters, blocks
//! mined, items used/crafted/broken/picked up/dropped and mobs killed or
//! killed by, saved to the target's stats file (`stats/<uuid>.json` in 1.8,
//! `players/stats/<uuid>.json` in 26.3), plus the phantom spawner that reads
//! the time since rest (`PhantomSpawner`).
use super::{Dim, Game};
use horizon_core::geometry::Vec3;
use horizon_core::random::RandomSource;
use horizon_world::nbt::Tag;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// `stats[type][key]`, with `type` and `key` namespaced ids.
#[derive(Default)]
pub(super) struct Stats {
    pub(super) values: BTreeMap<String, BTreeMap<String, i32>>,
    pub(super) loaded: bool,
    /// Ticks until the phantom spawner runs again.
    phantom_tick: i32,
    /// Statistic changes (type, key, amount; `None` = reset) for the
    /// scoreboard criteria to follow.
    pub(super) events: Vec<(String, String, Option<i32>)>,
    /// Changed since the client was last sent them (`StatsCounter`'s dirty
    /// set), by type.
    dirty: BTreeMap<String, BTreeSet<String>>,
}

/// `minecraft:` and `name`, given to `f` from the stack when short: some
/// counters go up every tick for every player, and they are found without
/// a string made for each.
fn namespaced<T>(name: &str, f: impl FnOnce(&str) -> T) -> T {
    const NAMESPACE: &str = "minecraft:";
    let mut buf = [0u8; 96];
    let len = NAMESPACE.len() + name.len();
    if len > buf.len() {
        return f(&format!("{NAMESPACE}{name}"));
    }
    buf[..NAMESPACE.len()].copy_from_slice(NAMESPACE.as_bytes());
    buf[NAMESPACE.len()..len].copy_from_slice(name.as_bytes());
    // Two strings one after the other are one.
    f(std::str::from_utf8(&buf[..len]).unwrap_or_default())
}

/// The map under `key`, made when missing, found without an owned key
/// when there.
fn entry<'a, V: Default>(map: &'a mut BTreeMap<String, V>, key: &str) -> &'a mut V {
    if !map.contains_key(key) {
        map.insert(key.to_owned(), V::default());
    }
    map.get_mut(key).expect("just inserted")
}

impl Stats {
    pub(super) fn add(&mut self, ty: &str, key: &str, n: i32) {
        let key = counter_key(ty, key);
        namespaced(ty, |ty| {
            let v = entry(entry(&mut self.values, ty), &key);
            *v = v.saturating_add(n);
            self.events.push((ty.to_owned(), key.clone(), Some(n)));
            self.mark_dirty(ty, &key);
        });
    }

    fn mark_dirty(&mut self, ty: &str, key: &str) {
        let keys = entry(&mut self.dirty, ty);
        if !keys.contains(key) {
            keys.insert(key.to_owned());
        }
    }
    pub(super) fn get(&self, ty: &str, key: &str) -> i32 {
        self.values
            .get(&format!("minecraft:{ty}"))
            .and_then(|m| m.get(key))
            .copied()
            .unwrap_or(0)
    }
    /// A 1.8 counter for a legacy item id. The screen's rows are one per id,
    /// so every shape of the id reads the row's canonical item.
    pub(super) fn get_legacy_item(&self, ty: &str, id: i16) -> i32 {
        let Some(key) = counter_key_for_id(id) else {
            return 0;
        };
        self.get(ty, &key)
    }
    pub(super) fn reset(&mut self, ty: &str, key: &str) {
        if let Some(m) = self.values.get_mut(&format!("minecraft:{ty}")) {
            m.insert(key.to_owned(), 0);
        }
        self.mark_dirty(&format!("minecraft:{ty}"), key);
        self.events
            .push((format!("minecraft:{ty}"), key.to_owned(), None));
    }
    pub(super) fn custom(&mut self, key: &str, n: i32) {
        namespaced(key, |key| self.add("custom", key, n));
    }
    /// `ServerStatsCounter.markAllDirty`: everything goes out on the next request.
    pub(super) fn mark_all_dirty(&mut self) {
        self.dirty = self
            .values
            .iter()
            .map(|(ty, m)| (ty.clone(), m.keys().cloned().collect()))
            .collect();
    }
    /// `ServerStatsCounter.sendStats`' entries: the dirty statistics and
    /// their values, the set cleared.
    pub(super) fn take_dirty(&mut self) -> Vec<(String, String, i32)> {
        std::mem::take(&mut self.dirty)
            .into_iter()
            .flat_map(|(ty, keys)| keys.into_iter().map(move |key| (ty.clone(), key)))
            .map(|(ty, key)| {
                let v = self.values.get(&ty).and_then(|m| m.get(&key)).copied();
                (ty, key, v.unwrap_or(0))
            })
            .collect()
    }
    /// `ServerStatsCounter.parseLocal`: the counters of a stats file.
    pub(super) fn read(path: &Path) -> Self {
        let mut stats = Self {
            loaded: true,
            ..Self::default()
        };
        let Some(json) = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        else {
            return stats;
        };
        if horizon_core::target::is_1_8_9() {
            // `ServerStatHandler.method_8271`: one flat object of
            // `Stat.name` → value; unknown ids (achievements and stats from
            // other versions) are skipped.
            let Some(all) = json.as_object() else {
                return stats;
            };
            for (name, v) in all {
                let Some(n) = v.as_i64() else {
                    continue;
                };
                let Some((ty, key)) = stat_1_8_9(name) else {
                    continue;
                };
                stats
                    .values
                    .entry(ty.to_owned())
                    .or_default()
                    .insert(key, n as i32);
            }
            return stats;
        }
        if let Some(all) = json.get("stats").and_then(|s| s.as_object()) {
            for (ty, entries) in all {
                let Some(entries) = entries.as_object() else {
                    continue;
                };
                let map = stats.values.entry(ty.clone()).or_default();
                for (key, v) in entries {
                    if let Some(n) = v.as_i64() {
                        map.insert(key.clone(), n as i32);
                    }
                }
            }
        }
        stats
    }
    /// `ServerStatsCounter.save`.
    pub(super) fn write(&self, path: &Path) {
        let json = if horizon_core::target::is_1_8_9() {
            // `ServerStatHandler.method_8272`: a flat `Stat.name` → value
            // object, no wrapper and no DataVersion. A statistic 1.8 does not
            // know (`total_world_time`, ...) is not written.
            let entries: serde_json::Map<String, serde_json::Value> = self
                .values
                .iter()
                .flat_map(|(ty, m)| {
                    m.iter()
                        .filter_map(move |(k, v)| Some((legacy_stat_name(ty, k)?, (*v).into())))
                })
                .collect();
            serde_json::Value::Object(entries)
        } else {
            let stats: serde_json::Map<String, serde_json::Value> = self
                .values
                .iter()
                .filter(|(_, m)| !m.is_empty())
                .map(|(ty, m)| {
                    let entries: serde_json::Map<String, serde_json::Value> =
                        m.iter().map(|(k, v)| (k.clone(), (*v).into())).collect();
                    (ty.clone(), serde_json::Value::Object(entries))
                })
                .collect();
            serde_json::json!({
                "stats": stats,
                "DataVersion": horizon_world::DATA_VERSION,
            })
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(&json) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// Whether a statistic type counts item shapes (1.8 keeps one row per numeric
/// item id).
fn is_item_counter(ty: &str) -> bool {
    matches!(
        ty.strip_prefix("minecraft:").unwrap_or(ty),
        "mined" | "used" | "crafted" | "broken" | "picked_up" | "dropped"
    )
}

/// The key a counter is stored under. 1.8 counts one statistic per item id,
/// not per flattened variant, so every wool colour, plank species and slab
/// shares the row of the id's first shape: the id is turned back into the
/// current item it names (`minecraft:red_wool` → `minecraft:white_wool`).
fn counter_key(ty: &str, key: &str) -> String {
    if !horizon_core::target::is_1_8_9() || !is_item_counter(ty) {
        return key.to_owned();
    }
    let Some((id, _)) = horizon_items::legacy_1_8_9::legacy_id(key) else {
        return key.to_owned();
    };
    counter_key_for_id(id).unwrap_or_else(|| key.to_owned())
}

/// The canonical item a 1.8 item id's statistics row reads (`2|60 => 3`:
/// grass and farmland count as dirt).
fn counter_key_for_id(id: i16) -> Option<String> {
    horizon_items::legacy_1_8_9::legacy_item_name(canonical_legacy_id(id)).map(str::to_owned)
}

/// 1.8 `Stats.method_8293`: each of these ids shares the statistic of the
/// second one, named after it, so 1.8 counts grass as dirt and a jack
/// o'lantern as a pumpkin.
pub(super) fn canonical_legacy_id(id: i16) -> i16 {
    match id {
        2 | 60 => 3, // grass, farmland -> dirt
        8 => 9,      // flowing water -> water
        10 => 11,    // flowing lava -> lava
        43 => 44,    // double stone slab -> stone slab
        62 => 61,    // lit furnace -> furnace
        74 => 73,    // lit redstone ore -> redstone ore
        // The unlit redstone torch (75) has no item, so `method_8294` moves
        // its statistics to the lit torch (76) instead of removing the row.
        75 => 76,   // unlit redstone torch -> redstone torch
        91 => 86,   // jack o'lantern -> pumpkin
        94 => 93,   // powered repeater -> repeater
        124 => 123, // lit redstone lamp -> redstone lamp
        125 => 126, // double wooden slab -> wooden slab
        150 => 149, // powered comparator -> comparator
        181 => 182, // double stone slab 2 -> stone slab 2
        _ => id,
    }
}

/// The 1.8 `Stat.name` and current key of every `SimpleStat`, in
/// `Stats.GENERAL` order.
const CUSTOM_1_8_9: &[(&str, &str)] = &[
    ("stat.leaveGame", "leave_game"),
    ("stat.playOneMinute", "play_time"),
    ("stat.timeSinceDeath", "time_since_death"),
    ("stat.walkOneCm", "walk_one_cm"),
    ("stat.crouchOneCm", "crouch_one_cm"),
    ("stat.sprintOneCm", "sprint_one_cm"),
    ("stat.swimOneCm", "swim_one_cm"),
    ("stat.fallOneCm", "fall_one_cm"),
    ("stat.climbOneCm", "climb_one_cm"),
    ("stat.flyOneCm", "fly_one_cm"),
    ("stat.diveOneCm", "walk_under_water_one_cm"),
    ("stat.minecartOneCm", "minecart_one_cm"),
    ("stat.boatOneCm", "boat_one_cm"),
    ("stat.pigOneCm", "pig_one_cm"),
    ("stat.horseOneCm", "horse_one_cm"),
    ("stat.jump", "jump"),
    ("stat.drop", "drop"),
    ("stat.damageDealt", "damage_dealt"),
    ("stat.damageTaken", "damage_taken"),
    ("stat.deaths", "deaths"),
    ("stat.mobKills", "mob_kills"),
    ("stat.animalsBred", "animals_bred"),
    ("stat.playerKills", "player_kills"),
    ("stat.fishCaught", "fish_caught"),
    ("stat.junkFished", "junk_fished"),
    ("stat.treasureFished", "treasure_fished"),
    ("stat.talkedToVillager", "talked_to_villager"),
    ("stat.tradedWithVillager", "traded_with_villager"),
    ("stat.cakeSlicesEaten", "eat_cake_slice"),
    ("stat.cauldronFilled", "fill_cauldron"),
    ("stat.cauldronUsed", "use_cauldron"),
    ("stat.armorCleaned", "clean_armor"),
    ("stat.bannerCleaned", "clean_banner"),
    ("stat.brewingstandInteraction", "interact_with_brewingstand"),
    ("stat.beaconInteraction", "interact_with_beacon"),
    ("stat.dropperInspected", "inspect_dropper"),
    ("stat.hopperInspected", "inspect_hopper"),
    ("stat.dispenserInspected", "inspect_dispenser"),
    ("stat.noteblockPlayed", "play_noteblock"),
    ("stat.noteblockTuned", "tune_noteblock"),
    ("stat.flowerPotted", "pot_flower"),
    ("stat.trappedChestTriggered", "trigger_trapped_chest"),
    ("stat.enderchestOpened", "open_enderchest"),
    ("stat.itemEnchanted", "enchant_item"),
    ("stat.recordPlayed", "play_record"),
    ("stat.furnaceInteraction", "interact_with_furnace"),
    (
        "stat.craftingTableInteraction",
        "interact_with_crafting_table",
    ),
    ("stat.chestOpened", "open_chest"),
];

/// 1.8 `EntityList` names to the current entity ids.
const MOBS_1_8_9: &[(&str, &str)] = &[
    ("Creeper", "minecraft:creeper"),
    ("Skeleton", "minecraft:skeleton"),
    ("Spider", "minecraft:spider"),
    ("Giant", "minecraft:giant"),
    ("Zombie", "minecraft:zombie"),
    ("Slime", "minecraft:slime"),
    ("Ghast", "minecraft:ghast"),
    ("PigZombie", "minecraft:zombified_piglin"),
    ("Enderman", "minecraft:enderman"),
    ("CaveSpider", "minecraft:cave_spider"),
    ("Silverfish", "minecraft:silverfish"),
    ("Blaze", "minecraft:blaze"),
    ("LavaSlime", "minecraft:magma_cube"),
    ("EnderDragon", "minecraft:ender_dragon"),
    ("WitherBoss", "minecraft:wither"),
    ("Witch", "minecraft:witch"),
    ("Guardian", "minecraft:guardian"),
    ("Endermite", "minecraft:endermite"),
    ("Rabbit", "minecraft:rabbit"),
    ("Villager", "minecraft:villager"),
    ("VillagerGolem", "minecraft:iron_golem"),
    ("SnowMan", "minecraft:snow_golem"),
    ("Bat", "minecraft:bat"),
    ("EntityHorse", "minecraft:horse"),
    ("Pig", "minecraft:pig"),
    ("Sheep", "minecraft:sheep"),
    ("Cow", "minecraft:cow"),
    ("Chicken", "minecraft:chicken"),
    ("Wolf", "minecraft:wolf"),
    ("MushroomCow", "minecraft:mooshroom"),
    ("Ozelot", "minecraft:ocelot"),
    ("Squid", "minecraft:squid"),
];

/// The 1.8 `EntityType.SPAWN_EGGS` entries, in registration order: the mobs
/// the statistics screen's Mobs tab has a row for.
pub(super) const SPAWN_EGGS_1_8: &[(&str, &str)] = &[
    ("Creeper", "minecraft:creeper"),
    ("Skeleton", "minecraft:skeleton"),
    ("Spider", "minecraft:spider"),
    ("Zombie", "minecraft:zombie"),
    ("Slime", "minecraft:slime"),
    ("Ghast", "minecraft:ghast"),
    ("PigZombie", "minecraft:zombified_piglin"),
    ("Enderman", "minecraft:enderman"),
    ("CaveSpider", "minecraft:cave_spider"),
    ("Silverfish", "minecraft:silverfish"),
    ("Blaze", "minecraft:blaze"),
    ("LavaSlime", "minecraft:magma_cube"),
    ("Bat", "minecraft:bat"),
    ("Witch", "minecraft:witch"),
    ("Endermite", "minecraft:endermite"),
    ("Guardian", "minecraft:guardian"),
    ("Pig", "minecraft:pig"),
    ("Sheep", "minecraft:sheep"),
    ("Cow", "minecraft:cow"),
    ("Chicken", "minecraft:chicken"),
    ("Squid", "minecraft:squid"),
    ("Wolf", "minecraft:wolf"),
    ("MushroomCow", "minecraft:mooshroom"),
    ("Ozelot", "minecraft:ocelot"),
    ("EntityHorse", "minecraft:horse"),
    ("Rabbit", "minecraft:rabbit"),
    ("Villager", "minecraft:villager"),
];

/// The current statistic type and key a 1.8 name counts, if this build has it.
///
/// 1.8 names statistics by legacy keys (`stat.walkOneCm`,
/// `stat.mineBlock.minecraft.stone`, `stat.killEntity.Zombie`); the current
/// model keys them by registry name.
pub(super) fn stat_1_8_9(name: &str) -> Option<(&'static str, String)> {
    if let Some(key) = name.strip_prefix("achievement.") {
        super::achievements::by_key(key)?;
        return Some(("minecraft:achievement", key.to_owned()));
    }
    if let Some((_, key)) = CUSTOM_1_8_9.iter().find(|(legacy, _)| *legacy == name) {
        return Some(("minecraft:custom", format!("minecraft:{key}")));
    }
    // Block and item counters: `stat.<action>.<namespace>.<path>`.
    const COUNTERS: &[(&str, &str)] = &[
        ("stat.mineBlock.", "minecraft:mined"),
        ("stat.craftItem.", "minecraft:crafted"),
        ("stat.useItem.", "minecraft:used"),
        ("stat.breakItem.", "minecraft:broken"),
        ("stat.pickup.", "minecraft:picked_up"),
        ("stat.drop.", "minecraft:dropped"),
    ];
    if let Some((prefix, ty)) = COUNTERS.iter().find(|(prefix, _)| name.starts_with(prefix)) {
        // `method_10801`: the identifier with its `:` turned into `.`, so
        // `stat.mineBlock.minecraft.stone` names the item `minecraft:stone`.
        let (namespace, path) = name[prefix.len()..].split_once('.')?;
        let key = format!("{namespace}:{path}");
        let stack = horizon_items::legacy_1_8_9::named_stack(&key, 1, 0, &Tag::End)?;
        return Some((*ty, counter_key(ty, &stack.item)));
    }
    // Mob counters: `stat.killEntity.<EntityList name>`.
    let (prefix, ty) = [
        ("stat.killEntity.", "minecraft:killed"),
        ("stat.entityKilledBy.", "minecraft:killed_by"),
    ]
    .into_iter()
    .find(|(prefix, _)| name.starts_with(prefix))?;
    let mob = MOBS_1_8_9
        .iter()
        .find(|(legacy, _)| *legacy == &name[prefix.len()..])?
        .1;
    Some((ty, mob.to_owned()))
}

/// The 1.8 `Stat.name` a current statistic is saved and sent under.
pub(super) fn legacy_stat_name(ty: &str, key: &str) -> Option<String> {
    let short = ty.strip_prefix("minecraft:").unwrap_or(ty);
    let counter = match short {
        "achievement" => {
            super::achievements::by_key(key)?;
            return Some(format!("achievement.{key}"));
        }
        "custom" => {
            let key = key.strip_prefix("minecraft:")?;
            // `walk_under_water_one_cm` is the datafixed name of 1.8's
            // `diveOneCm`, so a save written before that fix still writes
            // under 1.8's own name.
            let legacy = if matches!(key, "dive_one_cm" | "walk_under_water_one_cm") {
                "stat.diveOneCm"
            } else {
                CUSTOM_1_8_9.iter().find(|(_, k)| *k == key)?.0
            };
            return Some(legacy.to_owned());
        }
        "mined" => "stat.mineBlock.",
        "crafted" => "stat.craftItem.",
        "used" => "stat.useItem.",
        "broken" => "stat.breakItem.",
        "picked_up" => "stat.pickup.",
        "dropped" => "stat.drop.",
        "killed" => {
            let mob = MOBS_1_8_9.iter().find(|(_, id)| *id == key)?.0;
            return Some(format!("stat.killEntity.{mob}"));
        }
        "killed_by" => {
            let mob = MOBS_1_8_9.iter().find(|(_, id)| *id == key)?.0;
            return Some(format!("stat.entityKilledBy.{mob}"));
        }
        _ => return None,
    };
    let (id, _) = horizon_items::legacy_1_8_9::legacy_id(key)?;
    let name = horizon_items::legacy_1_8_9::legacy_name(canonical_legacy_id(id))?;
    Some(format!("{counter}{}", name.replace(':', ".")))
}
/// The stats or advancements file of a world. 1.8 has no `players/` folder
/// (its `PlayerDataStorage` only writes `playerdata/`) and its statistics live
/// in `stats/<uuid>.json`; it has no advancements, so Horizon keeps those
/// under `data/horizon/advancements` where the 1.8 game ignores them.
pub(super) fn player_json(world: &Path, dir: &str, uuid: [i32; 4]) -> PathBuf {
    let hex: String = uuid.iter().map(|v| format!("{:08x}", *v as u32)).collect();
    let text = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    if horizon_core::target::is_1_8_9() {
        return if dir == "stats" {
            world.join("stats").join(format!("{text}.json"))
        } else {
            world
                .join("data")
                .join("horizon")
                .join(dir)
                .join(format!("{text}.json"))
        };
    }
    world.join("players").join(dir).join(format!("{text}.json"))
}
impl Game<'_> {
    fn stats_path(&self) -> Option<PathBuf> {
        Some(player_json(
            &self.world_root,
            "stats",
            self.level.player_uuid?,
        ))
    }
    /// `ServerStatsCounter.parseLocal`.
    pub(super) fn load_stats(&mut self) {
        if self.stats.loaded || self.remote.is_some() || self.no_player_here() {
            return;
        }
        self.stats.loaded = true;
        if let Some(path) = self.stats_path() {
            self.stats.values = Stats::read(&path).values;
        }
    }
    /// `ServerStatsCounter.save`.
    pub(super) fn save_stats(&self) {
        if !self.stats.loaded || self.remote.is_some() || self.no_player_here() {
            return;
        }
        if let Some(path) = self.stats_path() {
            self.stats.write(&path);
        }
    }
    /// The per-tick counters (`Player.tick`/`ServerPlayer.tick` awardStat).
    pub(super) fn tick_stats(&mut self) {
        self.load_stats();
        self.count_tick_stats(self.sleeping.is_some());
        self.tick_movement_stats();
    }
    /// `Player.tick`'s counters for the player in `self.player`.
    pub(super) fn count_tick_stats(&mut self, sleeping: bool) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let alive = !p.health.dead_or_dying();
        let crouching = p.pose == horizon_client::pose::Pose::Crouching;
        let s = &mut self.stats;
        s.custom("play_time", 1);
        s.custom("total_world_time", 1);
        if alive {
            s.custom("time_since_death", 1);
            if !sleeping {
                s.custom("time_since_rest", 1);
            }
        }
        if crouching {
            s.custom("sneak_time", 1);
        }
    }
    fn tick_movement_stats(&mut self) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let d = p.position.subtract(self.previous_position);
        self.movement_stats(d);
    }
    /// `Player.checkMovementStatistics` / `checkRidingStatistics` for one move.
    pub(super) fn movement_stats(&mut self, d: Vec3) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let horizontal = ((d.x * d.x + d.z * d.z).sqrt() * 100.0).round() as i32;
        let total = ((d.x * d.x + d.y * d.y + d.z * d.z).sqrt() * 100.0).round() as i32;
        // Teleports are not travel.
        if total <= 0 || total > 1000 {
            return;
        }
        let crouching = p.pose == horizon_client::pose::Pose::Crouching;
        let feet = [
            p.position.x.floor() as i32,
            p.position.y.floor() as i32,
            p.position.z.floor() as i32,
        ];
        let eye_in_water = self.res.facts().get(self.world.block(
            feet[0],
            (p.position.y + f64::from(p.dimensions.eye_height)).floor() as i32,
            feet[2],
        ));
        let in_water = self
            .res
            .facts()
            .get(self.world.block(feet[0], feet[1], feet[2]))
            .fluid
            == horizon_client::blocks::Fluid::Water;
        let on_ground = p.ground.on_ground;
        let (sprinting, gliding, swimming) = (
            p.sprinting,
            p.fall_flying,
            p.pose == horizon_client::pose::Pose::Swimming,
        );
        let climbing = p.last_climbable.is_some() && d.y > 0.0;
        let riding = self.riding.map(|id| {
            self.mobs
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.species())
                .ok_or(())
        });
        let s = &mut self.stats;
        match riding {
            Some(Ok(sp)) => {
                use horizon_client::mobs::Species;
                // `checkRidingStatistics`: only these mounts count, the horse
                // family by the full 3D distance; any other mount nothing.
                match sp {
                    Species::Pig => s.custom("pig_one_cm", total),
                    Species::Strider => s.custom("strider_one_cm", total),
                    Species::HappyGhast => s.custom("happy_ghast_one_cm", total),
                    Species::Nautilus | Species::ZombieNautilus => {
                        s.custom("nautilus_one_cm", total)
                    }
                    sp if horizon_client::mobs::equine::is_equine(sp) => {
                        s.custom("horse_one_cm", total)
                    }
                    _ => {}
                }
            }
            Some(Err(())) => {
                if self.boats.iter().any(|b| Some(b.id) == self.riding) {
                    s.custom("boat_one_cm", total)
                } else if self.carts.iter().any(|c| Some(c.id) == self.riding) {
                    s.custom("minecart_one_cm", total)
                }
            }
            None if swimming => s.custom("swim_one_cm", total),
            None if eye_in_water.fluid == horizon_client::blocks::Fluid::Water => {
                s.custom("walk_under_water_one_cm", total)
            }
            None if in_water => s.custom("walk_on_water_one_cm", horizontal),
            None if climbing => s.custom("climb_one_cm", (d.y * 100.0).round() as i32),
            None if on_ground => {
                if sprinting {
                    s.custom("sprint_one_cm", horizontal)
                } else if crouching {
                    s.custom("crouch_one_cm", horizontal)
                } else {
                    s.custom("walk_one_cm", horizontal)
                }
            }
            None if gliding => s.custom("aviate_one_cm", total),
            // Airborne and not gliding: more than 25 cm sideways per move
            // (creative flight, or a long jump), not only flying.
            None if horizontal > 25 => s.custom("fly_one_cm", horizontal),
            None => {}
        }
    }
    /// `PhantomSpawner.tick`: tired players under an open night sky draw
    /// phantoms. A custom spawner ticks only while `spawn_mobs` is on
    /// (`ServerChunkCache.tickChunks`), and this one only spawns enemies
    /// (`ServerLevel.isSpawningMonsters`: `spawn_monsters` too). Every
    /// player of the level counts: the game's own and the hosted ones (a
    /// server thread's players are all hosted).
    pub(super) fn spawn_phantoms(&mut self) {
        let rules = &self.ui.rules;
        if !rules.bool("spawn_mobs")
            || !rules.bool("spawn_monsters")
            || !rules.bool("spawn_phantoms")
            || self.dim != Dim::Overworld
        {
            return;
        }
        self.stats.phantom_tick -= 1;
        if self.stats.phantom_tick > 0 {
            return;
        }
        let r = &mut self.world.random;
        self.stats.phantom_tick += (60 + r.next_int_bound(60).unwrap_or(0)) * 20;
        if self.sky_darken() < 5 {
            return;
        }
        let rest = |s: &Stats| s.get("custom", "minecraft:time_since_rest");
        // `level.players()`: (block position, time since rest) of each player
        // who isn't a spectator.
        let mut players = Vec::new();
        if let Some(p) = self.player.as_ref()
            && !self.spectator()
        {
            players.push((p.position, rest(&self.stats)));
        }
        if let Some(host) = self.lan.as_ref() {
            let level = self.dim.id();
            for g in host.hub.guests.iter().filter(|g| {
                g.placed
                    && g.dimension == level
                    && g.game_mode != horizon_client::lan::guest::SPECTATOR
            }) {
                let since = host.play.get(&g.conn).map_or(0, |p| rest(&p.stats));
                players.push((
                    Vec3::new(g.position[0], g.position[1], g.position[2]),
                    since,
                ));
            }
        }
        for (position, since) in players {
            self.spawn_phantoms_near(position, since);
        }
    }
    /// One player's turn of `PhantomSpawner.tick`.
    fn spawn_phantoms_near(&mut self, position: Vec3, since_rest: i32) {
        let pos = [
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        ];
        if pos[1] < 63 || !self.can_see_sky(pos) {
            return;
        }
        // `getCurrentDifficultyAt(playerPos).isHarderThan(random * 3)`.
        let difficulty = self.local_difficulty([position.x, position.y, position.z]);
        let r = &mut self.world.random;
        if difficulty.effective <= r.next_float() * 3.0 {
            return;
        }
        if r.next_int_bound(since_rest.max(1)).unwrap_or(0) < 72000 {
            return;
        }
        // `playerPos.above(20 + nextInt(15)).east(-10 + nextInt(21)).south(...)`:
        // the height is drawn first.
        let up = 20 + r.next_int_bound(15).unwrap_or(0);
        let east = -10 + r.next_int_bound(21).unwrap_or(10);
        let south = -10 + r.next_int_bound(21).unwrap_or(10);
        let at = [pos[0] + east, pos[1] + up, pos[2] + south];
        if !self
            .res
            .facts()
            .is_air(self.world.block(at[0], at[1], at[2]))
        {
            return;
        }
        let group = 1 + self.world.random.next_int_bound(3).unwrap_or(0);
        for _ in 0..group {
            let _ = self.spawn_mob(
                "minecraft:phantom",
                [
                    f64::from(at[0]) + 0.5,
                    f64::from(at[1]),
                    f64::from(at[2]) + 0.5,
                ],
                false,
            );
        }
    }
    fn can_see_sky(&self, pos: [i32; 3]) -> bool {
        (pos[1] + 1..pos[1] + 256).all(|y| {
            let f = self.res.facts().get(self.world.block(pos[0], y, pos[2]));
            !f.full_collision && f.fluid == horizon_client::blocks::Fluid::None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_eight_stat_names_round_trip() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        for name in [
            "stat.walkOneCm",
            "stat.playOneMinute",
            "stat.diveOneCm",
            "stat.mineBlock.minecraft.stone",
            "stat.mineBlock.minecraft.wool",
            "stat.craftItem.minecraft.wool",
            "stat.useItem.minecraft.dye",
            "stat.breakItem.minecraft.diamond_pickaxe",
            "stat.pickup.minecraft.apple",
            "stat.drop.minecraft.stone",
            "stat.killEntity.Zombie",
            "stat.entityKilledBy.Creeper",
            "stat.mineBlock.minecraft.pumpkin",
        ] {
            let (ty, key) = stat_1_8_9(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(
                legacy_stat_name(ty, &key).as_deref(),
                Some(name),
                "{ty} {key}"
            );
        }
        // Aliased ids share 1.8's statistic and therefore its name.
        assert_eq!(
            stat_1_8_9("stat.mineBlock.minecraft.lit_pumpkin"),
            stat_1_8_9("stat.mineBlock.minecraft.pumpkin")
        );
        assert_eq!(
            stat_1_8_9("stat.mineBlock.minecraft.grass").map(|(_, k)| k),
            stat_1_8_9("stat.mineBlock.minecraft.dirt").map(|(_, k)| k)
        );
    }
    #[test]
    fn one_eight_counters_share_one_row_per_item_id() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let mut s = Stats::default();
        s.add("mined", "minecraft:dirt", 1);
        s.add("mined", "minecraft:grass_block", 2);
        s.add("mined", "minecraft:farmland", 4);
        s.add("used", "minecraft:red_wool", 3);
        s.add("used", "minecraft:white_wool", 2);
        assert_eq!(s.get("mined", "minecraft:dirt"), 7);
        assert_eq!(s.get("used", "minecraft:white_wool"), 5);
        assert_eq!(s.get("used", "minecraft:red_wool"), 0);
    }
    #[test]
    fn one_eight_stats_file_is_the_flat_legacy_object() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "horizon-stats-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let path = dir.join("stats").join("player.json");
        let mut s = Stats::default();
        s.custom("play_time", 5);
        s.add("mined", "minecraft:stone", 7);
        s.custom("total_world_time", 9);
        s.write(&path);
        let text = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("stat.playOneMinute"), Some(&5.into()));
        assert_eq!(json.get("stat.mineBlock.minecraft.stone"), Some(&7.into()));
        // A statistic 1.8 does not have is left out, and there is no modern
        // `stats` wrapper or DataVersion.
        assert!(json.get("stat.total_world_time").is_none());
        assert!(json.get("stats").is_none());
        assert!(json.get("DataVersion").is_none());
        let read = Stats::read(&path);
        assert_eq!(read.get("custom", "minecraft:play_time"), 5);
        assert_eq!(read.get("mined", "minecraft:stone"), 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn counters_events_and_dirty_entries() {
        let mut s = Stats::default();
        s.custom("play_time", 1);
        s.custom("play_time", 1);
        s.add("mined", "minecraft:stone", 2);
        s.custom(&"x".repeat(100), 3);
        assert_eq!(s.get("custom", "minecraft:play_time"), 2);
        assert_eq!(s.get("mined", "minecraft:stone"), 2);
        assert_eq!(
            s.get("custom", &format!("minecraft:{}", "x".repeat(100))),
            3
        );
        assert_eq!(s.events.len(), 4);
        assert_eq!(
            s.events[1],
            (
                "minecraft:custom".into(),
                "minecraft:play_time".into(),
                Some(1)
            )
        );
        let long = format!("minecraft:{}", "x".repeat(100));
        let sent = |v: &[(&str, &str, i32)]| -> Vec<(String, String, i32)> {
            v.iter()
                .map(|&(t, k, n)| (t.to_owned(), k.to_owned(), n))
                .collect()
        };
        assert_eq!(
            s.take_dirty(),
            sent(&[
                ("minecraft:custom", "minecraft:play_time", 2),
                ("minecraft:custom", &long, 3),
                ("minecraft:mined", "minecraft:stone", 2),
            ])
        );
        assert!(s.take_dirty().is_empty());
        s.reset("mined", "minecraft:stone");
        assert_eq!(
            s.take_dirty(),
            sent(&[("minecraft:mined", "minecraft:stone", 0)])
        );
        s.mark_all_dirty();
        assert_eq!(s.take_dirty().len(), 3);
    }

    #[test]
    fn stats_and_advancements_stay_out_of_the_1_8_players_folder() {
        let uuid = [1, 2, 3, 4];
        let stats = player_json(Path::new("/world"), "stats", uuid);
        let advancements = player_json(Path::new("/world"), "advancements", uuid);
        if horizon_core::target::is_1_8_9() {
            assert_eq!(stats.parent(), Some(Path::new("/world/stats")));
            assert_eq!(
                advancements.parent(),
                Some(Path::new("/world/data/horizon/advancements"))
            );
        } else {
            assert_eq!(stats.parent(), Some(Path::new("/world/players/stats")));
            assert_eq!(
                advancements.parent(),
                Some(Path::new("/world/players/advancements"))
            );
        }
    }
}
