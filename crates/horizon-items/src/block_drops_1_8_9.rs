//! 1.8.9 block drops: `Block.harvest` / `Block.randomDropAsItem` and the
//! overrides of the blocks whose drop is not "one of this block's item".
//!
//! The per-state table (`block-drops.json`) is generated from a dump of the
//! real 1.8.9 jar by `tools/BlockDropProbe18.java` and
//! `examples/prepare_1_8_9_block_drops.rs`. It stores what the probe can call
//! directly: the item `getDropItem` returns, the metadata `getMeta` writes,
//! the count (`getBonusDrops`/`getDropCount` with a seeded random) and the
//! silk-touch stack `createStackFromBlock` makes.
//!
//! The code stays here when the answer depends on a random draw: 1.8's
//! fortune rules draw in a different order than 26.3's loot tables, and
//! several blocks (`LeavesBlock`, `CropBlock`, `StemBlock`, ...) have their own
//! `randomDropAsItem`. This module reproduces those draws exactly, including
//! the draws made before a failed explosion chance check and the three
//! `Block.spawnAsEntity` position draws each spawned stack makes, which
//! interleave with the next stack's chance draw.
use crate::{Result, items::ItemStack, legacy_1_8_9};
use horizon_core::random::RandomSource;
use horizon_world::{
    chunk::BlockState,
    nbt::Tag,
    registry::{BlockRegistry, BlockStateId},
};
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path};

/// The 1.8 block-drop table of one prepared resource root.
pub struct BlockDrops {
    rows: HashMap<u16, Row>,
}

/// One 1.8 block state's drop data, as `Block.dropAsItem` sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Row {
    /// `getDropItem` (`None` is 1.8's `null` item).
    pub drop: Option<i16>,
    /// `getMeta(state)`, the damage the dropped item takes.
    pub meta: u8,
    /// `getBonusDrops`/`getDropCount` when [`Bonus::One`] applies.
    pub count: u8,
    /// `createStackFromBlock(state)` for a silk-touch break.
    pub silk: Option<(i16, u8)>,
    /// `requiresSilkTouch()`: only then does silk touch use [`Row::silk`].
    pub silk_required: bool,
    /// The fortune formula `getBonusDrops` overrides.
    pub bonus: Bonus,
}

/// The 1.8 `getBonusDrops` overrides that are not a constant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Bonus {
    /// `Block.getBonusDrops` → `getDropCount` (a constant from the table).
    #[default]
    One,
    /// Coal, diamond, emerald and quartz ore:
    /// `count * (1 + max(0, nextInt(fortune + 2) - 1))`, and no draw when
    /// fortune is 0 or the ore drops itself (iron and gold).
    Ore,
    /// Lapis ore: the ore multiplier times `4 + nextInt(5)`.
    Lapis,
    /// Redstone ore: `(4 + nextInt(2)) + nextInt(fortune + 1)`.
    Redstone,
    /// Glowstone: `clamp((2 + nextInt(3)) + nextInt(fortune + 1), 1, 4)`.
    Glowstone,
    /// Melon: `min(9, (3 + nextInt(5)) + nextInt(fortune + 1))`.
    Melon,
    /// Sea lantern: `clamp((2 + nextInt(2)) + nextInt(fortune + 1), 1, 5)`.
    SeaLantern,
    /// Brown mushroom block: `max(0, nextInt(10) - 7)`.
    MushroomBrown,
    /// Red mushroom block: `max(0, nextInt(10) - 7)`.
    MushroomRed,
}

impl Bonus {
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Ore,
            2 => Self::Lapis,
            3 => Self::Redstone,
            4 => Self::Glowstone,
            5 => Self::Melon,
            6 => Self::SeaLantern,
            7 => Self::MushroomBrown,
            8 => Self::MushroomRed,
            _ => Self::One,
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::One => 0,
            Self::Ore => 1,
            Self::Lapis => 2,
            Self::Redstone => 3,
            Self::Glowstone => 4,
            Self::Melon => 5,
            Self::SeaLantern => 6,
            Self::MushroomBrown => 7,
            Self::MushroomRed => 8,
        }
    }

    /// `getBonusDrops(fortune, random)` for the rules whose count is a draw.
    fn count(self, base: u8, fortune: u32, random: &mut impl RandomSource) -> u32 {
        match self {
            Self::One => u32::from(base),
            Self::Ore => {
                if fortune == 0 {
                    return u32::from(base);
                }
                let mut extra = next_bound(random, fortune as i32 + 2) - 1;
                if extra < 0 {
                    extra = 0;
                }
                u32::from(base) * (extra as u32 + 1)
            }
            Self::Lapis => {
                // `OreBlock.getBonusDrops` draws the fortune multiplier
                // before `getDropCount`'s `4 + nextInt(5)`.
                let multiplier = if fortune > 0 {
                    let mut extra = next_bound(random, fortune as i32 + 2) - 1;
                    if extra < 0 {
                        extra = 0;
                    }
                    extra + 1
                } else {
                    1
                };
                ((4 + next_bound(random, 5)) * multiplier) as u32
            }
            Self::Redstone => {
                let base = 4 + next_bound(random, 2);
                (base + next_bound(random, fortune as i32 + 1)) as u32
            }
            Self::Glowstone => {
                let base = 2 + next_bound(random, 3);
                (base + next_bound(random, fortune as i32 + 1)).clamp(1, 4) as u32
            }
            Self::Melon => {
                let base = 3 + next_bound(random, 5);
                (base + next_bound(random, fortune as i32 + 1)).min(9) as u32
            }
            Self::SeaLantern => {
                let base = 2 + next_bound(random, 2);
                (base + next_bound(random, fortune as i32 + 1)).clamp(1, 5) as u32
            }
            Self::MushroomBrown | Self::MushroomRed => (next_bound(random, 10) - 7).max(0) as u32,
        }
    }
}

fn next_bound(random: &mut impl RandomSource, bound: i32) -> i32 {
    random.next_int_bound(bound).unwrap_or(0)
}

fn property_i32(state: &BlockState, key: &str) -> Option<i32> {
    state.properties.get(key)?.parse().ok()
}

fn stack(id: i16, count: i32, damage: i16) -> Option<ItemStack> {
    legacy_1_8_9::stack(id, count as i8, damage, &Tag::End)
}

/// A stack 1.8's `Block.spawnAsEntity` is about to spawn, with the three
/// position draws that call makes from the level's random the moment the
/// stack is created: `nextFloat() * 0.5 + 0.25` for x, then y, then z. They
/// are carried out because they interleave with the next stack's chance and
/// item draws; spawning in a later pass would read a different stream.
#[derive(Clone, Debug)]
pub struct Popped {
    pub stack: ItemStack,
    /// `None` when the stack is not a 1.8 `spawnAsEntity` drop (a 26.3 loot
    /// table, a block entity's dynamic contents), so the caller draws the
    /// position itself.
    pub pop: Option<[f32; 3]>,
}

impl std::ops::Deref for Popped {
    type Target = ItemStack;
    fn deref(&self) -> &ItemStack {
        &self.stack
    }
}

/// `Block.spawnAsEntity`: the three draws are skipped when the block's drops
/// are switched off (`doTileDrops`), because 1.8 checks the gamerule before
/// drawing them.
fn popped(stack: ItemStack, random: &mut impl RandomSource, spawn: bool) -> Popped {
    if !spawn {
        return Popped { stack, pop: None };
    }
    let x = random.next_float() * 0.5 + 0.25;
    let y = random.next_float() * 0.5 + 0.25;
    let z = random.next_float() * 0.5 + 0.25;
    Popped {
        stack,
        pop: Some([x, y, z]),
    }
}

fn push(out: &mut Vec<Popped>, id: i16, damage: i16, random: &mut impl RandomSource, spawn: bool) {
    if let Some(stack) = stack(id, 1, damage) {
        out.push(popped(stack, random, spawn));
    }
}

/// A `harvest` override that builds its stacks without drawing, then hands
/// each to `Block.spawnAsEntity`.
fn spawn_all(stacks: Vec<ItemStack>, random: &mut impl RandomSource, spawn: bool) -> Vec<Popped> {
    stacks
        .into_iter()
        .map(|stack| popped(stack, random, spawn))
        .collect()
}

/// `Block.spawnAsEntity` for a stack a `harvest` override built outside the
/// drop table: the three position draws, in vanilla's x/y/z order.
pub fn spawn_as_entity(stack: ItemStack, random: &mut impl RandomSource, spawn: bool) -> Popped {
    popped(stack, random, spawn)
}

impl BlockDrops {
    /// Load `block-drops.json` from a prepared root. A root without the file
    /// (a 26.3 one) has no 1.8 table.
    pub fn load(root: &Path) -> Result<Option<Self>> {
        let path = root.join("block-drops.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display()).into()),
        };
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let rows = value["rows"]
            .as_array()
            .ok_or_else(|| format!("{}: missing rows", path.display()))?;
        let mut table = HashMap::with_capacity(rows.len());
        for raw in rows {
            let raw = raw
                .as_array()
                .ok_or_else(|| format!("{}: row is not an array", path.display()))?;
            if raw.len() != 8 {
                return Err(format!("{}: row has {} fields", path.display(), raw.len()).into());
            }
            let number = |i: usize| -> Result<i64> {
                raw[i]
                    .as_i64()
                    .ok_or_else(|| format!("{}: row field {i}", path.display()).into())
            };
            let key = number(0)?;
            if !(0..=0x0FFF).contains(&key) {
                return Err(format!("{}: state key {key}", path.display()).into());
            }
            let drop = match number(1)? {
                -1 => None,
                id => Some(id as i16),
            };
            let silk = match number(4)? {
                -1 => None,
                id => Some((id as i16, number(5)? as u8)),
            };
            table.insert(
                key as u16,
                Row {
                    drop,
                    meta: number(2)? as u8,
                    count: number(3)? as u8,
                    silk,
                    silk_required: number(6)? & 1 != 0,
                    bonus: Bonus::from_code(number(7)? as u8),
                },
            );
        }
        Ok(Some(Self { rows: table }))
    }

    /// The drops of one 1.8 block state.
    ///
    /// `chance` is 1 for a normal break and `1 / power` for an explosion that
    /// decays drops; `fortune` is zero for an explosion, as in
    /// `Explosion` (`Block.dropAsItem(..., fortune 0)`). `breaker` is whether
    /// a player broke the block: only then do `harvest`'s shears and silk
    /// branches run. `spawn` is `doTileDrops`: 1.8 draws each stack's
    /// `spawnAsEntity` position only when the drops are actually spawned.
    #[allow(clippy::too_many_arguments)]
    pub fn drops(
        &self,
        registry: &BlockRegistry,
        state: BlockStateId,
        legacy: (u16, u8),
        held: Option<&ItemStack>,
        random: &mut impl RandomSource,
        chance: f32,
        fortune: u32,
        breaker: bool,
        spawn: bool,
    ) -> Vec<Popped> {
        let Some(block) = registry.state(state) else {
            return Vec::new();
        };
        let (id, meta) = legacy;
        let shears = breaker && held.is_some_and(|h| &*h.item == "minecraft:shears");
        let silk = held.is_some_and(|h| h.enchantment_level("minecraft:silk_touch") > 0);

        // `harvest` overrides that replace the generic path entirely.
        if breaker {
            match id {
                // SnowLayerBlock.harvest: the layer always gives
                // `layers + 1` snowballs, whatever the tool.
                78 => {
                    let layers =
                        property_i32(block, "layers").unwrap_or(i32::from(meta & 0x0F) + 1);
                    let mut out = Vec::new();
                    if let Some(stack) = stack(332, layers + 1, 0) {
                        out.push(stack);
                    }
                    return spawn_all(out, random, spawn);
                }
                // The shears branches: leaves keep their species, tall grass
                // and dead bushes drop themselves, vines drop a vine.
                18 | 161 | 31 | 32 | 106 if shears => {
                    return spawn_all(shears_drop(id, meta), random, spawn);
                }
                // DoublePlantBlock.harvest: shears on the lower half of a
                // grass or fern drop two tall-grass items; flowers fall
                // through to the generic path.
                175 if shears => {
                    if let Some(out) = double_plant_shears(block) {
                        return spawn_all(out, random, spawn);
                    }
                }
                _ => {}
            }
        }

        let Some(row) = self.rows.get(&(id << 4 | u16::from(meta & 0x0F))) else {
            return Vec::new();
        };
        if breaker && row.silk_required && silk {
            let stack = row
                .silk
                .and_then(|(id, meta)| stack(id, 1, i16::from(meta)))
                .into_iter()
                .collect();
            return spawn_all(stack, random, spawn);
        }

        let mut out = Vec::new();
        match id {
            // SkullBlock.randomDropAsItem is a no-op; `onBreaking` drops the
            // typed skull from the tile entity (game/spill.rs).
            144 => {}
            // BedBlock.randomDropAsItem: only the foot half drops, and the
            // super call gets fortune 0 because the head half is skipped.
            26 => {
                if meta & 0x08 == 0 {
                    generic(&mut out, *row, random, chance, 0, spawn);
                }
            }
            // JukeboxBlock.randomDropAsItem: the record belongs to the block
            // entity and stays out of the drops; the block's own drop gets
            // fortune 0.
            84 => generic(&mut out, *row, random, chance, 0, spawn),
            // `BlockPistonMoving.dropBlockAsItemWithChance` drops the block
            // its moving-piston block entity carries, through that block's
            // `dropBlockAsItem(..., 0)`; the world resolves the carried state
            // before rolling (`Game::drop_state_at`). Without the entity
            // vanilla draws nothing, so the table row is a no-op.
            36 => {}
            // BannerBlock.randomDropAsItem/harvest: a banner with a block
            // entity drops itself with the entity's base colour and patterns
            // through `spawnAsEntity`; `Pending::attach` fills those in.
            176 | 177 => push(&mut out, 425, 0, random, spawn),
            // GravelBlock.getDropItem: `nextInt(10 - fortune * 3) == 0`
            // gives flint, otherwise gravel.
            13 => {
                if random.next_float() <= chance {
                    let fortune = fortune.min(3);
                    let bound = 10 - (fortune as i32) * 3;
                    let id = if random.next_int_bound(bound).unwrap_or(1) == 0 {
                        318
                    } else {
                        13
                    };
                    push(&mut out, id, 0, random, spawn);
                }
            }
            // LeavesBlock.randomDropAsItem: a sapling chance, then an apple
            // chance for oak and dark oak. Explosion chance is ignored.
            18 | 161 => leaves(&mut out, id, meta, fortune, random, spawn),
            // TallPlantBlock: `getBonusDrops` is `1 + nextInt(fortune * 2 + 1)`
            // and `getDropItem` is `nextInt(8) == 0 ? wheat seeds : null`.
            31 => {
                let count = 1 + random.next_int_bound(fortune as i32 * 2 + 1).unwrap_or(0);
                for _ in 0..count {
                    if random.next_float() > chance {
                        continue;
                    }
                    if random.next_int_bound(8).unwrap_or(1) == 0 {
                        push(&mut out, 295, 0, random, spawn);
                    }
                }
            }
            59 | 141 | 142 => crop(&mut out, id, block, random, chance, fortune, spawn),
            // StemBlock.randomDropAsItem: the generic attempt drops nothing,
            // then three `nextInt(15) <= age` seed attempts.
            104 | 105 => stem(&mut out, id, block, random, spawn),
            // NetherWartBlock.randomDropAsItem ignores the chance.
            115 => nether_wart(&mut out, block, fortune, random, spawn),
            // CocoaBlock.randomDropAsItem ignores the chance and fortune.
            127 => cocoa(&mut out, block, random, spawn),
            // DoublePlantBlock.getDropItem draws the 1/8 wheat seed for the
            // lower tall grass; the other variants return their item or null
            // without a draw.
            175 if block.properties.get("half").map(String::as_str) == Some("lower")
                && block.name.as_str() == "minecraft:tall_grass" =>
            {
                let count = row.bonus.count(row.count, fortune, random);
                for _ in 0..count {
                    if random.next_float() > chance {
                        continue;
                    }
                    if random.next_int_bound(8).unwrap_or(1) == 0 {
                        push(&mut out, 295, 0, random, spawn);
                    }
                }
            }
            _ => generic(&mut out, *row, random, chance, fortune, spawn),
        }
        out
    }
}

/// The shears branch of `harvest` for leaves, tall grass, dead bushes and
/// vines. 1.8's `Item.fromBlock` keeps the pre-flattening damage: leaves2
/// use damage 0/1 (acacia/dark oak), tall grass uses its type id.
fn shears_drop(id: u16, meta: u8) -> Vec<ItemStack> {
    let (item, damage) = match id {
        18 | 161 => (id as i16, i16::from(meta & 3)),
        31 => (31, i16::from(meta & 3)),
        32 => (32, 0),
        106 => (106, 0),
        _ => return Vec::new(),
    };
    stack(item, 1, damage).into_iter().collect()
}

/// `DoublePlantBlock.harvest` + `method_8753`: two tall-grass items for the
/// lower half of tall grass or a large fern, `None` for every other variant.
pub fn double_plant_shears(block: &BlockState) -> Option<Vec<ItemStack>> {
    if block.properties.get("half").map(String::as_str) != Some("lower") {
        return None;
    }
    let damage = match block.name.as_str() {
        "minecraft:tall_grass" => 1,
        "minecraft:large_fern" => 2,
        _ => return None,
    };
    let mut out = Vec::new();
    if let Some(stack) = stack(31, 2, damage) {
        out.push(stack);
    }
    Some(out)
}

/// `LeavesBlock.randomDropAsItem` and the `Leaves1Block`/`Leaves2Block`
/// apple overrides.
fn leaves(
    out: &mut Vec<Popped>,
    id: u16,
    meta: u8,
    fortune: u32,
    random: &mut impl RandomSource,
    spawn: bool,
) {
    let variant = i16::from(meta & 3);
    let jungle = id == 18 && variant == 3;
    let mut chance = if jungle { 40 } else { 20 };
    if fortune > 0 {
        chance -= 2 << fortune;
        chance = chance.max(10);
    }
    if random.next_int_bound(chance).unwrap_or(1) == 0 {
        // `getMeta` keeps the wood type; leaves2's saplings are damage 4/5.
        let damage = variant + if id == 161 { 4 } else { 0 };
        push(out, 6, damage, random, spawn);
    }
    let mut apple_chance = 200;
    if fortune > 0 {
        apple_chance -= 10 << fortune;
        apple_chance = apple_chance.max(40);
    }
    let drops_apple = (id == 18 && variant == 0) || (id == 161 && variant == 1);
    if drops_apple && random.next_int_bound(apple_chance).unwrap_or(1) == 0 {
        push(out, 260, 0, random, spawn);
    }
}

/// `CropBlock.randomDropAsItem` (+ `PotatoesBlock`'s poisonous potato).
fn crop(
    out: &mut Vec<Popped>,
    id: u16,
    block: &BlockState,
    random: &mut impl RandomSource,
    chance: f32,
    fortune: u32,
    spawn: bool,
) {
    let age = property_i32(block, "age").unwrap_or(0);
    let (produce, seed) = match id {
        59 => (296, 295),
        141 => (391, 391),
        _ => (392, 392),
    };
    // The `super.randomDropAsItem` call: fortune is forced to 0 there.
    if random.next_float() <= chance {
        push(out, if age >= 7 { produce } else { seed }, 0, random, spawn);
    }
    if age >= 7 {
        for _ in 0..(3 + fortune) {
            if random.next_int_bound(15).unwrap_or(15) <= age {
                push(out, seed, 0, random, spawn);
            }
        }
        if id == 142 && random.next_int_bound(50).unwrap_or(1) == 0 {
            push(out, 394, 0, random, spawn);
        }
    }
}

/// `StemBlock.randomDropAsItem`: no generic drop, three seed chances.
fn stem(
    out: &mut Vec<Popped>,
    id: u16,
    block: &BlockState,
    random: &mut impl RandomSource,
    spawn: bool,
) {
    let age = property_i32(block, "age").unwrap_or(0);
    // The super call draws the chance before `getDropItem` returns null;
    // whether the draw passes does not change the (absent) drop.
    let _ = random.next_float();
    let seed = if id == 104 { 361 } else { 362 };
    for _ in 0..3 {
        if random.next_int_bound(15).unwrap_or(15) <= age {
            push(out, seed, 0, random, spawn);
        }
    }
}

/// `NetherWartBlock.randomDropAsItem`, which ignores the explosion chance.
fn nether_wart(
    out: &mut Vec<Popped>,
    block: &BlockState,
    fortune: u32,
    random: &mut impl RandomSource,
    spawn: bool,
) {
    let age = property_i32(block, "age").unwrap_or(0);
    let mut count = 1;
    if age >= 3 {
        count = 2 + random.next_int_bound(3).unwrap_or(0);
        if fortune > 0 {
            count += random.next_int_bound(fortune as i32 + 1).unwrap_or(0);
        }
    }
    for _ in 0..count {
        push(out, 372, 0, random, spawn);
    }
}

/// `CocoaBlock.randomDropAsItem`: 1 brown dye, or 3 when ripe. It ignores the
/// explosion chance.
fn cocoa(out: &mut Vec<Popped>, block: &BlockState, random: &mut impl RandomSource, spawn: bool) {
    let age = property_i32(block, "age").unwrap_or(0);
    let count = if age >= 2 { 3 } else { 1 };
    for _ in 0..count {
        // Brown dye (`Items.DYE`, damage 3 = cocoa beans after flattening).
        push(out, 351, 3, random, spawn);
    }
}

/// `Block.randomDropAsItem` with no item/count override.
fn generic(
    out: &mut Vec<Popped>,
    row: Row,
    random: &mut impl RandomSource,
    chance: f32,
    fortune: u32,
    spawn: bool,
) {
    let count = row.bonus.count(row.count, fortune, random);
    for _ in 0..count {
        // The chance draw happens for every slot, even when the block has no
        // item to drop.
        if random.next_float() > chance {
            continue;
        }
        if let Some(id) = row.drop {
            push(out, id, i16::from(row.meta), random, spawn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_core::random::LegacyRandom;
    use horizon_world::registry::BlockRegistry;
    use serde_json::{Value, json};

    fn registry(states: &[(&str, &[(&str, &str)])]) -> BlockRegistry {
        let mut rows = Vec::new();
        let mut defaults = serde_json::Map::new();
        for (i, (name, props)) in states.iter().enumerate() {
            let mut state = serde_json::Map::new();
            state.insert("id".to_owned(), json!(name));
            if !props.is_empty() {
                let properties: serde_json::Map<String, Value> = props
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), json!(value)))
                    .collect();
                state.insert("properties".to_owned(), Value::Object(properties));
            }
            rows.push(json!([name, false, false, "test", Value::Object(state)]));
            defaults.insert((*name).to_owned(), json!(i));
        }
        let facts = json!({"states": rows, "defaults": defaults});
        BlockRegistry::from_facts_json(facts.to_string().as_bytes()).unwrap()
    }

    fn table(rows: &[(u16, u8, Row)]) -> BlockDrops {
        let mut map = HashMap::new();
        for &(id, meta, row) in rows {
            map.insert(id << 4 | u16::from(meta), row);
        }
        BlockDrops { rows: map }
    }

    fn row(drop: Option<i16>, meta: u8, count: u8, bonus: Bonus) -> Row {
        Row {
            drop,
            meta,
            count,
            silk: None,
            silk_required: false,
            bonus,
        }
    }

    #[test]
    fn bonus_formulas_match_the_1_8_oracle() {
        // Values from `BonusProbe18` (the 1.8 named jar) with
        // `new java.util.Random(0x5eed)` for each call.
        assert_eq!(
            Bonus::Redstone.count(1, 0, &mut LegacyRandom::new(0x5eed)),
            5
        );
        assert_eq!(
            Bonus::Glowstone.count(1, 1, &mut LegacyRandom::new(0x5eed)),
            4
        );
        assert_eq!(Bonus::Melon.count(1, 3, &mut LegacyRandom::new(0x5eed)), 9);
        assert_eq!(
            Bonus::SeaLantern.count(1, 2, &mut LegacyRandom::new(0x5eed)),
            3
        );
        assert_eq!(Bonus::Ore.count(1, 3, &mut LegacyRandom::new(0x5eed)), 3);
        assert_eq!(Bonus::Lapis.count(1, 0, &mut LegacyRandom::new(0x5eed)), 7);
        assert_eq!(Bonus::Lapis.count(1, 3, &mut LegacyRandom::new(0x5eed)), 15);
        assert_eq!(
            Bonus::MushroomBrown.count(1, 0, &mut LegacyRandom::new(0x5eed)),
            1
        );
    }

    #[test]
    fn stone_silk_and_cobblestone() {
        let drops = table(&[(
            1,
            0,
            Row {
                drop: Some(4),
                meta: 0,
                count: 1,
                silk: Some((1, 0)),
                silk_required: true,
                bonus: Bonus::One,
            },
        )]);
        let registry = registry(&[("minecraft:stone", &[])]);
        let mut random = LegacyRandom::new(1);
        let plain = drops.drops(
            &registry,
            BlockStateId(0),
            (1, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(plain.len(), 1);
        assert_eq!(&*plain[0].item, "minecraft:cobblestone");
        let mut pick = ItemStack::new("minecraft:diamond_pickaxe", 1);
        pick.enchant("minecraft:silk_touch", 1);
        let silk = drops.drops(
            &registry,
            BlockStateId(0),
            (1, 0),
            Some(&pick),
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(silk.len(), 1);
        assert_eq!(&*silk[0].item, "minecraft:stone");
    }

    #[test]
    fn bookshelf_drops_three_separate_books() {
        let drops = table(&[(47, 0, row(Some(340), 0, 3, Bonus::One))]);
        let registry = registry(&[("minecraft:bookshelf", &[])]);
        let mut random = LegacyRandom::new(7);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (47, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 3);
        assert!(
            out.iter()
                .all(|s| &*s.item == "minecraft:book" && s.count == 1)
        );
    }

    /// `Block.spawnAsEntity` draws a spawned stack's three position floats
    /// right after that stack's chance and item draws, so a multi-drop
    /// block's offsets interleave with the next stack's chance draw.
    #[test]
    fn pop_offsets_interleave_with_the_next_chance_draw() {
        let drops = table(&[(47, 0, row(Some(340), 0, 3, Bonus::One))]);
        let registry = registry(&[("minecraft:bookshelf", &[])]);
        let mut random = LegacyRandom::new(7);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (47, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 3);
        // Java Random(7): 0.730699003 passes the chance; the offsets are the
        // next three floats scaled into 0.25..0.75.
        let mut expected = LegacyRandom::new(7);
        let _ = expected.next_float();
        let want = [
            expected.next_float() * 0.5 + 0.25,
            expected.next_float() * 0.5 + 0.25,
            expected.next_float() * 0.5 + 0.25,
        ];
        assert_eq!(out[0].pop, Some(want));
        assert!(out.iter().all(|d| d.pop.is_some()));
        // Three stacks, each one chance draw and three position draws.
        let mut after = LegacyRandom::new(7);
        for _ in 0..3 {
            after.next_float();
            for _ in 0..3 {
                after.next_float();
            }
        }
        assert_eq!(random.next_int(), after.next_int());
    }

    /// With `doTileDrops` off, `Block.spawnAsEntity` returns before its
    /// position draws, so only the chance floats are consumed.
    #[test]
    fn no_pop_offsets_when_drops_are_off() {
        let drops = table(&[(47, 0, row(Some(340), 0, 3, Bonus::One))]);
        let registry = registry(&[("minecraft:bookshelf", &[])]);
        let mut random = LegacyRandom::new(7);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (47, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            false,
        );
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|d| d.pop.is_none()));
        let mut after = LegacyRandom::new(7);
        for _ in 0..3 {
            after.next_float();
        }
        assert_eq!(random.next_int(), after.next_int());
    }

    #[test]
    fn explosion_chance_draws_before_the_item() {
        let drops = table(&[(47, 0, row(Some(340), 0, 3, Bonus::One))]);
        let registry = registry(&[("minecraft:bookshelf", &[])]);
        let mut random = LegacyRandom::new(7);
        // A chance of 0.0 skips every drop, but each slot still draws a float.
        let skipped = drops.drops(
            &registry,
            BlockStateId(0),
            (47, 0),
            None,
            &mut random,
            0.0,
            0,
            false,
            true,
        );
        assert!(skipped.is_empty());
    }

    #[test]
    fn redstone_ore_uses_the_bonus_formula() {
        let drops = table(&[(73, 0, row(Some(331), 0, 1, Bonus::Redstone))]);
        let registry = registry(&[("minecraft:redstone_ore", &[])]);
        let mut random = LegacyRandom::new(0x5eed);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (73, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 5);
        assert!(out.iter().all(|s| &*s.item == "minecraft:redstone"));
    }

    #[test]
    fn crop_seed_attempts_only_run_when_ripe() {
        let drops = table(&[(59, 7, row(Some(296), 0, 1, Bonus::One))]);
        let registry = registry(&[("minecraft:wheat", &[("age", "7")])]);
        let mut random = LegacyRandom::new(3);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (59, 7),
            None,
            &mut random,
            1.0,
            3,
            true,
            true,
        );
        assert!(!out.is_empty());
        assert!(out.iter().all(|s| s.count == 1));
        assert_eq!(
            out.iter().filter(|s| &*s.item == "minecraft:wheat").count(),
            1
        );
    }

    #[test]
    fn snow_layer_always_gives_layers_plus_one() {
        let drops = table(&[(78, 0, row(Some(332), 0, 0, Bonus::One))]);
        let registry = registry(&[("minecraft:snow", &[("layers", "4")])]);
        let mut random = LegacyRandom::new(1);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (78, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(&*out[0].item, "minecraft:snowball");
        assert_eq!(out[0].count, 5);
    }

    #[test]
    fn gravel_draws_the_chance_before_the_flint_roll() {
        // `Block.randomDropAsItem` draws the explosion chance, then
        // `GravelBlock.getDropItem` draws `nextInt(10 - fortune * 3)`.
        let drops = table(&[(13, 0, row(Some(13), 0, 1, Bonus::One))]);
        let registry = registry(&[("minecraft:gravel", &[])]);
        // Java Random(3): nextFloat() 0.73105735 passes chance 1, then
        // nextInt(10) is 0, so 1.8 gives flint. A flint roll made first would
        // read 4 and give gravel.
        let mut random = LegacyRandom::new(3);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (13, 0),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(&*out[0].item, "minecraft:flint");
    }

    #[test]
    fn bed_head_skips_every_draw_and_foot_drops_the_bed() {
        let drops = table(&[
            (26, 0, row(Some(355), 0, 1, Bonus::One)),
            (26, 8, row(Some(355), 0, 1, Bonus::One)),
        ]);
        let registry = registry(&[("minecraft:bed", &[])]);
        let mut random = LegacyRandom::new(9);
        let head = drops.drops(
            &registry,
            BlockStateId(0),
            (26, 8),
            None,
            &mut random,
            1.0,
            3,
            true,
            true,
        );
        // BedBlock.randomDropAsItem returns before `super` for the head half,
        // so not even the chance is drawn.
        assert!(head.is_empty());
        let mut untouched = LegacyRandom::new(9);
        assert_eq!(random.next_int(), untouched.next_int());
        let foot = drops.drops(
            &registry,
            BlockStateId(0),
            (26, 0),
            None,
            &mut random,
            1.0,
            3,
            true,
            true,
        );
        assert_eq!(foot.len(), 1);
        assert!(foot[0].item.ends_with("_bed"));
    }

    #[test]
    fn lower_tall_grass_has_the_wheat_seed_chance() {
        let drops = table(&[(175, 2, row(None, 0, 1, Bonus::One))]);
        let registry = registry(&[("minecraft:tall_grass", &[("half", "lower")])]);
        // Java Random(1): nextFloat() 0.7308782 passes chance 1, nextInt(8)
        // is 0: the lower tall grass gives a wheat seed. Random(0)'s roll is
        // 6, so the same break gives nothing.
        let mut random = LegacyRandom::new(1);
        let seed = drops.drops(
            &registry,
            BlockStateId(0),
            (175, 2),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert_eq!(seed.len(), 1);
        assert_eq!(&*seed[0].item, "minecraft:wheat_seeds");
        let mut random = LegacyRandom::new(0);
        let none = drops.drops(
            &registry,
            BlockStateId(0),
            (175, 2),
            None,
            &mut random,
            1.0,
            0,
            true,
            true,
        );
        assert!(none.is_empty());
    }

    #[test]
    fn a_banner_drops_without_drawing() {
        let drops = table(&[(176, 0, row(Some(425), 0, 1, Bonus::One))]);
        let registry = registry(&[("minecraft:standing_banner", &[])]);
        let mut random = LegacyRandom::new(5);
        // The block entity branch drops the banner even when the explosion
        // chance is zero, and draws no random at all.
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (176, 0),
            None,
            &mut random,
            0.0,
            0,
            false,
            true,
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].item.ends_with("_banner"));
        // The banner itself draws nothing, but `spawnAsEntity` still takes its
        // three position floats.
        let mut untouched = LegacyRandom::new(5);
        for _ in 0..3 {
            untouched.next_float();
        }
        assert_eq!(random.next_int(), untouched.next_int());
    }

    #[test]
    fn cocoa_drops_three_brown_dye_when_ripe() {
        let drops = table(&[(127, 0, row(None, 0, 1, Bonus::One))]);
        let registry = registry(&[("minecraft:cocoa", &[("age", "2")])]);
        let mut random = LegacyRandom::new(1);
        let out = drops.drops(
            &registry,
            BlockStateId(0),
            (127, 0),
            None,
            &mut random,
            0.0,
            0,
            true,
            true,
        );
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|s| &*s.item == "minecraft:cocoa_beans"));
    }
}
