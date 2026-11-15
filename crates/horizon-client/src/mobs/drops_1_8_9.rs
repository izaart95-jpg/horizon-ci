//! 1.8.9's death drops, which are code rather than loot tables.
//!
//! 1.8 has no `entities/*` loot tables: every mob overrides
//! `EntityLiving.dropFewItems` (or `EntityMob.getDropItem`) and some add a
//! 2.5% `addRandomDrop`. The draws happen in a fixed order — the species'
//! items, then the equipment roll, then the rare drop — so this module keeps
//! them here instead of projecting them from 26.3 data. `MobEntity` drops
//! 0-2 of its `getDefaultDrop` by default; the overrides below are the ones
//! that differ.
//!
//! 1.8 also gates the item drops and the experience on `canDropLoot`
//! (`!isChild`) and charges looting a 1% chance per level for equipment, not
//! the modern loot predicate. A 1.8 baby therefore drops neither items nor
//! experience.
use super::drops::{Drop, clear_slot, slot_drop};
use super::{Mob, Species};
use horizon_core::random::RandomSource;
use horizon_world::loot::{LootData, LootEntity, LootStack};

/// The five 1.8 equipment slots, in `EntityLiving.dropEquipment`'s order.
const SLOTS: [&str; 5] = ["mainhand", "feet", "legs", "chest", "head"];

/// The wool item for each 1.8 dye metadata, in `EnumDyeColor` order.
const WOOL: [&str; 16] = [
    "minecraft:white_wool",
    "minecraft:orange_wool",
    "minecraft:magenta_wool",
    "minecraft:light_blue_wool",
    "minecraft:yellow_wool",
    "minecraft:lime_wool",
    "minecraft:pink_wool",
    "minecraft:gray_wool",
    "minecraft:light_gray_wool",
    "minecraft:cyan_wool",
    "minecraft:purple_wool",
    "minecraft:blue_wool",
    "minecraft:brown_wool",
    "minecraft:green_wool",
    "minecraft:red_wool",
    "minecraft:black_wool",
];

/// `EntityWitch.witchDrops`, in order; the two sticks are separate entries.
const WITCH_DROPS: [&str; 8] = [
    "minecraft:glowstone_dust",
    "minecraft:sugar",
    "minecraft:redstone",
    "minecraft:spider_eye",
    "minecraft:glass_bottle",
    "minecraft:gunpowder",
    "minecraft:stick",
    "minecraft:stick",
];

/// The twelve record items, in `ItemRecord` order (ids 2256..2267).
const RECORDS: [&str; 12] = [
    "minecraft:music_disc_13",
    "minecraft:music_disc_cat",
    "minecraft:music_disc_blocks",
    "minecraft:music_disc_chirp",
    "minecraft:music_disc_far",
    "minecraft:music_disc_mall",
    "minecraft:music_disc_mellohi",
    "minecraft:music_disc_stal",
    "minecraft:music_disc_strad",
    "minecraft:music_disc_ward",
    "minecraft:music_disc_11",
    "minecraft:music_disc_wait",
];

/// The fish entry the guardian's `addRandomDrop` picks: `getFishingLoot`'s
/// cod/salmon/clownfish/pufferfish weights (60, 25, 2, 13).
const GUARDIAN_FISH: [(&str, i32); 4] = [
    ("minecraft:cod", 60),
    ("minecraft:salmon", 25),
    ("minecraft:tropical_fish", 2),
    ("minecraft:pufferfish", 13),
];

/// `LivingEntity.onKilled`'s item part for the 1.8 target. `source` is the
/// killer (`DamageSource.getEntity`), `recently_hit` its `recentlyHit > 0`.
/// Returns the drops and whether a charged creeper left a head.
pub(super) fn items(
    m: &mut Mob,
    recently_hit: bool,
    looting: i32,
    mob_drops: bool,
    source: Option<&LootEntity>,
    loot: Option<&LootData>,
) -> (Vec<Drop>, bool) {
    let mut out = Vec::new();
    // `canDropLoot` is `!isChild` for the item part. The `onDeath` overrides
    // below still run: their own gates are `doMobLoot` through the creeper's
    // `isAIEnabled` (heads) and nothing at all (a skeleton's record).
    if !mob_drops || m.is_baby() {
        let skulls = death_extras(m, mob_drops, source, &mut out);
        // `EntityHorse.onDeath`'s inventory is outside `doMobLoot` too.
        horse_inventory(m, &mut out);
        return (out, skulls);
    }
    let burning = m.is_on_fire();
    // `dropFewItems` / `dropEquipment` / `addRandomDrop`.
    match m.species() {
        Species::Cow | Species::Mooshroom => {
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(&mut out, "minecraft:leather", n);
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + 1
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(
                &mut out,
                if burning {
                    "minecraft:cooked_beef"
                } else {
                    "minecraft:beef"
                },
                n,
            );
        }
        Species::Pig => {
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + 1
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(
                &mut out,
                if burning {
                    "minecraft:cooked_porkchop"
                } else {
                    "minecraft:porkchop"
                },
                n,
            );
            if m.extra.saddled {
                push(&mut out, "minecraft:saddle", 1);
            }
        }
        Species::Chicken => {
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(&mut out, "minecraft:feather", n);
            push(
                &mut out,
                if burning {
                    "minecraft:cooked_chicken"
                } else {
                    "minecraft:chicken"
                },
                1,
            );
        }
        Species::Sheep => {
            if !m.extra.sheared {
                let color = WOOL[m.extra.color.min(15) as usize];
                push(&mut out, color, 1);
            }
            let n = m.random.next_int_bound(2).unwrap_or(0)
                + 1
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(
                &mut out,
                if burning {
                    "minecraft:cooked_mutton"
                } else {
                    "minecraft:mutton"
                },
                n,
            );
        }
        Species::Rabbit => {
            let n = m.random.next_int_bound(2).unwrap_or(0)
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(&mut out, "minecraft:rabbit_hide", n);
            let n = m.random.next_int_bound(2).unwrap_or(0);
            push(
                &mut out,
                if burning {
                    "minecraft:cooked_rabbit"
                } else {
                    "minecraft:rabbit"
                },
                n,
            );
        }
        Species::Squid => {
            let n = m.random.next_int_bound(3 + looting).unwrap_or(0) + 1;
            push(&mut out, "minecraft:ink_sac", n);
        }
        Species::SnowGolem => {
            push(
                &mut out,
                "minecraft:snowball",
                m.random.next_int_bound(16).unwrap_or(0),
            );
        }
        Species::IronGolem => {
            push(
                &mut out,
                "minecraft:poppy",
                m.random.next_int_bound(3).unwrap_or(0),
            );
            push(
                &mut out,
                "minecraft:iron_ingot",
                3 + m.random.next_int_bound(3).unwrap_or(0),
            );
        }
        Species::Horse
        | Species::Donkey
        | Species::Mule
        | Species::SkeletonHorse
        | Species::ZombieHorse => {
            // `Horse.getDropItem` draws `rand(4)` for every horse, even when
            // the answer does not use it: a quarter of zombie horses drop no
            // flesh and the draw still happens for the living ones.
            let quarter = m.random.next_int_bound(4).unwrap_or(0) == 0;
            let item = match m.species() {
                Species::SkeletonHorse => Some("minecraft:bone"),
                Species::ZombieHorse if quarter => None,
                Species::ZombieHorse => Some("minecraft:rotten_flesh"),
                _ => Some("minecraft:leather"),
            };
            if let Some(item) = item {
                generic(&mut out, m, item, looting);
            }
        }
        // `Ocelot.dropFewItems` is empty, so a cat leaves nothing behind.
        Species::Blaze => {
            // Blaze.dropFewItems only drops rods when a player hit it.
            if recently_hit {
                push(
                    &mut out,
                    "minecraft:blaze_rod",
                    m.random.next_int_bound(2 + looting).unwrap_or(0),
                );
            }
        }
        Species::Creeper => generic(&mut out, m, "minecraft:gunpowder", looting),
        Species::Enderman => {
            push(
                &mut out,
                "minecraft:ender_pearl",
                m.random.next_int_bound(2 + looting).unwrap_or(0),
            );
        }
        Species::Ghast => {
            let n = m.random.next_int_bound(2).unwrap_or(0)
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(&mut out, "minecraft:ghast_tear", n);
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + m.random.next_int_bound(1 + looting).unwrap_or(0);
            push(&mut out, "minecraft:gunpowder", n);
        }
        Species::Guardian | Species::ElderGuardian => {
            let n = m.random.next_int_bound(3).unwrap_or(0)
                + m.random.next_int_bound(looting + 1).unwrap_or(0);
            push(&mut out, "minecraft:prismarine_shard", n);
            if m.random.next_int_bound(3 + looting).unwrap_or(0) > 1 {
                push(&mut out, "minecraft:cod", 1);
            } else if m.random.next_int_bound(3 + looting).unwrap_or(0) > 1 {
                push(&mut out, "minecraft:prismarine_crystals", 1);
            }
            if recently_hit && m.species() == Species::ElderGuardian {
                push(&mut out, "minecraft:wet_sponge", 1);
            }
        }
        Species::Skeleton => {
            let n = m.random.next_int_bound(3 + looting).unwrap_or(0);
            push(&mut out, "minecraft:arrow", n);
            let n = m.random.next_int_bound(3 + looting).unwrap_or(0);
            push(&mut out, "minecraft:bone", n);
        }
        Species::WitherSkeleton => {
            let n = m.random.next_int_bound(3 + looting).unwrap_or(0) - 1;
            push(&mut out, "minecraft:coal", n);
            let n = m.random.next_int_bound(3 + looting).unwrap_or(0);
            push(&mut out, "minecraft:bone", n);
        }
        Species::Spider | Species::CaveSpider => {
            // `Spider.getDefaultDrop` is string; the eye is the extra drop.
            generic(&mut out, m, "minecraft:string", looting);
            // `Spider.dropFewItems`: the extra eye needs a recent player hit,
            // and `rand(3) == 0` short-circuits the looting draw.
            if recently_hit
                && (m.random.next_int_bound(3).unwrap_or(0) == 0
                    || m.random.next_int_bound(1 + looting).unwrap_or(0) > 0)
            {
                push(&mut out, "minecraft:spider_eye", 1);
            }
        }
        Species::Slime => {
            if m.extra.size.max(1) == 1 {
                generic(&mut out, m, "minecraft:slime_ball", looting);
            }
        }
        Species::MagmaCube => {
            let size = m.extra.size.max(1);
            if size > 1 {
                let mut n = m.random.next_int_bound(4).unwrap_or(0) - 2;
                if looting > 0 {
                    n += m.random.next_int_bound(looting + 1).unwrap_or(0);
                }
                push(&mut out, "minecraft:magma_cream", n);
            }
        }
        Species::Witch => {
            let rounds = m.random.next_int_bound(3).unwrap_or(0) + 1;
            for _ in 0..rounds {
                let mut n = m.random.next_int_bound(3).unwrap_or(0);
                let item = WITCH_DROPS[m.random.next_int_bound(8).unwrap_or(0) as usize];
                if looting > 0 {
                    n += m.random.next_int_bound(looting + 1).unwrap_or(0);
                }
                push(&mut out, item, n);
            }
        }
        Species::Wither => push(&mut out, "minecraft:nether_star", 1),
        Species::Zombie | Species::ZombieVillager => {
            generic(&mut out, m, "minecraft:rotten_flesh", looting)
        }
        Species::ZombifiedPiglin => {
            let n = m.random.next_int_bound(2 + looting).unwrap_or(0);
            push(&mut out, "minecraft:rotten_flesh", n);
            let n = m.random.next_int_bound(2 + looting).unwrap_or(0);
            push(&mut out, "minecraft:gold_nugget", n);
        }
        // `getDropItem` is null: bat, silverfish, endermite, wolf, villager.
        Species::Bat
        | Species::Silverfish
        | Species::Endermite
        | Species::Wolf
        | Species::Villager => {}
        // 26.3-only species cannot spawn on the 1.8 target; they drop the
        // generic item where 1.8 has a counterpart and nothing otherwise.
        _ => {}
    }
    drop_equipment(m, recently_hit, looting, loot, &mut out);
    // `onKilled`: one float for the rare drop, then `addRandomDrop`.
    if recently_hit && m.random.next_float() < 0.025 + looting as f32 * 0.01 {
        rare_drop(m, &mut out);
    }
    // `Creeper.isAIEnabled` doubles as the one-head budget: the first victim
    // of a charged creeper gets its head, later ones in the same explosion do
    // not (the caller drops the killer from `charged_exploders`).
    let skulls = death_extras(m, mob_drops, source, &mut out);
    horse_inventory(m, &mut out);
    (out, skulls)
}

/// `EntityHorse.onDeath`: after `super.onDeath`, the whole horse inventory
/// (saddle, armour, chest slots), then the chest item itself. None of this
/// checks `doMobLoot` in 1.8.
fn horse_inventory(m: &mut Mob, out: &mut Vec<Drop>) {
    if !matches!(
        m.species(),
        Species::Horse
            | Species::Donkey
            | Species::Mule
            | Species::SkeletonHorse
            | Species::ZombieHorse
    ) {
        return;
    }
    if let Some(saddle) = m.saddle_item() {
        out.push(Drop::Stack(saddle));
    }
    if let Some(armor) = super::drops::slot_item(m, "body") {
        out.push(slot_drop(m, "body", armor));
    }
    for stack in m.extra.items.iter().flatten() {
        if stack.count > 0 {
            out.push(Drop::Stack(stack.clone()));
        }
    }
    if m.extra.chested {
        push(out, "minecraft:chest", 1);
    }
}

/// `MobEntity.getXpToDrop`'s equipment part: one `1 + rand(3)` for every
/// occupied slot, counted before the drops clear them (1.8 leaves dropped
/// gear in its slots until the mob is removed). Only monsters reach the
/// bonus: animals and water creatures override `getExperiencePoints`.
pub(super) fn equipment_xp_slots(m: &Mob) -> i32 {
    if !super::drops::is_monster(m.species()) {
        return 0;
    }
    SLOTS
        .iter()
        .filter(|slot| {
            super::drops::slot_item(m, slot).is_some() && super::drops::drop_chance(m, slot) <= 1.0
        })
        .count() as i32
}

/// `MobEntity.dropLoot`: `nextInt(3)`, and the looting draw only at Looting
/// I or more (1.8 guards it, so a plain kill leaves the RNG there).
fn generic(out: &mut Vec<Drop>, m: &mut Mob, item: &str, looting: i32) {
    let mut n = m.random.next_int_bound(3).unwrap_or(0);
    if looting > 0 {
        n += m.random.next_int_bound(looting + 1).unwrap_or(0);
    }
    push(out, item, n);
}

/// `EntityMob.dropEquipment`: a player hit is needed for the 8.5% chance,
/// looting adds 1% per level, and a kept item loses only 25 durability.
fn drop_equipment(
    m: &mut Mob,
    recently_hit: bool,
    looting: i32,
    loot: Option<&LootData>,
    out: &mut Vec<Drop>,
) {
    for slot in SLOTS {
        let Some(mut stack) = super::drops::slot_item(m, slot) else {
            continue;
        };
        let chance = super::drops::drop_chance(m, slot);
        // A chance above 1 is `EntityLiving.equipmentDropChances`'s "always
        // drop" marker: it needs no recent hit and takes no damage.
        let always = chance > 1.0;
        if !recently_hit && !always {
            continue;
        }
        if m.random.next_float() - looting as f32 * 0.01 >= chance {
            continue;
        }
        let max = loot.map_or(0, |loot| loot.max_damage(&stack.item));
        if !always && max > 0 {
            let k = (max - 25).max(1);
            let inner = m.random.next_int_bound(k).unwrap_or(0);
            let damage = max - m.random.next_int_bound(inner + 1).unwrap_or(0);
            stack.damage = damage.min(k).max(1);
        }
        out.push(slot_drop(m, slot, stack));
        clear_slot(m, slot);
    }
}

/// `addRandomDrop` (2.5% + 1% per looting level) for the mobs that override
/// it. Guardians roll one fish from their four-entry pool.
fn rare_drop(m: &mut Mob, out: &mut Vec<Drop>) {
    match m.species() {
        Species::Rabbit => push(out, "minecraft:rabbit_foot", 1),
        Species::WitherSkeleton => push(out, "minecraft:wither_skeleton_skull", 1),
        Species::Zombie | Species::ZombieVillager => {
            let item = match m.random.next_int_bound(3).unwrap_or(0) {
                0 => "minecraft:iron_ingot",
                1 => "minecraft:carrot",
                _ => "minecraft:potato",
            };
            push(out, item, 1);
        }
        Species::ZombifiedPiglin => push(out, "minecraft:gold_ingot", 1),
        Species::Guardian | Species::ElderGuardian => {
            let total: i32 = GUARDIAN_FISH.iter().map(|(_, weight)| weight).sum();
            let mut roll = m.random.next_int_bound(total).unwrap_or(0);
            let mut item = GUARDIAN_FISH[0].0;
            for (fish, weight) in GUARDIAN_FISH {
                roll -= weight;
                if roll < 0 {
                    item = fish;
                    break;
                }
            }
            push(out, item, 1);
        }
        _ => {}
    }
}

/// The `onDeath` overrides that run after `super.onDeath`: a charged creeper
/// takes a head, and a skeleton that kills a creeper makes it drop a record.
/// Returns whether a head dropped (`Creeper.isAIEnabled`'s one-head budget).
fn death_extras(
    m: &mut Mob,
    mob_drops: bool,
    source: Option<&LootEntity>,
    out: &mut Vec<Drop>,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    // `Creeper.isAIEnabled`: the head needs `doMobLoot`, and the caller's
    // `charged_exploders` list gives the one-per-explosion budget.
    if mob_drops && source.kind == "minecraft:creeper" && source.powered {
        let skull = match m.species() {
            Species::Creeper => Some("minecraft:creeper_head"),
            Species::Skeleton => Some("minecraft:skeleton_skull"),
            Species::WitherSkeleton => Some("minecraft:wither_skeleton_skull"),
            Species::Zombie | Species::ZombieVillager => Some("minecraft:zombie_head"),
            _ => None,
        };
        if let Some(skull) = skull {
            push(out, skull, 1);
            return true;
        }
    }
    if m.species() == Species::Creeper
        && matches!(
            source.kind.as_str(),
            "minecraft:skeleton" | "minecraft:wither_skeleton"
        )
    {
        let record = RECORDS[m.random.next_int_bound(12).unwrap_or(0) as usize];
        push(out, record, 1);
    }
    false
}

fn push(out: &mut Vec<Drop>, item: &str, count: i32) {
    if count > 0 {
        let mut stack = LootStack::new(item);
        stack.count = count;
        out.push(Drop::Item(stack));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobs::sensor_scan_tests;
    use horizon_core::geometry::Vec3;
    use horizon_core::random::LegacyRandom;
    use horizon_world::loot::PackOverlay;
    use std::path::Path;
    use std::sync::Arc;

    fn mob(species: Species, seed: i64) -> Mob {
        let mut kind = (*sensor_scan_tests::kind()).clone();
        kind.species = species;
        let mut m = Mob::new(1, Arc::new(kind), Vec3::ZERO, 0.0);
        m.random = LegacyRandom::new(seed);
        m
    }

    /// Name and count of each drop, in spawn order. Equipment keeps its
    /// rolled damage out of this view; tests that care read the stack.
    fn summary(out: Vec<Drop>) -> Vec<(String, i32)> {
        out.into_iter()
            .filter_map(|d| match d {
                Drop::Item(s) => Some((s.item, s.count)),
                Drop::Stack(s) => Some((s.item.to_string(), s.count as i32)),
                Drop::Orb(_) | Drop::CarriedBlock(_) => None,
            })
            .collect()
    }

    fn drops(m: &mut Mob, recent: bool) -> Vec<(String, i32)> {
        summary(items(m, recent, 0, true, None, None).0)
    }

    fn find(out: &[(String, i32)], item: &str) -> Option<i32> {
        out.iter()
            .find(|(name, _)| name == item)
            .map(|(_, count)| *count)
    }

    /// `CowEntity.dropLoot`: two `rand(3)` draws (plus the looting one even at
    /// 0) for leather and for beef; the real 1.8 `Random(42)` answers 2 and 1.
    #[test]
    fn a_cow_drops_two_leather_and_one_beef() {
        let mut m = mob(Species::Cow, 42);
        assert_eq!(
            drops(&mut m, false),
            [
                ("minecraft:leather".to_owned(), 2),
                ("minecraft:beef".to_owned(), 1),
            ]
        );
    }

    /// A sheared sheep keeps its wool; the mutton draw is independent.
    #[test]
    fn a_sheared_sheep_drops_only_mutton() {
        let mut m = mob(Species::Sheep, 42);
        m.extra.sheared = true;
        assert_eq!(drops(&mut m, false), [("minecraft:mutton".to_owned(), 2)]);
        let mut woolly = mob(Species::Sheep, 42);
        woolly.extra.color = 14;
        assert_eq!(
            drops(&mut woolly, false),
            [
                ("minecraft:red_wool".to_owned(), 1),
                ("minecraft:mutton".to_owned(), 2),
            ]
        );
    }

    /// `ChickenEntity.dropLoot`: a burning chicken cooks its meat.
    #[test]
    fn a_burning_chicken_drops_cooked_chicken() {
        let mut m = mob(Species::Chicken, 42);
        m.fire_ticks = 10;
        assert_eq!(
            drops(&mut m, false),
            [
                ("minecraft:feather".to_owned(), 2),
                ("minecraft:cooked_chicken".to_owned(), 1),
            ]
        );
    }

    /// `PigEntity.dropLoot`: the saddle comes after the porkchops.
    #[test]
    fn a_saddled_pig_drops_its_saddle() {
        let mut m = mob(Species::Pig, 42);
        m.extra.saddled = true;
        assert_eq!(
            drops(&mut m, false),
            [
                ("minecraft:porkchop".to_owned(), 3),
                ("minecraft:saddle".to_owned(), 1),
            ]
        );
    }

    /// `Creeper.onDeath` / `Skeleton.onDeath` / `Zombie.onDeath`: a charged
    /// creeper's victim leaves its head, and the flag spends the explosion's
    /// one-head budget; `doMobLoot` off suppresses the head.
    #[test]
    fn a_charged_creeper_gives_its_victim_a_head() {
        let source = LootEntity {
            kind: "minecraft:creeper".to_owned(),
            powered: true,
            ..LootEntity::default()
        };
        for (species, head) in [
            (Species::Creeper, "minecraft:creeper_head"),
            (Species::Skeleton, "minecraft:skeleton_skull"),
            (Species::WitherSkeleton, "minecraft:wither_skeleton_skull"),
            (Species::Zombie, "minecraft:zombie_head"),
        ] {
            let mut m = mob(species, 42);
            let (out, skulls) = items(&mut m, false, 0, true, Some(&source), None);
            assert_eq!(find(&summary(out), head), Some(1), "{species:?}");
            assert!(skulls, "{species:?}");
            let mut off = mob(species, 42);
            let (out, skulls) = items(&mut off, false, 0, false, Some(&source), None);
            assert_eq!(
                find(&summary(out), head),
                None,
                "{species:?} with doMobLoot off"
            );
            assert!(!skulls, "{species:?} with doMobLoot off");
        }
    }

    /// `Creeper.onDeath`: a skeleton's kill makes the creeper drop one of the
    /// twelve records, drawn from the entity random after the gunpowder.
    #[test]
    fn a_skeleton_killing_a_creeper_gets_a_record() {
        let source = LootEntity::new("minecraft:skeleton");
        let mut m = mob(Species::Creeper, 42);
        let out = summary(items(&mut m, true, 0, true, Some(&source), None).0);
        let records: Vec<_> = out
            .iter()
            .filter(|(item, _)| RECORDS.contains(&item.as_str()))
            .collect();
        assert_eq!(records.len(), 1, "{out:?}");
    }

    /// `Blaze.dropFewItems` only drops rods after a recent player hit.
    #[test]
    fn blaze_rods_need_a_recent_player_hit() {
        let mut untouched = mob(Species::Blaze, 42);
        assert!(drops(&mut untouched, false).is_empty());
        let mut hit = mob(Species::Blaze, 42);
        assert_eq!(find(&drops(&mut hit, true), "minecraft:blaze_rod"), Some(1));
    }

    /// `Guardian.dropFewItems`: the elder's sponge needs a recent player hit;
    /// the shard and fish/crystal draws happen either way.
    #[test]
    fn an_elder_guardian_sponge_needs_a_player_hit() {
        let mut untouched = mob(Species::ElderGuardian, 42);
        let out = drops(&mut untouched, false);
        assert_eq!(find(&out, "minecraft:prismarine_shard"), Some(2));
        assert_eq!(find(&out, "minecraft:prismarine_crystals"), Some(1));
        assert_eq!(find(&out, "minecraft:wet_sponge"), None);
        let mut hit = mob(Species::ElderGuardian, 42);
        assert_eq!(
            find(&drops(&mut hit, true), "minecraft:wet_sponge"),
            Some(1)
        );
    }

    /// `canDropLoot` / `isChild`: a baby leaves no items behind, and the
    /// caller adds no experience either.
    #[test]
    fn a_baby_drops_nothing() {
        let mut m = mob(Species::Cow, 42);
        m.extra.age = -1;
        assert!(drops(&mut m, true).is_empty());
    }

    /// `Ocelot.dropFewItems` is empty even though `getDropItem` names leather.
    #[test]
    fn an_ocelot_drops_no_leather() {
        let mut m = mob(Species::Ocelot, 42);
        assert!(drops(&mut m, true).is_empty());
    }

    /// `Horse.onDeath`: `super.onDeath` first, then saddle, armour, the chest
    /// contents and finally the chest block.
    #[test]
    fn a_horse_drops_its_whole_inventory() {
        let mut m = mob(Species::Horse, 42);
        m.extra.saddled = true;
        m.extra.chested = true;
        m.extra.head = Some(Arc::from("minecraft:iron_horse_armor"));
        m.extra.items = vec![Some(crate::items::ItemStack::new(
            "minecraft:gold_ingot",
            2,
        ))];
        let out = drops(&mut m, false);
        assert_eq!(
            &out[out.len() - 4..],
            [
                ("minecraft:saddle".to_owned(), 1),
                ("minecraft:iron_horse_armor".to_owned(), 1),
                ("minecraft:gold_ingot".to_owned(), 2),
                ("minecraft:chest".to_owned(), 1),
            ]
        );
    }

    /// `MobEntity.dropEquipment`: seed 12 puts the zombie's flesh count at 0,
    /// the 8.5% chance at 0.0598 and the iron sword's damage at 195 of 250.
    #[test]
    fn equipment_rolls_its_drop_chance_and_damage() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/horizon-client")
            .join(horizon_core::target::RESOURCES_VERSION);
        if !root.join("items.json").exists() {
            return;
        }
        let loot = LootData::load(&root, &PackOverlay::NONE).unwrap();
        let mut m = mob(Species::Zombie, 12);
        m.extra.main_hand = Some(Arc::from("minecraft:iron_sword"));
        assert_eq!(equipment_xp_slots(&m), 1);
        let (out, _) = items(&mut m, true, 0, true, None, Some(&loot));
        let sword = out.iter().find_map(|d| match d {
            Drop::Item(s) if s.item == "minecraft:iron_sword" => Some(s.damage),
            Drop::Stack(s) if &*s.item == "minecraft:iron_sword" => Some(s.damage as i32),
            _ => None,
        });
        assert_eq!(sword, Some(195));
        assert!(super::super::drops::slot_item(&m, "mainhand").is_none());
    }

    /// The equipment experience is for monsters only: a horse's saddle and
    /// armour are part of its `EntityAnimal` reward, not extra orbs.
    #[test]
    fn equipment_experience_counts_monsters_only() {
        let mut cow = mob(Species::Cow, 1);
        cow.extra.main_hand = Some(Arc::from("minecraft:iron_sword"));
        assert_eq!(equipment_xp_slots(&cow), 0);
        let mut zombie = mob(Species::Zombie, 1);
        zombie.extra.main_hand = Some(Arc::from("minecraft:iron_sword"));
        zombie.extra.feet = Some(Arc::from("minecraft:leather_boots"));
        assert_eq!(equipment_xp_slots(&zombie), 2);
    }

    /// 1.8's `EntityAnimal.getExperiencePoints` draws `1 + world.rand(3)`,
    /// while the entity random only feeds the species' item drops.
    #[test]
    fn animal_experience_draws_from_the_world_random() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/horizon-client")
            .join(horizon_core::target::RESOURCES_VERSION);
        if !root.join("items.json").exists() {
            return;
        }
        let loot = LootData::load(&root, &PackOverlay::NONE).unwrap();
        for seed in [0, 1, 42, 99] {
            let expected = 1 + LegacyRandom::new(seed).next_int_bound(3).unwrap();
            let mut world = LegacyRandom::new(seed);
            let mut cow = mob(Species::Cow, 7);
            let mut death = super::super::drops::Death {
                loot: &loot,
                damage: None,
                player: Some(LootEntity::default()),
                luck: 0.0,
                vehicle: None,
                mob_drops: true,
                world_seed: 0,
                world_random: Some(&mut world),
            };
            let (out, _) = super::super::drops::die(
                &mut cow,
                &mut death,
                &mut crate::random_sequences::RandomSequences::default(),
            );
            let xp: i32 = out
                .iter()
                .map(|d| match d {
                    Drop::Orb(v) => *v,
                    _ => 0,
                })
                .sum();
            assert_eq!(xp, expected, "seed {seed}");
        }
    }
}
