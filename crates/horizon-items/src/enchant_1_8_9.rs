//! 1.8.9's own enchantment table selection: `EnchantmentHelper.buildEnchantmentList`,
//! which gives a monster the enchantments on its spawn gear
//! (`EntityLiving.setEnchantmentBasedOnDifficulty`) and, with the 1.8 costs,
//! what the enchanting table puts on an item.
//!
//! 26.3 picks from data-driven `min_cost`/`max_cost` windows and component
//! tags; 1.8 has an `int` id per enchantment, a weight, a cost window that is
//! a pair of linear functions of the level, and `canApplyTogether` between the
//! entries. `ENCHANTMENTS` and `ITEMS` in the tables file included below are
//! those tables, dumped from the real jar by `tools/EnchantProbe18.java`.
use horizon_core::random::RandomSource;
use std::collections::BTreeMap;
use std::sync::Arc;

include!("enchant_1_8_9_tables.rs");

fn next_int<R: RandomSource>(random: &mut R, bound: i32) -> i32 {
    random.next_int_bound(bound.max(1)).unwrap_or(0)
}

/// `Item.b()`: the enchantability `buildEnchantmentList` starts from, or 0
/// when 1.8 has no such item.
pub fn enchantability(item: &str) -> u32 {
    ITEMS
        .iter()
        .find(|(name, _, _)| *name == item)
        .map_or(0, |(_, enchantability, _)| *enchantability)
}

/// `buildEnchantmentList`'s item gate: the enchantment's
/// `EnumEnchantmentType` must accept the item. A book takes every entry, as
/// the jar's own book check does. (Note this is *not* `Enchantment.canApply`:
/// that one also lets sharpness onto an axe, which the type gate refuses.)
pub fn can_apply(item: &str, index: usize) -> bool {
    item == "minecraft:book"
        || ITEMS
            .iter()
            .find(|(name, _, _)| *name == item)
            .is_some_and(|(_, _, types)| types & (1 << ENCHANTMENTS[index].kind) != 0)
}

/// `Enchantment.canApplyTogether` in the direction `buildEnchantmentList`
/// asks it: `picked` is already on the item, `candidate` the next one.
pub fn compatible(picked: usize, candidate: usize) -> bool {
    ENCHANTMENTS[picked].compatible & (1 << candidate) != 0
}

/// The order Java's `HashMap` iterates the candidate `Integer` keys in: the
/// table starts at 16 buckets and doubles at 12 and 24 entries, iteration
/// walks the buckets and each bucket keeps its insertion order, so ids that
/// share a bucket (`id & (capacity - 1)`) keep ascending id order. `capacity`
/// is the map's at insertion; `HashMap` never shrinks it, so the pruning in
/// `buildEnchantmentList` keeps the order it started with.
fn map_order(candidates: &BTreeMap<usize, u32>, capacity: usize) -> Vec<usize> {
    let mask = capacity - 1;
    let mut order: Vec<usize> = candidates.keys().copied().collect();
    order.sort_by_key(|&i| ENCHANTMENTS[i].id as usize & mask);
    order
}

/// The capacity `HashMap` reaches after inserting `entries`: 16 to start,
/// doubling past three quarters full.
fn map_capacity(entries: usize) -> usize {
    match entries {
        0..=12 => 16,
        13..=24 => 32,
        _ => 64,
    }
}

/// `WeightedRandom.getRandomItem`: the weights of the collection in the
/// iteration order above, then a subtraction walk over it.
fn weighted<R: RandomSource>(
    random: &mut R,
    candidates: &BTreeMap<usize, u32>,
    order: &[usize],
) -> Option<usize> {
    let total: i32 = order.iter().map(|&i| ENCHANTMENTS[i].weight as i32).sum();
    if total <= 0 {
        return None;
    }
    let mut pick = next_int(random, total);
    for &i in order {
        pick -= ENCHANTMENTS[i].weight as i32;
        if pick < 0 {
            return Some(i);
        }
    }
    None
}

/// `EnchantmentHelper.buildEnchantmentList`: the `(index, level)` pairs a
/// roll of `level` gives `item`, in the order 1.8 appends them.
pub fn build_list<R: RandomSource>(random: &mut R, item: &str, level: i32) -> Vec<(usize, u32)> {
    let enchantability = enchantability(item);
    if enchantability == 0 {
        return Vec::new();
    }
    let half = (enchantability / 2) as i32;
    let base = 1 + next_int(random, (half >> 1) + 1) + next_int(random, (half >> 1) + 1);
    let span = (random.next_float() + random.next_float() - 1.0) * 0.15;
    let mut cost = ((base + level) as f32 * (1.0 + span) + 0.5) as i32;
    if cost < 1 {
        cost = 1;
    }
    let mut candidates: BTreeMap<usize, u32> = BTreeMap::new();
    for (index, e) in ENCHANTMENTS.iter().enumerate() {
        if !can_apply(item, index) {
            continue;
        }
        for level in 1..=e.max_level {
            let slot = level as usize - 1;
            if cost < e.first[slot] || cost > e.last[slot] {
                continue;
            }
            candidates.insert(index, level);
        }
    }
    let capacity = map_capacity(candidates.len());
    let mut picked = Vec::new();
    if let Some(first) = weighted(random, &candidates, &map_order(&candidates, capacity)) {
        picked.push((first, candidates[&first]));
    }
    if picked.is_empty() {
        return picked;
    }
    while next_int(random, 50) <= cost {
        candidates.retain(|&index, _| {
            picked
                .iter()
                .all(|&(on_item, _)| compatible(on_item, index))
        });
        if !candidates.is_empty() {
            let order = map_order(&candidates, capacity);
            let pick = weighted(random, &candidates, &order).expect("the candidates have weight");
            picked.push((pick, candidates[&pick]));
        }
        cost >>= 1;
    }
    picked
}

/// `EnchantmentHelper.addRandomEnchantment` on a piece of spawn gear: the
/// enchantment ids and levels a `level` roll gives `item`.
pub fn add_random<R: RandomSource>(random: &mut R, item: &str, level: i32) -> Vec<(Arc<str>, u32)> {
    build_list(random, item, level)
        .into_iter()
        .map(|(index, level)| (Arc::from(ENCHANTMENTS[index].name), level))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::random::LegacyRandom;

    /// The 1.8 jar's own `buildEnchantmentList` output, item by item, over
    /// nine levels and two seeds: the picked levels in order and the draws
    /// that follow, which count the ones the build made.
    #[test]
    fn the_jar_builds_the_same_lists() {
        for &(item, level, seed, expected, after) in VECTORS {
            let mut random = LegacyRandom::new(seed);
            let picked = build_list(&mut random, item, level);
            let ids: Vec<(u32, u32)> = picked
                .iter()
                .map(|&(index, level)| (ENCHANTMENTS[index].id, level))
                .collect();
            assert_eq!(ids, expected, "{item} at level {level} from seed {seed}");
            assert_eq!(
                random.next_int_bound(1000).unwrap_or(0),
                after,
                "{item} at level {level} from seed {seed}"
            );
        }
    }

    /// The tables are 1.8's: 25 entries in ascending id, the enchantability
    /// an item had before 1.9 flattened it, and the weapon/armour/job split
    /// `EnumEnchantmentType` made.
    #[test]
    fn the_tables_are_the_jar_tables() {
        assert_eq!(ENCHANTMENTS.len(), 25);
        assert!(ENCHANTMENTS.windows(2).all(|w| w[0].id < w[1].id));
        assert_eq!(enchantability("minecraft:golden_sword"), 22);
        assert_eq!(enchantability("minecraft:diamond_sword"), 10);
        assert_eq!(enchantability("minecraft:leather_helmet"), 15);
        assert_eq!(enchantability("minecraft:iron_helmet"), 9);
        assert_eq!(enchantability("minecraft:bow"), 1);
        // Not a 1.8 item name: 26.3's netherite sword has no enchantability.
        assert_eq!(enchantability("minecraft:netherite_sword"), 0);

        let sharpness = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:sharpness")
            .unwrap();
        let protection = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:protection")
            .unwrap();
        let respiration = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:respiration")
            .unwrap();
        let efficiency = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:efficiency")
            .unwrap();
        let power = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:power")
            .unwrap();
        assert!(can_apply("minecraft:diamond_sword", sharpness));
        assert!(!can_apply("minecraft:iron_helmet", sharpness));
        assert!(can_apply("minecraft:iron_helmet", protection));
        assert!(can_apply("minecraft:iron_helmet", respiration));
        assert!(can_apply("minecraft:book", sharpness));
        // 1.8 axes are `ItemTool`: the type gate gives them the digging
        // enchantments only, although `Enchantment.canApply` lets sharpness
        // through. Swords, bows and armour split the same way.
        assert!(!can_apply("minecraft:iron_axe", sharpness));
        assert!(!can_apply("minecraft:iron_axe", protection));
        assert!(can_apply("minecraft:iron_axe", efficiency));
        assert!(can_apply("minecraft:diamond_sword", sharpness));
        assert!(!can_apply("minecraft:diamond_sword", efficiency));
        assert!(can_apply("minecraft:bow", power));
        assert!(!can_apply("minecraft:bow", sharpness));
        assert!(!can_apply("minecraft:iron_helmet", efficiency));
        // Protection and fire protection never share an item; feather
        // falling combines with both.
        let fire = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:fire_protection")
            .unwrap();
        let feather = ENCHANTMENTS
            .iter()
            .position(|e| e.name == "minecraft:feather_falling")
            .unwrap();
        assert!(!compatible(protection, fire));
        assert!(compatible(protection, feather));
        assert!(compatible(fire, feather));
        // Weight and cost windows: protection is the common, cheap one.
        assert_eq!(ENCHANTMENTS[protection].weight, 10);
        assert_eq!(ENCHANTMENTS[protection].first, [1, 12, 23, 34, 45]);
        assert_eq!(ENCHANTMENTS[protection].last, [21, 32, 43, 54, 65]);
        assert_eq!(ENCHANTMENTS[protection].max_level, 4);
    }

    /// A level the item cannot take draws nothing at all: 1.8 reads the
    /// item's enchantability before any random call.
    #[test]
    fn a_non_enchantable_item_draws_nothing() {
        let mut random = LegacyRandom::new(7);
        let before = random.clone();
        assert!(build_list(&mut random, "minecraft:stone", 30).is_empty());
        assert_eq!(random.next_int(), before.clone().next_int());
    }

    /// The pick order is Java's `HashMap` iteration, not ascending id: the
    /// ids that share a bucket come in insertion order, and the table's
    /// capacity grows at 13 and at 25 entries.
    #[test]
    fn the_candidate_order_is_the_java_map_order() {
        let mut candidates = BTreeMap::new();
        // Ids 0 and 32 share bucket 0, so the ascending-id order and the map
        // order disagree.
        for index in [0, 1, 2, 3, 4, 32, 33, 34, 35] {
            let position = ENCHANTMENTS.iter().position(|e| e.id == index).unwrap();
            candidates.insert(position, 1);
        }
        let order = map_order(&candidates, map_capacity(candidates.len()));
        let ids: Vec<u32> = order.iter().map(|&i| ENCHANTMENTS[i].id).collect();
        assert_eq!(ids, [0, 32, 1, 33, 2, 34, 3, 35, 4]);
    }
}
