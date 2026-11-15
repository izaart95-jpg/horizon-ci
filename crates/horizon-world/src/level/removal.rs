//! What vanilla's `BlockEntity.preRemoveSideEffects` does when a block with a
//! block entity is replaced by another block (`LevelChunk.setBlockState`
//! without `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS`), and the item spill of
//! `Containers.dropItemStack` those side effects use.
use horizon_core::{position::BlockPos, random::RandomSource};

/// `preRemoveSideEffects` of the block entity of a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreRemove {
    /// No side effect (signs, banners, beacons, skulls, ...).
    Nothing,
    /// `Containers.dropContents(level, pos, container)`: every slot, empty or
    /// not, through `Containers.dropItemStack` (chests, barrels, hoppers,
    /// dispensers, droppers, brewing stands, crafters, chiseled bookshelves,
    /// shelves, decorated pots). Furnaces also award their recipes' experience.
    Contents { slots: usize, furnace: bool },
    /// `CampfireBlockEntity`: `Containers.dropContents(level, pos, items)`.
    Campfire,
    /// `ShulkerBoxBlockEntity`: keeps its contents for the dropped box.
    ShulkerBox,
    /// `JukeboxBlockEntity.popOutTheItem`.
    Jukebox,
    /// `LecternBlockEntity`: the book, when the state has one.
    Lectern,
    /// `SculkShriekerBlockEntity.tryRespond` while shrieking.
    SculkShrieker,
    /// `CreakingHeartBlockEntity.removeProtector`.
    CreakingHeart,
    /// No side effect, but `BeaconBlockEntity.setRemoved` plays the
    /// deactivation sound whenever it goes.
    Beacon,
}

/// The side effect of removing the block entity of block `name` (a
/// `minecraft:` identifier).
pub fn pre_remove(name: &str) -> PreRemove {
    let n = name.strip_prefix("minecraft:").unwrap_or(name);
    let contents = |slots| PreRemove::Contents {
        slots,
        furnace: false,
    };
    match n {
        "chest" | "trapped_chest" | "barrel" => contents(27),
        _ if n.ends_with("copper_chest") => contents(27),
        _ if n.ends_with("shulker_box") => PreRemove::ShulkerBox,
        "hopper" => contents(5),
        "dispenser" | "dropper" | "crafter" => contents(9),
        "brewing_stand" => contents(5),
        "chiseled_bookshelf" => contents(6),
        _ if n.ends_with("_shelf") => contents(3),
        "decorated_pot" => contents(1),
        "furnace" | "blast_furnace" | "smoker" => PreRemove::Contents {
            slots: 3,
            furnace: true,
        },
        "campfire" | "soul_campfire" => PreRemove::Campfire,
        "jukebox" => PreRemove::Jukebox,
        "lectern" => PreRemove::Lectern,
        "sculk_shrieker" => PreRemove::SculkShrieker,
        "creaking_heart" => PreRemove::CreakingHeart,
        "beacon" => PreRemove::Beacon,
        _ => PreRemove::Nothing,
    }
}

/// One item entity of a spill: position, stack size and motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spilled {
    pub pos: [f64; 3],
    pub count: u32,
    pub motion: [f64; 3],
}

/// `Containers.dropItemStack(level, pos.x, pos.y, pos.z, stack)` for a stack
/// of `count` items: the spot inside the block, then pieces of 10 to 30 items
/// with triangular motions, all from the level random. An empty stack still
/// draws the spot.
pub fn drop_item_stack<R: RandomSource>(random: &mut R, pos: BlockPos, count: u32) -> Vec<Spilled> {
    // EntityTypes.ITEM is 0.25 wide.
    let size = f64::from(0.25f32);
    let center_range = 1.0 - size;
    let half = size / 2.0;
    let x = f64::from(pos.x) + random.next_double() * center_range + half;
    let y = f64::from(pos.y) + random.next_double() * center_range;
    let z = f64::from(pos.z) + random.next_double() * center_range + half;
    let mut left = count;
    let mut out = Vec::new();
    while left > 0 {
        let piece = (random.next_int_bound(21).unwrap_or(0) + 10) as u32;
        let piece = piece.min(left);
        left -= piece;
        let deviation = 0.114_850_001_711_398_36;
        let motion = [
            random.triangle(0.0, deviation),
            random.triangle(0.2, deviation),
            random.triangle(0.0, deviation),
        ];
        out.push(Spilled {
            pos: [x, y, z],
            count: piece,
            motion,
        });
    }
    out
}

/// `Block.dropStack` (1.8): `ItemEntity` at three `nextFloat` offsets in
/// [0.25, 0.75], then `new ItemEntity`'s own motion.
pub fn drop_stack_position_1_8_9<R: RandomSource>(random: &mut R, pos: BlockPos) -> [f64; 3] {
    // `nextFloat() * 0.5F` is a float multiplication; `(1 - 0.5) * 0.5` is
    // the double 0.25 the bytecode adds.
    let mut offset = || 0.25 + f64::from(random.next_float() * 0.5);
    [
        f64::from(pos.x) + offset(),
        f64::from(pos.y) + offset(),
        f64::from(pos.z) + offset(),
    ]
}

/// `JukeboxBlockEntity.popOutTheItem`: `Vec3.atLowerCornerWithOffset(pos, 0.5,
/// 1.01, 0.5).offsetRandomXZ(level.random, 0.7)`.
pub fn jukebox_pop_position<R: RandomSource>(random: &mut R, pos: BlockPos) -> [f64; 3] {
    if horizon_core::target::is_1_8_9() {
        // `BlockJukebox.dropRecord`: `float f = 0.7F`; x and z add
        // `(1 - f) * 0.5`, y adds `(1 - f) * 0.2 + 0.6`. The spread comes
        // from a float product widened to double, the edge terms stay double
        // exactly as the class file evaluates them.
        const F: f32 = 0.7;
        let edge = f64::from(1.0f32 - F);
        let dx = f64::from(random.next_float() * F) + edge * 0.5;
        let dy = f64::from(random.next_float() * F) + edge * 0.2 + 0.6;
        let dz = f64::from(random.next_float() * F) + edge * 0.5;
        return [
            f64::from(pos.x) + dx,
            f64::from(pos.y) + dy,
            f64::from(pos.z) + dz,
        ];
    }
    // `JukeboxBlockEntity.popOutTheItem` (26.3):
    // `Vec3.atLowerCornerWithOffset(pos, 0.5, 1.01, 0.5).offsetRandomXZ(level.random, 0.7)`.
    let dx = f64::from((random.next_float() - 0.5) * 0.7);
    let dz = f64::from((random.next_float() - 0.5) * 0.7);
    [
        f64::from(pos.x) + 0.5 + dx,
        f64::from(pos.y) + 1.01,
        f64::from(pos.z) + 0.5 + dz,
    ]
}

/// `LecternBlockEntity.preRemoveSideEffects`: the book a quarter block
/// towards the lectern's facing, one block up (`step` is the facing's x and z).
pub fn lectern_book_position(pos: BlockPos, step: [i32; 2]) -> [f64; 3] {
    let xo = 0.25f32 * step[0] as f32;
    let zo = 0.25f32 * step[1] as f32;
    [
        f64::from(pos.x) + 0.5 + f64::from(xo),
        f64::from(pos.y + 1),
        f64::from(pos.z) + 0.5 + f64::from(zo),
    ]
}

/// `BlockBehaviour.spawnAfterBreak`'s experience for block `name` broken with
/// an unenchanted tool: `DropExperienceBlock` ranges (a `UniformInt` draws
/// `nextInt(max - min + 1) + min`, a `ConstantInt` draws nothing), redstone
/// ores, the sculk family and spawners. None when the block gives none.
pub fn break_experience<R: RandomSource>(random: &mut R, name: &str) -> Option<u32> {
    let n = name.strip_prefix("minecraft:").unwrap_or(name);
    let mut uniform = |min: i32, max: i32| random.next_int_bound(max - min + 1).unwrap_or(0) + min;
    let xp = match n {
        "coal_ore" | "deepslate_coal_ore" => uniform(0, 2),
        "nether_gold_ore" => uniform(0, 1),
        "lapis_ore" | "deepslate_lapis_ore" | "nether_quartz_ore" => uniform(2, 5),
        "diamond_ore" | "deepslate_diamond_ore" | "emerald_ore" | "deepslate_emerald_ore" => {
            uniform(3, 7)
        }
        "redstone_ore" | "deepslate_redstone_ore" => uniform(1, 5),
        "sculk" => 1,
        "sculk_catalyst" | "sculk_sensor" | "calibrated_sculk_sensor" | "sculk_shrieker" => 5,
        "spawner" => {
            let a = random.next_int_bound(15).unwrap_or(0);
            15 + a + random.next_int_bound(15).unwrap_or(0)
        }
        _ => return None,
    };
    (xp > 0).then_some(xp as u32)
}

/// `BlockOre.randomDropAsItem` and `BlockRedstoneOre.randomDropAsItem`'s
/// experience. 1.8 pays ore experience inside the drop call, after the drops
/// and their chance draws, so an explosion pays it even when every drop roll
/// fails. The bounds are `MathHelper.nextInt`'s inclusive ones; a zero draw
/// pays nothing but still consumes the draw.
pub fn explosion_experience<R: RandomSource>(random: &mut R, name: &str) -> Option<u32> {
    let n = name.strip_prefix("minecraft:").unwrap_or(name);
    let (min, max) = match n {
        "coal_ore" => (0, 2),
        "diamond_ore" | "emerald_ore" => (3, 7),
        "lapis_ore" | "nether_quartz_ore" => (2, 5),
        "redstone_ore" => (1, 5),
        _ => return None,
    };
    let xp = random.next_int_bound(max - min + 1).unwrap_or(0) + min;
    (xp > 0).then_some(xp as u32)
}

/// `ExperienceOrb.getExperienceValue`: the largest orb size not above `value`.
pub fn orb_value(value: u32) -> u32 {
    [2477, 1237, 617, 307, 149, 73, 37, 17, 7, 3, 1]
        .into_iter()
        .find(|&v| value >= v)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{explosion_experience, jukebox_pop_position};
    use horizon_core::{
        position::BlockPos,
        random::{LegacyRandom, RandomSource},
    };

    /// `java.util.Random(42)` and 1.8's `BlockJukebox.dropRecord` bytecode
    /// (`float f = 0.7F`; `x += (1 - f) * 0.5`, `y += (1 - f) * 0.2 + 0.6`).
    #[test]
    fn a_1_8_jukebox_pops_its_record_like_block_jukebox_drop_record() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let mut random = LegacyRandom::new(42);
        let at = jukebox_pop_position(&mut random, BlockPos::new(0, 0, 0));
        assert_eq!(
            at,
            [0.6592945754528046, 0.6982656478881836, 0.6282564103603363,]
        );
        // One float per coordinate, in x/y/z order: the next draw must be
        // the fourth of a fresh sequence.
        let mut expected = LegacyRandom::new(42);
        expected.next_float();
        expected.next_float();
        expected.next_float();
        assert_eq!(random.next_float(), expected.next_float());
    }

    #[test]
    fn explosion_ore_experience_is_vanillas_inclusive_range_in_one_draw() {
        for seed in 0..64 {
            for (name, min, max) in [
                ("minecraft:coal_ore", 0, 2),
                ("minecraft:diamond_ore", 3, 7),
                ("minecraft:emerald_ore", 3, 7),
                ("minecraft:lapis_ore", 2, 5),
                ("minecraft:nether_quartz_ore", 2, 5),
                ("minecraft:redstone_ore", 1, 5),
            ] {
                let mut drawn = LegacyRandom::new(seed);
                let mut expected = LegacyRandom::new(seed);
                let xp = explosion_experience(&mut drawn, name);
                let roll = expected.next_int_bound(max - min + 1).unwrap() + min;
                assert_eq!(xp, (roll > 0).then_some(roll as u32), "{name} seed {seed}");
                // A zero draw still consumes the one draw the range makes.
                assert_eq!(
                    drawn.next_int_bound(1000),
                    expected.next_int_bound(1000),
                    "{name} seed {seed} consumed the wrong number of draws"
                );
            }
        }
    }

    #[test]
    fn explosion_experience_is_ore_only() {
        for name in [
            "minecraft:iron_ore",
            "minecraft:gold_ore",
            "minecraft:spawner",
            "minecraft:sculk",
            "minecraft:stone",
        ] {
            let mut random = LegacyRandom::new(7);
            assert_eq!(explosion_experience(&mut random, name), None, "{name}");
        }
    }
}
