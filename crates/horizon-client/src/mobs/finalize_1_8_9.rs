//! The 1.8.9 `EntityLiving.onInitialSpawn` and its overrides.
//!
//! 1.8 finalizes a mob through `EntityLiving.onInitialSpawn`, which every
//! species but the cave spider calls: the superclass adds the "Random spawn
//! bonus" to its follow range, then the species picks its variant, colour,
//! size, age and gear. Vanilla gives each entity its own `Random`, so only
//! the draws a species takes from the level's random join the level's
//! sequence: a sheep's fleece colour, a villager's profession and a spider's
//! jockey (the jockey is spawned with the mob, in `Game::spawn_jockeys`).
//!
//! Not ported yet: the 26.3-only conversion paths (a frog grown from a
//! tadpole) still run the 26.3 finalize. Every 1.8 spawn path uses this one:
//! natural and generation spawns, spawn eggs, `/summon` without NBT and
//! spawner blocks; a mob loaded from NBT keeps its saved state, as in 1.8.
use super::{
    Mob, Species, cube, doors, equine,
    finalize::{self, Attr, Op},
    gear,
    spawning::LocalDifficulty,
};
use horizon_core::random::RandomSource;

/// `EntityVillager`'s five 1.8 professions as modern profession ids: farmer,
/// librarian, priest, blacksmith and butcher.
const PROFESSIONS_1_8_9: [u8; 5] = [5, 9, 4, 14, 2];

/// What finalizing a 1.8 mob produced besides the mob itself.
#[derive(Default)]
pub struct Finalized {
    /// The entity id the mob becomes (a horse turns into a donkey, a zombie
    /// into a villager zombie and a nether skeleton into a wither skeleton),
    /// or `None` when it stays what it was.
    pub convert: Option<&'static str>,
    /// The entities `onInitialSpawn` spawned with it: the ocelot's two
    /// kittens.
    pub extra: Vec<Mob>,
}

/// `EntityLiving.onInitialSpawn` and each species' override. Returns the
/// entity id the mob becomes and the entities the spawn comes with.
pub fn finalize(
    m: &mut Mob,
    world_random: &mut impl RandomSource,
    d: &LocalDifficulty,
    nether: bool,
) -> Finalized {
    use Species as S;
    let mut extra = Vec::new();
    // `CaveSpider.onInitialSpawn` returns without calling its superclass, so
    // it alone misses the follow-range bonus.
    if m.species() != S::CaveSpider {
        let bonus = m.random.next_gaussian() * 0.05;
        finalize::add_modifier(
            m,
            Attr::FollowRange,
            "minecraft:random_spawn_bonus",
            bonus,
            Op::AddMultipliedBase,
        );
    }
    match m.species() {
        // `EntityHorse.onInitialSpawn`: one spawn in ten is a donkey, one in
        // five a foal. A donkey is a species of its own here.
        S::Horse => {
            let donkey = m.random.next_int_bound(10).unwrap_or(1) == 0;
            let foal = m.random.next_int_bound(5).unwrap_or(1) == 0;
            if foal {
                finalize::set_age(m, -24000);
            }
            if donkey {
                return Finalized {
                    convert: Some("minecraft:donkey"),
                    extra,
                };
            }
            equine::finalize(m);
        }
        S::Donkey | S::Mule | S::SkeletonHorse | S::ZombieHorse => equine::finalize(m),
        // `EntitySheep.onInitialSpawn` rolls on the world's random: 18 in 100
        // pick a colour outright, the rest draw the once-in-500 pink.
        S::Sheep => {
            let roll = world_random.next_int_bound(100).unwrap_or(0);
            let rare = if roll < 18 {
                1
            } else {
                world_random.next_int_bound(500).unwrap_or(1)
            };
            m.extra.color = sheep_color(roll, rare);
        }
        // 1.8 has no sound variants and no climate variants: a pig is a pig.
        S::Pig | S::Cow | S::Chicken => m.extra.variant = 0,
        S::Mooshroom => m.extra.variant = 0,
        S::Wolf => m.extra.variant = 0,
        // `EntityVillager.onInitialSpawn` rolls the profession on the world's
        // random; 1.8 has no biome type.
        S::Villager => {
            let profession = world_random.next_int_bound(5).unwrap_or(0) as usize;
            m.extra.variant = 0;
            m.extra.variant2 = PROFESSIONS_1_8_9[profession];
        }
        // `EntityRabbit.onInitialSpawn`: one of the six 1.8 coats.
        S::Rabbit => m.extra.variant = m.random.next_int_bound(6).unwrap_or(0) as u8,
        // `EntitySlime.onInitialSpawn`: a size of 1, 2 or 4, one step larger
        // half the time on harder difficulties.
        S::Slime | S::MagmaCube => {
            let mut scale = m.random.next_int_bound(3).unwrap_or(0);
            if scale < 2 && m.random.next_float() < 0.5 * d.special() {
                scale += 1;
            }
            cube::set_size(m, 1 << scale);
        }
        S::Zombie => {
            if zombie(m, d, world_random, false) {
                return Finalized {
                    convert: Some("minecraft:zombie_villager"),
                    extra,
                };
            }
        }
        // A zombie villager the path above made keeps the zombie's rolls.
        S::ZombieVillager => {
            zombie(m, d, world_random, false);
        }
        S::Skeleton => {
            // `EntitySkeleton.onInitialSpawn`: four nether skeletons in five
            // are wither skeletons, with a stone sword and four attack damage.
            if nether && m.random.next_int_bound(5).unwrap_or(1) > 0 {
                m.extra.set_main_hand(Some("minecraft:stone_sword".into()));
                finalize::set_base(m, Attr::AttackDamage, 4.0);
                let pick_up = m.random.next_float() < 0.55 * d.special();
                m.extra.pick_up_loot = pick_up;
                halloween(m);
                return Finalized {
                    convert: Some("minecraft:wither_skeleton"),
                    extra,
                };
            }
            skeleton(m, d);
        }
        S::WitherSkeleton => {
            m.extra.set_main_hand(Some("minecraft:stone_sword".into()));
            finalize::set_base(m, Attr::AttackDamage, 4.0);
            let pick_up = m.random.next_float() < 0.55 * d.special();
            m.extra.pick_up_loot = pick_up;
            halloween(m);
        }
        // `EntityPigZombie.onInitialSpawn` runs `EntityZombie`'s whole path
        // and then clears the child flag; its own equipment override gives it
        // a golden sword (1.8 has no golden spear) and no armour.
        S::ZombifiedPiglin => {
            zombie(m, d, world_random, true);
            finalize::set_age(m, 0);
        }
        // `EntityOcelot.onInitialSpawn`: one spawn in seven brings two
        // kittens, babies at the parent's position and facing. They are born
        // with the spawn, not finalized themselves.
        S::Ocelot => {
            if world_random.next_int_bound(7).unwrap_or(1) == 0 {
                for _ in 0..2 {
                    let mut kitten = Mob::new(0, m.kind.clone(), m.position, m.yaw);
                    finalize::set_age(&mut kitten, -24000);
                    extra.push(kitten);
                }
            }
        }
        _ => {}
    }
    Finalized {
        convert: None,
        extra,
    }
}

/// `EntitySheep.getRandomFleeceColor`: black, grey, light grey and brown at
/// 5, 5, 5 and 3 in 100, pink once in 500 when neither, else white.
fn sheep_color(roll: i32, rare: i32) -> u8 {
    match roll {
        0..=4 => 15,
        5..=9 => 7,
        10..=14 => 8,
        15..=17 => 12,
        _ if rare == 0 => 6,
        _ => 0,
    }
}

/// `EntityZombie.onInitialSpawn`: loot picking, a baby or a villager one
/// time in twenty each, doors, armour, enchantments, the halloween pumpkin
/// and `handleAttributes`' knockback, follow range and leader bonuses. True
/// when the zombie is a villager zombie. A pigman runs the same path with
/// its own equipment override, which is the golden sword alone.
fn zombie(
    m: &mut Mob,
    d: &LocalDifficulty,
    world_random: &mut impl RandomSource,
    pigman: bool,
) -> bool {
    m.extra.pick_up_loot = m.random.next_float() < 0.55 * d.special();
    // The living data's two rolls are the level's, not the mob's.
    let baby = world_random.next_float() < 0.05;
    let villager = world_random.next_float() < 0.05;
    if baby {
        finalize::set_age(m, -24000);
    }
    let can_break_doors = m.random.next_float() < d.special() * 0.1;
    doors::set_can_break_doors(m, can_break_doors);
    if pigman {
        m.extra.set_main_hand(Some("minecraft:golden_sword".into()));
    } else {
        populate_armor(m, d);
    }
    enchant_equipment(m, d);
    halloween(m);
    // `EntityZombie.handleAttributes`: the knockback resistance, the follow
    // range and the leader's reinforcements and health.
    let knockback = m.random.next_double() * f64::from(0.05f32);
    finalize::add_modifier(
        m,
        Attr::KnockbackResistance,
        "minecraft:random_spawn_bonus",
        knockback,
        Op::AddValue,
    );
    let follow = m.random.next_double() * 1.5 * f64::from(d.special());
    if follow > 1.0 {
        finalize::add_modifier(
            m,
            Attr::FollowRange,
            "minecraft:zombie_random_spawn_bonus",
            follow,
            Op::AddMultipliedTotal,
        );
    }
    if m.random.next_float() < d.special() * 0.05 {
        let reinforcements = m.random.next_double() * 0.25 + 0.5;
        let health = m.random.next_double() * 3.0 + 1.0;
        finalize::add_modifier(
            m,
            Attr::SpawnReinforcements,
            "minecraft:leader_zombie_bonus",
            reinforcements,
            Op::AddValue,
        );
        finalize::add_modifier(
            m,
            Attr::MaxHealth,
            "minecraft:leader_zombie_bonus",
            health,
            Op::AddMultipliedTotal,
        );
        m.health = finalize::max_health(m);
        doors::set_can_break_doors(m, true);
    }
    villager
}

/// The non-wither `EntitySkeleton.onInitialSpawn`: armour, a bow,
/// enchantments, loot picking and the halloween pumpkin.
fn skeleton(m: &mut Mob, d: &LocalDifficulty) {
    populate_armor(m, d);
    m.extra.set_main_hand(Some("minecraft:bow".into()));
    enchant_equipment(m, d);
    let pick_up = m.random.next_float() < 0.55 * d.special();
    m.extra.pick_up_loot = pick_up;
    halloween(m);
}

/// `EntityLiving.setEquipmentBasedOnDifficulty`: sometimes a set of armour,
/// from the head down, of one random 1.8 material.
fn populate_armor(m: &mut Mob, d: &LocalDifficulty) {
    if m.random.next_float() >= 0.15 * d.special() {
        return;
    }
    let mut tier = m.random.next_int_bound(2).unwrap_or(0);
    for _ in 0..3 {
        if m.random.next_float() < 0.095 {
            tier += 1;
        }
    }
    let partial = if d.hard { 0.1 } else { 0.25 };
    let material = ["leather", "golden", "chainmail", "iron", "diamond"][tier as usize];
    for (i, piece) in ["helmet", "chestplate", "leggings", "boots"]
        .into_iter()
        .enumerate()
    {
        if i > 0 && m.random.next_float() < partial {
            break;
        }
        let slot = match i {
            0 => &mut m.extra.head,
            1 => &mut m.extra.chest,
            2 => &mut m.extra.legs,
            _ => &mut m.extra.feet,
        };
        if slot.is_none() {
            *slot = Some(format!("minecraft:{material}_{piece}").into());
        }
    }
}

/// `EntityLiving.setEnchantmentBasedOnDifficulty`: the main hand at
/// `0.25 * special`, then each armour piece from the feet up at
/// `0.5 * special`, each at a level of `5 + special * nextInt(18)` from the
/// mob's own random. The tables and the selection are
/// [`horizon_items::enchant_1_8_9`], not the 26.3 spawn equipment.
fn enchant_equipment(m: &mut Mob, d: &LocalDifficulty) {
    let special = d.special();
    let mut random = m.random.clone();
    if m.extra.main_hand.is_some() && random.next_float() < 0.25 * special {
        let level = (5.0 + special * random.next_int_bound(18).unwrap_or(0) as f32) as i32;
        let item = m.extra.main_hand.clone().expect("checked above");
        for (enchantment, level) in
            horizon_items::enchant_1_8_9::add_random(&mut random, &item, level)
        {
            gear::upgrade(m, gear::MAIN, &enchantment, level);
        }
    }
    // `getCurrentArmor(0..3)` is the feet, legs, chest and head.
    let armor = [
        (gear::FEET, m.extra.feet.clone()),
        (gear::LEGS, m.extra.legs.clone()),
        (gear::CHEST, m.extra.chest.clone()),
        (gear::HEAD, m.extra.head.clone()),
    ];
    for (slot, item) in armor {
        let Some(item) = item else { continue };
        if random.next_float() >= 0.5 * special {
            continue;
        }
        let level = (5.0 + special * random.next_int_bound(18).unwrap_or(0) as f32) as i32;
        for (enchantment, level) in
            horizon_items::enchant_1_8_9::add_random(&mut random, &item, level)
        {
            gear::upgrade(m, slot, &enchantment, level);
        }
    }
    m.random = random;
}

/// `Zombie`/`AbstractSkeleton`'s halloween pumpkin, drawn at the end of
/// their own `onInitialSpawn` in 1.8.
fn halloween(m: &mut Mob) {
    let mut random = m.random.clone();
    finalize::halloween_head(m, &mut random, finalize::is_halloween());
    m.random = random;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `EntitySheep.getRandomFleeceColor`'s thresholds, and the once-in-500
    /// pink only when no other colour matched.
    #[test]
    fn one_eight_sheep_colours_follow_their_thresholds() {
        assert_eq!(sheep_color(0, 1), 15);
        assert_eq!(sheep_color(4, 1), 15);
        assert_eq!(sheep_color(5, 1), 7);
        assert_eq!(sheep_color(9, 1), 7);
        assert_eq!(sheep_color(10, 1), 8);
        assert_eq!(sheep_color(14, 1), 8);
        assert_eq!(sheep_color(15, 1), 12);
        assert_eq!(sheep_color(17, 1), 12);
        assert_eq!(sheep_color(18, 1), 0);
        assert_eq!(sheep_color(99, 1), 0);
        assert_eq!(sheep_color(18, 0), 6);
        assert_eq!(sheep_color(99, 0), 6);
        // The rare roll never sees a colour the first roll settled.
        assert_eq!(sheep_color(0, 0), 15);
        assert_eq!(sheep_color(17, 0), 12);
    }

    /// The five 1.8 professions as modern ids: farmer, librarian, priest,
    /// blacksmith, butcher.
    #[test]
    fn one_eight_professions_map_to_their_modern_ids() {
        assert_eq!(PROFESSIONS_1_8_9, [5, 9, 4, 14, 2]);
        assert_eq!(PROFESSIONS_1_8_9.len(), 5);
    }

    fn ocelot() -> Mob {
        let mut kind = (*crate::mobs::sensor_scan_tests::kind()).clone();
        kind.species = Species::Ocelot;
        Mob::new(
            0,
            std::sync::Arc::new(kind),
            horizon_core::geometry::Vec3::new(4.5, 64.0, -2.5),
            37.5,
        )
    }

    /// `EntityOcelot.onInitialSpawn`: one spawn in seven comes with two
    /// kittens, babies at the parent's position and facing.
    #[test]
    fn one_ocelot_in_seven_brings_two_kittens() {
        use horizon_core::random::LegacyRandom;
        let d = LocalDifficulty::new(2, 0, 0, 1.0);
        let (mut kittens, mut plain) = (0, 0);
        for seed in 0..64 {
            let mut parent = ocelot();
            let out = finalize(&mut parent, &mut LegacyRandom::new(seed), &d, false);
            assert_eq!(out.convert, None);
            match out.extra.len() {
                0 => plain += 1,
                2 => {
                    for kitten in &out.extra {
                        assert_eq!(kitten.species(), Species::Ocelot);
                        assert_eq!(kitten.extra.age, -24000);
                        assert_eq!(kitten.position, parent.position);
                        assert_eq!(kitten.yaw, parent.yaw);
                    }
                    kittens += 1;
                }
                n => panic!("seed {seed} spawned {n} kittens"),
            }
        }
        // The gate is one in seven, so both outcomes appear in 64 seeds.
        assert!(kittens > 0, "no seed in 64 rolled the kittens");
        assert!(plain > 0, "every seed rolled the kittens");
    }

    fn zombie(at: (f64, f64)) -> Mob {
        let mut kind = (*crate::mobs::sensor_scan_tests::kind()).clone();
        kind.species = Species::Zombie;
        Mob::new(
            0,
            std::sync::Arc::new(kind),
            horizon_core::geometry::Vec3::new(at.0, 64.0, at.1),
            0.0,
        )
    }

    /// `EntityZombie.onInitialSpawn` takes its two living-data rolls from the
    /// level's random: the first 5% is the baby, the second the villager
    /// zombie. A world seed decides both whatever the mob's own random is,
    /// which the position varies.
    #[test]
    fn one_eight_zombie_baby_comes_from_the_worlds_random() {
        use horizon_core::random::LegacyRandom;
        let d = LocalDifficulty::new(2, 0, 0, 1.0);
        // 61000183 rolls 0.0261 then 0.6459: a baby, not a villager.
        for at in [(0.5, 0.5), (40.5, -16.5)] {
            let mut m = zombie(at);
            let out = finalize(&mut m, &mut LegacyRandom::new(61_000_183), &d, false);
            assert_eq!(m.extra.age, -24000, "{at:?}");
            assert_eq!(out.convert, None, "{at:?}");
            // Seed 0 rolls 0.7310: an adult, not a villager.
            let mut m = zombie(at);
            finalize(&mut m, &mut LegacyRandom::new(0), &d, false);
            assert_eq!(m.extra.age, 0, "{at:?}");
        }
    }
}
