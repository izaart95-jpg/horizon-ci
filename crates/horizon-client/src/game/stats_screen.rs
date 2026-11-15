//! The Statistics screen (`StatsScreen`): General, Items and Mobs tabs over
//! the player's statistics, with the Done button.
use super::stats::{SPAWN_EGGS_1_8, canonical_legacy_id};
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    items::ItemStack,
    text::Text,
    ui,
};
use std::{collections::HashSet, fmt::Write as _};

/// `Stats.CUSTOM` in registration order with its formatter.
const CUSTOM: &[(&str, Format)] = &[
    ("leave_game", Format::Default),
    ("play_time", Format::Time),
    ("total_world_time", Format::Time),
    ("time_since_death", Format::Time),
    ("time_since_rest", Format::Time),
    ("sneak_time", Format::Time),
    ("walk_one_cm", Format::Distance),
    ("crouch_one_cm", Format::Distance),
    ("sprint_one_cm", Format::Distance),
    ("walk_on_water_one_cm", Format::Distance),
    ("fall_one_cm", Format::Distance),
    ("climb_one_cm", Format::Distance),
    ("fly_one_cm", Format::Distance),
    ("walk_under_water_one_cm", Format::Distance),
    ("minecart_one_cm", Format::Distance),
    ("boat_one_cm", Format::Distance),
    ("pig_one_cm", Format::Distance),
    ("happy_ghast_one_cm", Format::Distance),
    ("horse_one_cm", Format::Distance),
    ("aviate_one_cm", Format::Distance),
    ("swim_one_cm", Format::Distance),
    ("strider_one_cm", Format::Distance),
    ("nautilus_one_cm", Format::Distance),
    ("jump", Format::Default),
    ("drop", Format::Default),
    ("damage_dealt", Format::Tenths),
    ("damage_dealt_absorbed", Format::Tenths),
    ("damage_dealt_resisted", Format::Tenths),
    ("damage_taken", Format::Tenths),
    ("damage_blocked_by_shield", Format::Tenths),
    ("damage_absorbed", Format::Tenths),
    ("damage_resisted", Format::Tenths),
    ("deaths", Format::Default),
    ("mob_kills", Format::Default),
    ("animals_bred", Format::Default),
    ("player_kills", Format::Default),
    ("fish_caught", Format::Default),
    ("talked_to_villager", Format::Default),
    ("traded_with_villager", Format::Default),
    ("eat_cake_slice", Format::Default),
    ("fill_cauldron", Format::Default),
    ("use_cauldron", Format::Default),
    ("clean_armor", Format::Default),
    ("clean_banner", Format::Default),
    ("clean_shulker_box", Format::Default),
    ("interact_with_brewingstand", Format::Default),
    ("interact_with_beacon", Format::Default),
    ("inspect_dropper", Format::Default),
    ("inspect_hopper", Format::Default),
    ("inspect_dispenser", Format::Default),
    ("play_noteblock", Format::Default),
    ("tune_noteblock", Format::Default),
    ("pot_flower", Format::Default),
    ("trigger_trapped_chest", Format::Default),
    ("open_enderchest", Format::Default),
    ("enchant_item", Format::Default),
    ("play_record", Format::Default),
    ("interact_with_furnace", Format::Default),
    ("interact_with_crafting_table", Format::Default),
    ("open_chest", Format::Default),
    ("sleep_in_bed", Format::Default),
    ("sleep_in_straw_bed", Format::Default),
    ("open_shulker_box", Format::Default),
    ("open_barrel", Format::Default),
    ("interact_with_blast_furnace", Format::Default),
    ("interact_with_smoker", Format::Default),
    ("interact_with_lectern", Format::Default),
    ("interact_with_campfire", Format::Default),
    ("interact_with_cartography_table", Format::Default),
    ("interact_with_loom", Format::Default),
    ("interact_with_stonecutter", Format::Default),
    ("bell_ring", Format::Default),
    ("raid_trigger", Format::Default),
    ("raid_win", Format::Default),
    ("interact_with_anvil", Format::Default),
    ("interact_with_grindstone", Format::Default),
    ("target_hit", Format::Default),
    ("interact_with_smithing_table", Format::Default),
];
#[derive(Clone, Copy)]
enum Format {
    Default,
    Tenths,
    Distance,
    Time,
}
fn two(v: f64) -> String {
    format!("{v:.2}")
}
/// `StatFormatter`.
fn format(f: Format, v: i32) -> String {
    match f {
        Format::Default => {
            // NumberFormat.getIntegerInstance(Locale.US): thousands separators.
            let digits = v.unsigned_abs().to_string();
            let mut out = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(c);
            }
            if v < 0 {
                out.insert(0, '-');
            }
            out
        }
        Format::Tenths => two(f64::from(v) * 0.1),
        Format::Distance => {
            let m = f64::from(v) / 100.0;
            let km = m / 1000.0;
            if km > 0.5 {
                format!("{} km", two(km))
            } else if m > 0.5 {
                format!("{} m", two(m))
            } else {
                format!("{v} cm")
            }
        }
        Format::Time => {
            let s = f64::from(v) / 20.0;
            let (min, h) = (s / 60.0, s / 3600.0);
            let (d, y) = (h / 24.0, h / 24.0 / 365.0);
            if y > 0.5 {
                format!("{} y", two(y))
            } else if d > 0.5 {
                format!("{} d", two(d))
            } else if h > 0.5 {
                format!("{} h", two(h))
            } else if min > 0.5 {
                format!("{} min", two(min))
            } else {
                // Java's double + " s".
                let mut t = String::new();
                let _ = write!(t, "{s:?}");
                format!("{t} s")
            }
        }
    }
}
/// The item columns: (stat type, header sprite).
const COLUMNS: [(&str, &str); 6] = [
    ("mined", "gui/sprites/statistics/block_mined"),
    ("broken", "gui/sprites/statistics/item_broken"),
    ("crafted", "gui/sprites/statistics/item_crafted"),
    ("used", "gui/sprites/statistics/item_used"),
    ("picked_up", "gui/sprites/statistics/item_picked_up"),
    ("dropped", "gui/sprites/statistics/item_dropped"),
];

/// 1.8 `Stats.GENERAL`: the display key, the current counter key and the
/// formatter, in `SimpleStat` declaration order.
const GENERAL_1_8: &[(&str, &str, Format)] = &[
    ("stat.leaveGame", "leave_game", Format::Default),
    ("stat.playOneMinute", "play_time", Format::Time),
    ("stat.timeSinceDeath", "time_since_death", Format::Time),
    ("stat.walkOneCm", "walk_one_cm", Format::Distance),
    ("stat.crouchOneCm", "crouch_one_cm", Format::Distance),
    ("stat.sprintOneCm", "sprint_one_cm", Format::Distance),
    ("stat.swimOneCm", "swim_one_cm", Format::Distance),
    ("stat.fallOneCm", "fall_one_cm", Format::Distance),
    ("stat.climbOneCm", "climb_one_cm", Format::Distance),
    ("stat.flyOneCm", "fly_one_cm", Format::Distance),
    (
        "stat.diveOneCm",
        "walk_under_water_one_cm",
        Format::Distance,
    ),
    ("stat.minecartOneCm", "minecart_one_cm", Format::Distance),
    ("stat.boatOneCm", "boat_one_cm", Format::Distance),
    ("stat.pigOneCm", "pig_one_cm", Format::Distance),
    ("stat.horseOneCm", "horse_one_cm", Format::Distance),
    ("stat.jump", "jump", Format::Default),
    ("stat.drop", "drop", Format::Default),
    ("stat.damageDealt", "damage_dealt", Format::Tenths),
    ("stat.damageTaken", "damage_taken", Format::Tenths),
    ("stat.deaths", "deaths", Format::Default),
    ("stat.mobKills", "mob_kills", Format::Default),
    ("stat.animalsBred", "animals_bred", Format::Default),
    ("stat.playerKills", "player_kills", Format::Default),
    ("stat.fishCaught", "fish_caught", Format::Default),
    ("stat.junkFished", "junk_fished", Format::Default),
    ("stat.treasureFished", "treasure_fished", Format::Default),
    (
        "stat.talkedToVillager",
        "talked_to_villager",
        Format::Default,
    ),
    (
        "stat.tradedWithVillager",
        "traded_with_villager",
        Format::Default,
    ),
    ("stat.cakeSlicesEaten", "eat_cake_slice", Format::Default),
    ("stat.cauldronFilled", "fill_cauldron", Format::Default),
    ("stat.cauldronUsed", "use_cauldron", Format::Default),
    ("stat.armorCleaned", "clean_armor", Format::Default),
    ("stat.bannerCleaned", "clean_banner", Format::Default),
    (
        "stat.brewingstandInteraction",
        "interact_with_brewingstand",
        Format::Default,
    ),
    (
        "stat.beaconInteraction",
        "interact_with_beacon",
        Format::Default,
    ),
    ("stat.dropperInspected", "inspect_dropper", Format::Default),
    ("stat.hopperInspected", "inspect_hopper", Format::Default),
    (
        "stat.dispenserInspected",
        "inspect_dispenser",
        Format::Default,
    ),
    ("stat.noteblockPlayed", "play_noteblock", Format::Default),
    ("stat.noteblockTuned", "tune_noteblock", Format::Default),
    ("stat.flowerPotted", "pot_flower", Format::Default),
    (
        "stat.trappedChestTriggered",
        "trigger_trapped_chest",
        Format::Default,
    ),
    ("stat.enderchestOpened", "open_enderchest", Format::Default),
    ("stat.itemEnchanted", "enchant_item", Format::Default),
    ("stat.recordPlayed", "play_record", Format::Default),
    (
        "stat.furnaceInteraction",
        "interact_with_furnace",
        Format::Default,
    ),
    // `Stat.name` is `stat.craftingTableInteraction`, but the text 1.8 shows
    // is `stat.workbenchInteraction` (its `TranslatableText`).
    (
        "stat.workbenchInteraction",
        "interact_with_crafting_table",
        Format::Default,
    ),
    ("stat.chestOpened", "open_chest", Format::Default),
];

/// 1.8's `TIME_PROVIDER` etc.: the same as [`format`] except that 1.8 spells
/// minutes `m`, not `min`.
fn format_1_8(f: Format, v: i32) -> String {
    if !matches!(f, Format::Time) {
        return format(f, v);
    }
    let s = f64::from(v) / 20.0;
    let (min, h) = (s / 60.0, s / 3600.0);
    let (d, y) = (h / 24.0, h / 24.0 / 365.0);
    if y > 0.5 {
        format!("{} y", two(y))
    } else if d > 0.5 {
        format!("{} d", two(d))
    } else if h > 0.5 {
        format!("{} h", two(h))
    } else if min > 0.5 {
        format!("{} m", two(min))
    } else {
        // Java's `double + " s"`.
        let mut t = String::new();
        let _ = write!(t, "{s:?}");
        format!("{t} s")
    }
}

/// The block ids that have a block item in 1.8 (`Block.REGISTRY` with
/// `Item.fromBlock`), ascending.
#[cfg(test)]
const BLOCK_ITEMS_1_8: &[i16] = &[
    1, 2, 3, 4, 5, 6, 7, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 28, 29, 30,
    31, 32, 33, 35, 37, 38, 39, 40, 41, 42, 44, 45, 46, 47, 48, 49, 50, 52, 53, 54, 56, 57, 58, 60,
    61, 62, 65, 66, 67, 69, 70, 72, 73, 76, 77, 78, 79, 80, 81, 82, 84, 85, 86, 87, 88, 89, 91, 95,
    96, 97, 98, 99, 100, 101, 102, 103, 106, 107, 108, 109, 110, 111, 112, 113, 114, 116, 120, 121,
    122, 123, 126, 128, 129, 130, 131, 133, 134, 135, 136, 137, 138, 139, 143, 145, 146, 147, 148,
    151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169,
    170, 171, 172, 173, 174, 175, 179, 180, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
];

/// 1.8 `Stats.MINE`, in id order: the block items without statistics
/// (`disableStats`: 7 bedrock, 52 spawner, 96 trapdoor, 167 iron trapdoor)
/// and without the rows `method_8294` removed (2 grass, 60 farmland,
/// 62 lit furnace, 91 jack o'lantern).
const MINE_1_8: &[i16] = &[
    1, 3, 4, 5, 6, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 28, 29, 30, 31, 32,
    33, 35, 37, 38, 39, 40, 41, 42, 44, 45, 46, 47, 48, 49, 50, 53, 54, 56, 57, 58, 61, 65, 66, 67,
    69, 70, 72, 73, 76, 77, 78, 79, 80, 81, 82, 84, 85, 86, 87, 88, 89, 95, 97, 98, 99, 100, 101,
    102, 103, 106, 107, 108, 109, 110, 111, 112, 113, 114, 116, 120, 121, 122, 123, 126, 128, 129,
    130, 131, 133, 134, 135, 136, 137, 138, 139, 143, 145, 146, 147, 148, 151, 152, 153, 154, 155,
    156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 168, 169, 170, 171, 172, 173, 174, 175,
    179, 180, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
];

/// 1.8 `Stats.ITEM`: every non-block item id, ascending.
const ITEM_IDS_1_8: &[i16] = &[
    256, 257, 258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274,
    275, 276, 277, 278, 279, 280, 281, 282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293,
    294, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312,
    313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331,
    332, 333, 334, 335, 336, 337, 338, 339, 340, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350,
    351, 352, 353, 354, 355, 356, 357, 358, 359, 360, 361, 362, 363, 364, 365, 366, 367, 368, 369,
    370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 382, 383, 384, 385, 386, 387, 388,
    389, 390, 391, 392, 393, 394, 395, 396, 397, 398, 399, 400, 401, 402, 403, 404, 405, 406, 407,
    408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 427,
    428, 429, 430, 431, 2256, 2257, 2258, 2259, 2260, 2261, 2262, 2263, 2264, 2265, 2266, 2267,
];
#[derive(Default)]
pub(super) struct StatsUi {
    tab: usize,
    /// The list's scrolling: the wheel and the held scrollbar.
    scroll: horizon_client::ui::Scroll,
    sort: Option<(usize, i32)>,
    /// The 1.8 tabs: General, Blocks, Items, Mobs.
    pub(super) tab_1_8: usize,
    /// Each 1.8 list's own `scrollAmount`.
    scroll_1_8: [f32; 4],
    /// The 1.8 lists' comparators (`field_1343`, `field_1344`), Blocks then
    /// Items.
    sort_1_8: [Option<(usize, i32)>; 2],
    /// The 1.8 header column held down, drawn depressed.
    header_pressed_1_8: Option<usize>,
    /// A held 1.8 list or scrollbar: the pointer's last y and the pixels
    /// scrolled per pixel (`yDrag`, `field_1254`).
    drag_1_8: Option<(f32, f32)>,
    /// The 1.8 rows, built once when the screen opens (`Stats.MINE`,
    /// `Stats.ITEM` and `EntityType.SPAWN_EGGS`, each filtered by whether it
    /// has a non-zero statistic).
    blocks_1_8: Vec<i16>,
    items_1_8: Vec<i16>,
    mobs_1_8: Vec<usize>,
    /// The item ids with a `Stats.CRAFTING_STATS` row: a recipe or a cooking
    /// recipe makes them (`craftable_1_8` approximates vanilla's recipe set).
    craftable_1_8: HashSet<i16>,
}
/// `MenuTabBar` geometry: three tabs over at most 400 pixels.
fn tabs(w: f32) -> (f32, f32) {
    let total = w.min(400.0) - 28.0;
    let tab = ((total / 3.0) / 2.0).round() * 2.0;
    let x0 = ((w - tab * 3.0) / 2.0).floor();
    (x0, tab)
}
const TAB_H: f32 = 24.0;
const FOOTER: f32 = 33.0;

/// 1.8's `ListWidget` layout: `yStart` 32, `yEnd` height - 64, rows 220 wide
/// centred, scrollbar at `width / 2 + 124`.
struct Stats189Layout {
    top: f32,
    bottom: f32,
    entry_h: f32,
    header_h: f32,
    max_scroll: f32,
    /// `(int) scrollAmount`: the 1.8 renderer truncates it.
    scroll: f32,
    /// `xStart + width / 2 - rowWidth / 2 + 2`.
    row_x: f32,
    bar_x: f32,
    thumb: f32,
}

/// The header column's icon: Blocks has crafted/used/mined, Items
/// depleted/crafted/used.
fn stats_header_icon_1_8(tab: usize, column: usize) -> &'static str {
    match (tab, column) {
        (1, 0) => "gui/sprites/statistics/item_crafted",
        (1, 1) => "gui/sprites/statistics/item_used",
        (1, 2) => "gui/sprites/statistics/block_mined",
        (2, 0) => "gui/sprites/statistics/item_broken",
        (2, 1) => "gui/sprites/statistics/item_crafted",
        _ => "gui/sprites/statistics/item_used",
    }
}

/// The header column's hover text (`CustomStatsListWidget.getText`).
fn stats_header_key_1_8(tab: usize, column: usize) -> &'static str {
    match (tab, column) {
        (1, 0) => "stat.crafted",
        (1, 1) => "stat.used",
        (1, 2) => "stat.mined",
        (2, 0) => "stat.depleted",
        (2, 1) => "stat.crafted",
        _ => "stat.used",
    }
}

/// One press on a 1.8 header. `ListWidget.handleMouse` calls
/// `clickedHeader` twice per press (once from the press branch, once when it
/// sets up the drag), so the first press on a column lands on ascending (the
/// first call picks the column at -1, the second flips it to 1), the next on
/// descending and the third clears the sort.
fn next_sort_1_8(state: Option<(usize, i32)>, column: usize) -> Option<(usize, i32)> {
    match state {
        Some((sorted, 1)) if sorted == column => Some((column, -1)),
        Some((sorted, _)) if sorted == column => None,
        _ => Some((column, 1)),
    }
}

/// `CustomStatsListWidget`'s comparator: a row with a statistic comes before
/// one without it (`stat == null` returns 1 without the direction), equal
/// values fall back to the lower 1.8 id, and the direction only applies to
/// the values.
fn compare_stats_1_8(
    a: (i16, Option<i32>),
    b: (i16, Option<i32>),
    order: i32,
) -> std::cmp::Ordering {
    match (a.1, b.1) {
        (Some(x), Some(y)) => {
            if x != y {
                let cmp = x.cmp(&y);
                return if order < 0 { cmp.reverse() } else { cmp };
            }
        }
        (None, Some(_)) => return std::cmp::Ordering::Greater,
        (Some(_), None) => return std::cmp::Ordering::Less,
        (None, None) => {}
    }
    a.0.cmp(&b.0)
}

impl Game<'_> {
    pub(super) fn open_stats(&mut self) {
        self.load_stats();
        // StatsScreen.init: a server's client asks for its statistics.
        if self.remote.is_some() {
            self.send(horizon_protocol::packets::play::Serverbound::ClientCommand(
                horizon_protocol::packets::play::ClientCommand { action: 1 },
            ));
        }
        self.stats_ui = StatsUi::default();
        if horizon_core::target::is_1_8_9() {
            self.prepare_stats_1_8();
        }
        self.screen = Screen::Stats;
    }
    /// 1.8 `StatsScreen.onStatsReady`: build the four lists once. They are
    /// filtered by whether they hold a statistic, as vanilla's constructors
    /// do, and sorted only when a header is pressed.
    pub(super) fn prepare_stats_1_8(&mut self) {
        let craftable: HashSet<i16> = {
            let recipes = self.recipes();
            let mut ids = HashSet::new();
            // `Stats.loadCraftingStats` keys the row by `Item.getRawId`, and
            // every row reads its count from the canonical id (`method_8293`),
            // so a jack-o'lantern makes pumpkins craftable.
            for (_, recipe) in &recipes.crafting {
                if let Some(item) = recipe.output_item()
                    && let Some((id, _)) = horizon_items::legacy_1_8_9::legacy_id(item)
                {
                    ids.insert(canonical_legacy_id(id));
                }
            }
            for cooking in &recipes.cooking {
                if let Some((id, _)) = horizon_items::legacy_1_8_9::legacy_id(&cooking.result.item)
                {
                    ids.insert(canonical_legacy_id(id));
                }
            }
            // `SmeltingRecipeRegistry` is 1.8's own table, not the data pack's
            // cooking recipes.
            for product in horizon_client::smelting_1_8_9::products() {
                if let Some((id, _)) = horizon_items::legacy_1_8_9::legacy_id(product) {
                    ids.insert(canonical_legacy_id(id));
                }
            }
            ids
        };
        self.stats_ui.craftable_1_8 = craftable;
        self.stats_ui.blocks_1_8 = MINE_1_8
            .iter()
            .copied()
            .filter(|&id| {
                ["mined", "used", "crafted"]
                    .iter()
                    .any(|ty| self.stats.get_legacy_item(ty, id) > 0)
            })
            .collect();
        self.stats_ui.items_1_8 = ITEM_IDS_1_8
            .iter()
            .copied()
            .filter(|&id| {
                ["used", "broken", "crafted"]
                    .iter()
                    .any(|ty| self.stats.get_legacy_item(ty, id) > 0)
            })
            .collect();
        self.stats_ui.mobs_1_8 = SPAWN_EGGS_1_8
            .iter()
            .enumerate()
            .filter(|(_, (_, id))| {
                self.stats.get("killed", id) > 0 || self.stats.get("killed_by", id) > 0
            })
            .map(|(i, _)| i)
            .collect();
    }
    /// The list's scrollbar column (right of the 280 pixel list) and the
    /// pixels it scrolls.
    fn stats_bar(w: f32) -> f32 {
        ((w - 280.0) / 2.0).floor() + 280.0 + 6.0
    }
    fn stats_content(&self) -> f32 {
        let (row_h, rows) = self.stats_rows();
        row_h * rows as f32 + 4.0
    }
    fn stats_rows(&self) -> (f32, usize) {
        match self.stats_ui.tab {
            0 => (14.0, CUSTOM.len()),
            1 => (22.0, self.stat_items().len() + 1),
            _ => (36.0, self.stat_mobs().len()),
        }
    }
    /// Whether tab `i` can be chosen: the Items and Mobs tabs are active only
    /// while their list has rows.
    fn stats_tab_active(&self, i: usize) -> bool {
        match i {
            1 => !self.stat_items().is_empty(),
            2 => !self.stat_mobs().is_empty(),
            _ => true,
        }
    }
    /// Items (and block items) with any non-zero column.
    fn stat_items(&self) -> Vec<String> {
        let mut items: Vec<String> = Vec::new();
        for (ty, _) in COLUMNS {
            if let Some(m) = self.stats.values.get(&format!("minecraft:{ty}")) {
                for (k, v) in m {
                    if *v > 0 && k != "minecraft:air" && !items.contains(k) {
                        items.push(k.clone());
                    }
                }
            }
        }
        if let Some((col, order)) = self.stats_ui.sort {
            let ty = COLUMNS[col].0;
            items.sort_by_key(|i| self.stats.get(ty, i) * -order);
        }
        items
    }
    fn stat_mobs(&self) -> Vec<String> {
        let mut mobs: Vec<String> = Vec::new();
        for ty in ["killed", "killed_by"] {
            if let Some(m) = self.stats.values.get(&format!("minecraft:{ty}")) {
                for (k, v) in m {
                    if *v > 0 && !mobs.contains(k) {
                        mobs.push(k.clone());
                    }
                }
            }
        }
        mobs
    }
    /// `MobRow`: the mob's name, its kills and killed-by lines, and their
    /// colours (grey when none).
    fn mob_row(&self, mob: &str) -> (Text, Text, Text, (u32, u32)) {
        let name = Text::translate(&format!("entity.{}", mob.replace(':', ".")), &[]);
        let kills = self.stats.get("killed", mob);
        let by = self.stats.get("killed_by", mob);
        let k = if kills == 0 {
            Text::translate(
                "stat_type.minecraft.killed.none",
                std::slice::from_ref(&name),
            )
        } else {
            Text::translate(
                "stat_type.minecraft.killed",
                &[Text::literal(kills.to_string()), name.clone()],
            )
        };
        let b = if by == 0 {
            Text::translate(
                "stat_type.minecraft.killed_by.none",
                std::slice::from_ref(&name),
            )
        } else {
            Text::translate(
                "stat_type.minecraft.killed_by",
                &[name.clone(), Text::literal(by.to_string())],
            )
        };
        let colors = (
            if kills > 0 { 0xFFBA_BABA } else { 0xFF80_8080 },
            if by > 0 { 0xFFBA_BABA } else { 0xFF80_8080 },
        );
        (name, k, b, colors)
    }
    pub(super) fn draw_stats(&mut self, batch: &mut HudBatch) {
        horizon_client::widgets::begin_screen(|| {
            horizon_client::widgets::ScreenInfo::new(
                "stats",
                super::screen_info::tr("gui.stats", &[]),
            )
        });
        if horizon_core::target::is_1_8_9() {
            return self.draw_stats_1_8(batch);
        }
        let (w, h) = (batch.width, batch.height);
        let atlas = &self.gui.atlas;
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, true);
        let (top, bottom) = (TAB_H, h - FOOTER);
        ui::separators(batch, atlas, top, bottom, true);
        // The tab bar.
        let mut tab_tip = false;
        let (x0, tw) = tabs(w);
        const TABS: [&str; 3] = ["stat.generalButton", "stat.itemsButton", "stat.mobsButton"];
        // Its TabNavigationBar: the hovered tab, else the current one (a
        // GridLayoutTab's extra narration is empty).
        horizon_client::widgets::narratable(|| {
            use horizon_client::widgets::{ListKind, Narratable, Row, What};
            let row = |i: usize| {
                let mut row = Row::new(i, Text::translate(TABS[i], &[]).plain());
                row.hint = Some(String::new());
                row
            };
            let hovered = (0..TABS.len())
                .find(|&i| ui::hovered(self.cursor, x0 + tw * i as f32, 0.0, tw, TAB_H));
            Narratable::new(
                What::List {
                    kind: ListKind::Tabs,
                    total: TABS.len(),
                    hovered: hovered.map(row),
                    selected: Some(row(self.stats_ui.tab.min(TABS.len() - 1))),
                },
                [x0, 0.0, tw * TABS.len() as f32, TAB_H],
            )
            .key(horizon_client::widgets::key_of(&"stats/tabs"))
        });
        for (i, key) in TABS.into_iter().enumerate() {
            let x = x0 + tw * i as f32;
            let selected = self.stats_ui.tab == i;
            let active = self.stats_tab_active(i);
            let hover = ui::hovered(self.cursor, x, 0.0, tw, TAB_H);
            if hover && !active {
                tab_tip = true;
            }
            let sprite = match (selected, hover) {
                (true, true) => "gui/sprites/widget/tab_selected_highlighted",
                (true, false) => "gui/sprites/widget/tab_selected",
                (false, true) => "gui/sprites/widget/tab_highlighted",
                (false, false) => "gui/sprites/widget/tab",
            };
            ui::nine_slice(batch, atlas, sprite, x, 0.0, tw, TAB_H, u32::MAX);
            let label = Text::translate(key, &[]);
            let lw = atlas.font.width_text(&label);
            let color = if selected && active {
                u32::MAX
            } else {
                0xFFA0_A0A0
            };
            batch.text_styled(
                atlas,
                &label,
                (x + (tw - lw) / 2.0).floor(),
                8.0,
                color,
                true,
            );
            if selected {
                batch.fill(atlas, x + 2.0, TAB_H - 3.0, tw - 4.0, 1.0, u32::MAX);
            }
        }
        // The list.
        let (row_h, rows) = self.stats_rows();
        let content = row_h * rows as f32 + 4.0;
        self.stats_ui.scroll.clamp(top, bottom, content);
        let left = ((w - 280.0) / 2.0).floor();
        let scroll = self.stats_ui.scroll.amount;
        let row_y = |i: usize| top + 4.0 + i as f32 * row_h - scroll;
        let visible = |y: f32| y >= top && y + row_h <= bottom;
        // The list's row under the mouse (`getHovered`).
        let hovered_row = {
            let (cx, cy) = self.cursor;
            let i = ((cy - (top + 4.0) + scroll) / row_h).floor();
            let inside = cy >= top && cy < bottom && cx >= left && cx < left + 280.0;
            (inside && i >= 0.0 && (i as usize) < rows).then_some(i as usize)
        };
        let list_rect = [0.0, top, w, bottom - top];
        let list_key = horizon_client::widgets::key_of(&("stats/list", self.stats_ui.tab));
        let tr = super::screen_info::tr;
        // GeneralStatisticsList and MobsStatisticsList: ObjectSelectionLists
        // narrating the hovered row (none is selected).
        let object_list = |narration: &dyn Fn(usize) -> String| {
            horizon_client::widgets::narratable(|| {
                use horizon_client::widgets::{ListKind, Narratable, Row};
                let hovered = hovered_row.map(|i| Row::new(i, narration(i)));
                Narratable::list(ListKind::Object, rows, hovered, None, list_rect).key(list_key)
            });
        };
        // The hovered item's tooltip, drawn over the list.
        let mut tip = None;
        match self.stats_ui.tab {
            0 => {
                let mut sorted: Vec<(Text, String)> = CUSTOM
                    .iter()
                    .map(|(k, f)| {
                        let label = Text::translate(&format!("stat.minecraft.{k}"), &[]);
                        let v = self.stats.get("custom", &format!("minecraft:{k}"));
                        (label, format(*f, v))
                    })
                    .collect();
                sorted.sort_by_key(|(l, _)| l.plain());
                object_list(&|i| {
                    let (label, value) = &sorted[i];
                    tr("narrator.select", &[&format!("{} {value}", label.plain())])
                });
                for (i, (label, value)) in sorted.iter().enumerate() {
                    let y = row_y(i);
                    if !visible(y) {
                        continue;
                    }
                    let color = if i % 2 == 0 { u32::MAX } else { 0xFFBA_BABA };
                    let ty = y + (row_h - 8.0) / 2.0;
                    batch.text_styled(atlas, label, left + 2.0, ty.floor(), color, true);
                    let vw = HudBatch::text_width(atlas, value);
                    batch.text(
                        atlas,
                        value,
                        left + 280.0 - vw - 4.0,
                        ty.floor(),
                        color,
                        true,
                    );
                }
            }
            1 => {
                let items = self.stat_items();
                if items.is_empty() {
                    return self.draw_stats_footer(batch, w, h);
                }
                // ItemStatisticsList, a ContainerObjectSelectionList: the
                // header's sort buttons and each row's item.
                horizon_client::widgets::narratable(|| {
                    horizon_client::widgets::Narratable::container_list(
                        rows,
                        hovered_row,
                        list_rect,
                    )
                    .key(list_key)
                });
                // The header row of sort buttons.
                let y = row_y(0);
                if visible(y) {
                    for (c, (ty, sprite)) in COLUMNS.iter().enumerate() {
                        let x = left + 75.0 + 40.0 * c as f32 - 18.0;
                        let hover = ui::hovered(self.cursor, x, y + 1.0, 18.0, 18.0);
                        // StatSortButton: an ImageButton named by its stat
                        // type, which is its tooltip too.
                        horizon_client::widgets::narratable(|| {
                            let name = tr(&format!("stat_type.minecraft.{ty}"), &[]);
                            horizon_client::widgets::Narratable::button(
                                name.clone(),
                                [x, y + 1.0, 18.0, 18.0],
                                true,
                            )
                            .hint(name)
                            .key(horizon_client::widgets::key_of(&("stats/sort", c)))
                            .child_of(list_key, 0)
                        });
                        let bg = if hover {
                            "gui/sprites/statistics/header"
                        } else {
                            "gui/sprites/container/slot"
                        };
                        batch.sprite(atlas, bg, x, y + 1.0);
                        batch.sprite(atlas, sprite, x, y + 1.0);
                        if let Some((sc, order)) = self.stats_ui.sort
                            && sc == c
                        {
                            let s = if order < 0 {
                                "gui/sprites/statistics/sort_down"
                            } else {
                                "gui/sprites/statistics/sort_up"
                            };
                            batch.sprite(atlas, s, x, y + 1.0);
                        }
                    }
                }
                for (i, item) in items.iter().enumerate() {
                    let y = row_y(i + 1);
                    if !visible(y) {
                        continue;
                    }
                    let stack = ItemStack::new(item, 1);
                    // ItemRowWidget, an ItemDisplayWidget: "Item: <name>".
                    horizon_client::widgets::narratable(|| {
                        let names = |id: &str| self.pack.names.display_item(id);
                        let name = horizon_client::stations::hover_name(&names, &stack);
                        horizon_client::widgets::Narratable::text(
                            tr("narration.item", &[&name]),
                            [left + 1.0, y + 1.0, 18.0, 18.0],
                        )
                        .key(horizon_client::widgets::key_of(&("stats/item", item)))
                        .child_of(list_key, i + 1)
                        .without_tooltip()
                    });
                    batch.sprite(
                        &self.gui.atlas,
                        "gui/sprites/container/slot",
                        left + 1.0,
                        y + 1.0,
                    );
                    self.draw_item(batch, &stack, left + 2.0, y + 2.0);
                    // `ItemRowWidget.extractTooltip`: the item's tooltip at the slot's corner.
                    if ui::hovered(self.cursor, left + 1.0, y + 1.0, 18.0, 18.0) {
                        tip = Some((stack.clone(), left + 19.0, y + 19.0));
                    }
                    let atlas = &self.gui.atlas;
                    let color = if (i + 1) % 2 == 0 {
                        u32::MAX
                    } else {
                        0xFFBA_BABA
                    };
                    for (c, (ty, _)) in COLUMNS.iter().enumerate() {
                        let x = left + 75.0 + 40.0 * c as f32;
                        let block = c == 0
                            && self
                                .res
                                .items()
                                .get(item)
                                .is_some_and(|i| i.block.is_some());
                        let text = if c == 0 && !block {
                            Text::translate("stats.none", &[]).plain()
                        } else {
                            format(Format::Default, self.stats.get(ty, item))
                        };
                        let tw = HudBatch::text_width(atlas, &text);
                        batch.text(atlas, &text, x - tw, (y + 7.0).floor(), color, true);
                    }
                }
            }
            _ => {
                let mobs = self.stat_mobs();
                object_list(&|i| {
                    let (_, kills, killed_by, _) = self.mob_row(&mobs[i]);
                    let both = horizon_client::widgets::join_for_narration(&[
                        &kills.plain(),
                        &killed_by.plain(),
                    ]);
                    tr("narrator.select", &[&both])
                });
                for (i, mob) in mobs.iter().enumerate() {
                    let y = row_y(i);
                    if !visible(y) {
                        continue;
                    }
                    let (name, k, b, (ck, cb)) = self.mob_row(mob);
                    batch.text_styled(atlas, &name, left + 2.0, y + 1.0, u32::MAX, true);
                    batch.text_styled(atlas, &k, left + 12.0, y + 10.0, ck, true);
                    batch.text_styled(atlas, &b, left + 12.0, y + 19.0, cb, true);
                }
            }
        }
        let atlas = &self.gui.atlas;
        ui::scrollbar(
            batch,
            atlas,
            Self::stats_bar(w),
            top,
            bottom,
            content,
            self.stats_ui.scroll.amount,
        );
        self.draw_stats_footer(batch, w, h);
        if let Some((stack, x, y)) = tip {
            self.draw_stack_tooltip(batch, &stack, x, y);
        }
        // The empty tab's Tooltip.create(gui.stats.none_found).
        if tab_tip {
            let atlas = &self.gui.atlas;
            let tip = Text::translate("gui.stats.none_found", &[]);
            // Tooltip.splitTooltip: font.split(message, 170).
            ui::tooltip(batch, atlas, &atlas.font.split(&tip, 170.0), self.cursor);
        }
    }
    fn draw_stats_footer(&self, batch: &mut HudBatch, w: f32, h: f32) {
        let atlas = &self.gui.atlas;
        let (bx, by) = ((w / 2.0 - 100.0).floor(), h - FOOTER + 6.0);
        let hover = ui::hovered(self.cursor, bx, by, 200.0, 20.0);
        ui::button(
            batch,
            atlas,
            bx,
            by,
            200.0,
            20.0,
            &Text::translate("gui.done", &[]),
            true,
            hover,
        );
    }
    /// 1.8 `ListWidget` metrics. `scrollAmount` is clipped to the list's
    /// `getMaxScroll` and read as `(int)`: the renderer and its hit tests
    /// truncate it.
    fn stats_1_8_layout(&self, w: f32, h: f32) -> Stats189Layout {
        let tab = self.stats_ui.tab_1_8;
        let (entry_h, header_h, rows) = match tab {
            1 => (20.0, 20.0, self.stats_ui.blocks_1_8.len()),
            2 => (20.0, 20.0, self.stats_ui.items_1_8.len()),
            3 => (36.0, 0.0, self.stats_ui.mobs_1_8.len()),
            _ => (10.0, 0.0, GENERAL_1_8.len()),
        };
        let top = 32.0;
        let bottom = h - 64.0;
        let band = (bottom - top).max(1.0);
        let max_position = rows as f32 * entry_h + header_h;
        let max_scroll = (max_position - (band - 4.0)).max(0.0);
        let thumb = if max_position > 0.0 {
            (band * band / max_position)
                .trunc()
                .clamp(32.0, (band - 8.0).max(32.0))
        } else {
            0.0
        };
        let half = (w / 2.0).floor();
        Stats189Layout {
            top,
            bottom,
            entry_h,
            header_h,
            max_scroll,
            scroll: self.stats_ui.scroll_1_8[tab].clamp(0.0, max_scroll).trunc(),
            row_x: half - 108.0,
            bar_x: half + 124.0,
            thumb,
        }
    }
    /// `Item.isDamageable`: the `BROKEN` column exists only for these.
    fn item_damageable_1_8(&self, id: i16) -> bool {
        horizon_items::legacy_1_8_9::stack(id, 1, 0, &horizon_world::nbt::Tag::End).is_some_and(
            |stack| {
                self.res
                    .items()
                    .get(&stack.item)
                    .is_some_and(|item| item.max_damage > 0)
            },
        )
    }
    /// `CustomStatsListWidget.method_1126` + `Collections.sort`: one column
    /// ascending, then descending, then the id order. That cycle is the
    /// double `clickedHeader` a press on the header gets in `ListWidget`
    /// (`handleMouse` calls it from the press branch and again when it sets
    /// up the drag). A stat the item has no row for (`crafted` without a
    /// recipe, `broken` when unbreakable) sorts after the rows that have
    /// one, whichever the direction.
    fn sort_stats_1_8(&mut self, which: usize, state: Option<(usize, i32)>) {
        let mut ids = if which == 0 {
            std::mem::take(&mut self.stats_ui.blocks_1_8)
        } else {
            std::mem::take(&mut self.stats_ui.items_1_8)
        };
        if let Some((column, order)) = state {
            let mut keyed: Vec<(i16, Option<i32>)> = ids
                .iter()
                .map(|&id| {
                    let (ty, present) = match (which, column) {
                        (0, 0) => ("crafted", self.stats_ui.craftable_1_8.contains(&id)),
                        (0, 1) => ("used", true),
                        (0, 2) => ("mined", true),
                        (1, 0) => ("broken", self.item_damageable_1_8(id)),
                        (1, 1) => ("crafted", self.stats_ui.craftable_1_8.contains(&id)),
                        _ => ("used", true),
                    };
                    (id, present.then(|| self.stats.get_legacy_item(ty, id)))
                })
                .collect();
            keyed.sort_by(|a, b| compare_stats_1_8(*a, *b, order));
            ids = keyed.into_iter().map(|(id, _)| id).collect();
        } else {
            ids.sort_unstable();
        }
        if which == 0 {
            self.stats_ui.blocks_1_8 = ids;
        } else {
            self.stats_ui.items_1_8 = ids;
        }
    }
    /// 1.8 `StatsScreen.render`: the list (with its own background, rows,
    /// holes, gradients, scrollbar and tooltips), then the title and the five
    /// footer buttons.
    fn draw_stats_1_8(&mut self, batch: &mut HudBatch) {
        let (w, h) = (batch.width, batch.height);
        let tab = self.stats_ui.tab_1_8;
        let max_scroll = self.stats_1_8_layout(w, h).max_scroll;
        self.stats_ui.scroll_1_8[tab] = self.stats_ui.scroll_1_8[tab].clamp(0.0, max_scroll);
        let m = self.stats_1_8_layout(w, h);
        // `renderHeader`: the pressed column is held only while the button is.
        if !ui::button_down() {
            self.stats_ui.header_pressed_1_8 = None;
        }
        let atlas = &self.gui.atlas;
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, self.player.is_some());
        let has_background = atlas.sprites.contains_key("gui/options_background");
        // `renderHoleBackground` samples the texture at the hole's absolute y,
        // so the dirt keeps its phase across the whole screen.
        let fill_background = |batch: &mut HudBatch, y: f32, height: f32, tint: u32| {
            if has_background {
                ui::tiled_offset(
                    batch,
                    atlas,
                    "gui/options_background",
                    0.0,
                    y,
                    w,
                    height,
                    [32.0, 32.0],
                    [0.0, y],
                    tint,
                );
            } else {
                batch.fill(atlas, 0.0, y, w, height, tint);
            }
        };
        // `ListWidget.render`'s scrolled, 0.125-tinted options texture.
        if has_background {
            ui::tiled_offset(
                batch,
                atlas,
                "gui/options_background",
                0.0,
                m.top,
                w,
                m.bottom - m.top,
                [32.0, 32.0],
                [0.0, m.scroll],
                rgba(32, 32, 32, 255),
            );
        } else {
            batch.fill(
                atlas,
                0.0,
                m.top,
                w,
                m.bottom - m.top,
                rgba(32, 32, 32, 255),
            );
        }
        let row_y = |i: usize| m.top + 4.0 - m.scroll + m.header_h + i as f32 * m.entry_h;
        let visible = |y: f32| y <= m.bottom && y + m.entry_h - 4.0 >= m.top;
        let header_y = m.top + 4.0 - m.scroll + 1.0;
        // The 1.8 header is only covered by the top hole when it scrolls off.
        if matches!(tab, 1 | 2) && header_y + 18.0 > m.top && header_y < m.bottom {
            for column in 0..3 {
                let pressed = self.stats_ui.header_pressed_1_8 == Some(column);
                let x = m.row_x + 97.0 + 50.0 * column as f32;
                let background = if pressed {
                    "gui/sprites/statistics/slot"
                } else {
                    "gui/sprites/statistics/header"
                };
                batch.sprite(atlas, background, x, header_y);
                let shift = if pressed { 1.0 } else { 0.0 };
                batch.sprite(
                    atlas,
                    stats_header_icon_1_8(tab, column),
                    x + shift,
                    header_y + shift,
                );
            }
            let which = if tab == 1 { 0 } else { 1 };
            if let Some((column, order)) = self.stats_ui.sort_1_8[which] {
                let sprite = if order < 0 {
                    "gui/sprites/statistics/sort_down"
                } else {
                    "gui/sprites/statistics/sort_up"
                };
                batch.sprite(
                    atlas,
                    sprite,
                    m.row_x + 79.0 + 50.0 * column as f32,
                    header_y,
                );
            }
        }
        match tab {
            0 => {
                for (i, (key, counter, formatter)) in GENERAL_1_8.iter().enumerate() {
                    let y = row_y(i);
                    if !visible(y) {
                        continue;
                    }
                    let color = if i % 2 == 0 {
                        u32::MAX
                    } else {
                        rgba(144, 144, 144, 255)
                    };
                    let label = Text::translate(key, &[]).plain();
                    batch.text(atlas, &label, m.row_x + 2.0, y + 1.0, color, true);
                    let value = format_1_8(
                        *formatter,
                        self.stats.get("custom", &format!("minecraft:{counter}")),
                    );
                    let width = HudBatch::text_width(atlas, &value);
                    batch.text(atlas, &value, m.row_x + 215.0 - width, y + 1.0, color, true);
                }
            }
            1 | 2 => {
                let ids = if tab == 1 {
                    &self.stats_ui.blocks_1_8
                } else {
                    &self.stats_ui.items_1_8
                };
                for (i, &id) in ids.iter().enumerate() {
                    let y = row_y(i);
                    if !visible(y) {
                        continue;
                    }
                    let color = if i % 2 == 0 {
                        u32::MAX
                    } else {
                        rgba(144, 144, 144, 255)
                    };
                    let stack =
                        horizon_items::legacy_1_8_9::stack(id, 1, 0, &horizon_world::nbt::Tag::End);
                    batch.sprite(
                        atlas,
                        "gui/sprites/statistics/slot",
                        m.row_x + 41.0,
                        y + 1.0,
                    );
                    if let Some(stack) = &stack {
                        self.draw_item(batch, stack, m.row_x + 42.0, y + 2.0);
                    }
                    let craftable = self.stats_ui.craftable_1_8.contains(&id);
                    let damageable = stack.as_ref().is_some_and(|stack| {
                        self.res
                            .items()
                            .get(&stack.item)
                            .is_some_and(|item| item.max_damage > 0)
                    });
                    let get =
                        |ty: &str| format(Format::Default, self.stats.get_legacy_item(ty, id));
                    let columns: [Option<String>; 3] = if tab == 1 {
                        [
                            craftable.then(|| get("crafted")),
                            Some(get("used")),
                            Some(get("mined")),
                        ]
                    } else {
                        [
                            damageable.then(|| get("broken")),
                            craftable.then(|| get("crafted")),
                            Some(get("used")),
                        ]
                    };
                    for (column, value) in columns.iter().enumerate() {
                        let text = value.clone().unwrap_or_else(|| "-".to_owned());
                        let width = HudBatch::text_width(atlas, &text);
                        batch.text(
                            atlas,
                            &text,
                            m.row_x + 115.0 + 50.0 * column as f32 - width,
                            y + 5.0,
                            color,
                            true,
                        );
                    }
                }
            }
            _ => {
                for (i, &index) in self.stats_ui.mobs_1_8.iter().enumerate() {
                    let y = row_y(i);
                    if !visible(y) {
                        continue;
                    }
                    let (legacy, id) = SPAWN_EGGS_1_8[index];
                    let name = Text::translate(&format!("entity.{legacy}.name"), &[]).plain();
                    let killed = self.stats.get("killed", id);
                    let killed_by = self.stats.get("killed_by", id);
                    let kills = if killed == 0 {
                        Text::translate("stat.entityKills.none", &[Text::literal(&name)]).plain()
                    } else {
                        Text::translate(
                            "stat.entityKills",
                            &[Text::literal(killed.to_string()), Text::literal(&name)],
                        )
                        .plain()
                    };
                    let by = if killed_by == 0 {
                        Text::translate("stat.entityKilledBy.none", &[Text::literal(&name)]).plain()
                    } else {
                        Text::translate(
                            "stat.entityKilledBy",
                            &[Text::literal(&name), Text::literal(killed_by.to_string())],
                        )
                        .plain()
                    };
                    let grey = rgba(144, 144, 144, 255);
                    let dark = rgba(96, 96, 96, 255);
                    batch.text(atlas, &name, m.row_x - 8.0, y + 1.0, u32::MAX, true);
                    batch.text(
                        atlas,
                        &kills,
                        m.row_x + 2.0,
                        y + 10.0,
                        if killed == 0 { dark } else { grey },
                        true,
                    );
                    batch.text(
                        atlas,
                        &by,
                        m.row_x + 2.0,
                        y + 19.0,
                        if killed_by == 0 { dark } else { grey },
                        true,
                    );
                }
            }
        }
        // `renderHoleBackground` above and below the list band.
        fill_background(batch, 0.0, m.top, rgba(64, 64, 64, 255));
        fill_background(batch, m.bottom, h - m.bottom, rgba(64, 64, 64, 255));
        batch.fill_gradient(
            atlas,
            (0.0, m.top, w, 4.0),
            rgba(0, 0, 0, 255),
            rgba(0, 0, 0, 0),
        );
        batch.fill_gradient(
            atlas,
            (0.0, m.bottom - 4.0, w, 4.0),
            rgba(0, 0, 0, 0),
            rgba(0, 0, 0, 255),
        );
        if m.max_scroll > 0.0 {
            let band = m.bottom - m.top;
            // `ListWidget.render`: `p = (int)scroll * (height - thumb) / maxScroll`,
            // all integer arithmetic.
            let thumb_y = (m.scroll as i32 * (band as i32 - m.thumb as i32) / m.max_scroll as i32)
                as f32
                + m.top;
            let thumb_y = thumb_y.max(m.top);
            batch.fill(atlas, m.bar_x, m.top, 6.0, band, rgba(0, 0, 0, 255));
            batch.fill(
                atlas,
                m.bar_x,
                thumb_y,
                6.0,
                m.thumb,
                rgba(128, 128, 128, 255),
            );
            batch.fill(
                atlas,
                m.bar_x,
                thumb_y,
                5.0,
                m.thumb - 1.0,
                rgba(192, 192, 192, 255),
            );
        }
        self.stats_tooltip_1_8(batch, w, &m);
        let title = Text::translate("gui.stats", &[]);
        ui::centered_text(batch, atlas, &title, 0.0, 20.0, w, u32::MAX);
        self.draw_stats_buttons_1_8(batch, w, h);
    }
    /// `CustomStatsListWidget.renderDecorations`: the hovered item's name, or
    /// a header column's name, in the list's flat tooltip.
    fn stats_tooltip_1_8(&self, batch: &mut HudBatch, w: f32, m: &Stats189Layout) {
        let tab = self.stats_ui.tab_1_8;
        if !matches!(tab, 1 | 2) {
            return;
        }
        let (cursor_x, cursor_y) = self.cursor;
        if cursor_y < m.top || cursor_y > m.bottom {
            return;
        }
        let row_left = (w / 2.0).floor() - 110.0;
        let row_right = row_left + 220.0;
        let offset = cursor_y - m.top - m.header_h + m.scroll - 4.0;
        let index = (offset / m.entry_h).floor();
        let rows = if tab == 1 {
            self.stats_ui.blocks_1_8.len()
        } else {
            self.stats_ui.items_1_8.len()
        };
        let over_entry = cursor_x < m.bar_x
            && cursor_x >= row_left
            && cursor_x <= row_right
            && index >= 0.0
            && offset >= 0.0
            && (index as usize) < rows;
        let atlas = &self.gui.atlas;
        if over_entry {
            if cursor_x >= m.row_x + 40.0 && cursor_x <= m.row_x + 60.0 {
                let id = if tab == 1 {
                    self.stats_ui.blocks_1_8[index as usize]
                } else {
                    self.stats_ui.items_1_8[index as usize]
                };
                if let Some(stack) =
                    horizon_items::legacy_1_8_9::stack(id, 1, 0, &horizon_world::nbt::Tag::End)
                {
                    let name = self.pack.names.display_item(&stack.item);
                    let name = name.trim();
                    if !name.is_empty() {
                        ui::legacy_tooltip(batch, atlas, name, self.cursor);
                    }
                }
            }
            return;
        }
        let column = if cursor_x >= m.row_x + 97.0 && cursor_x <= m.row_x + 115.0 {
            Some(0)
        } else if cursor_x >= m.row_x + 147.0 && cursor_x <= m.row_x + 165.0 {
            Some(1)
        } else if cursor_x >= m.row_x + 197.0 && cursor_x <= m.row_x + 215.0 {
            Some(2)
        } else {
            None
        };
        if let Some(column) = column {
            let text = Text::translate(stats_header_key_1_8(tab, column), &[]).plain();
            let text = text.trim();
            if !text.is_empty() {
                ui::legacy_tooltip(batch, atlas, text, self.cursor);
            }
        }
    }
    /// `StatsScreen.initButtons`: Done, then General, Blocks, Items, Mobs;
    /// the three lists that can be empty are disabled when they are.
    fn draw_stats_buttons_1_8(&self, batch: &mut HudBatch, w: f32, h: f32) {
        let atlas = &self.gui.atlas;
        let half = (w / 2.0).floor();
        let hovered =
            |x: f32, y: f32, width: f32, height: f32| ui::hovered(self.cursor, x, y, width, height);
        ui::button(
            batch,
            atlas,
            half + 4.0,
            h - 28.0,
            150.0,
            20.0,
            &Text::translate("gui.done", &[]),
            true,
            hovered(half + 4.0, h - 28.0, 150.0, 20.0),
        );
        let tabs: [(&str, bool); 4] = [
            ("stat.generalButton", true),
            ("stat.blocksButton", !self.stats_ui.blocks_1_8.is_empty()),
            ("stat.itemsButton", !self.stats_ui.items_1_8.is_empty()),
            ("stat.mobsButton", !self.stats_ui.mobs_1_8.is_empty()),
        ];
        for (i, (key, active)) in tabs.into_iter().enumerate() {
            let x = half - 160.0 + 80.0 * i as f32;
            let y = h - 52.0;
            ui::button(
                batch,
                atlas,
                x,
                y,
                80.0,
                20.0,
                &Text::translate(key, &[]),
                active,
                active && hovered(x, y, 80.0, 20.0),
            );
        }
    }
    fn stats_click_1_8(&mut self, size: [u32; 2]) {
        let batch = HudBatch::new(size);
        let (w, h) = (batch.width, batch.height);
        let half = (w / 2.0).floor();
        // `StatsScreen.handleMouse` runs the buttons before the active list.
        if ui::hovered(self.cursor, half + 4.0, h - 28.0, 150.0, 20.0) {
            self.audio.play_button_click();
            self.screen = Screen::Paused;
            return;
        }
        let tabs: [(&str, bool); 4] = [
            ("stat.generalButton", true),
            ("stat.blocksButton", !self.stats_ui.blocks_1_8.is_empty()),
            ("stat.itemsButton", !self.stats_ui.items_1_8.is_empty()),
            ("stat.mobsButton", !self.stats_ui.mobs_1_8.is_empty()),
        ];
        for (i, (_, active)) in tabs.into_iter().enumerate() {
            let x = half - 160.0 + 80.0 * i as f32;
            if active && ui::hovered(self.cursor, x, h - 52.0, 80.0, 20.0) {
                // `ButtonWidget.mousePressed` plays the press sound before
                // `buttonClicked`, even when the tab is already the active one.
                self.stats_ui.tab_1_8 = i;
                self.audio.play_button_click();
                return;
            }
        }
        let tab = self.stats_ui.tab_1_8;
        let m = self.stats_1_8_layout(w, h);
        let (cursor_x, cursor_y) = self.cursor;
        if matches!(tab, 1 | 2) && cursor_y >= m.top && cursor_y <= m.bottom {
            let row_left = (w / 2.0).floor() - 110.0;
            let row_right = row_left + 220.0;
            let offset = cursor_y - m.top - m.header_h + m.scroll - 4.0;
            if offset < 0.0 && cursor_x >= row_left && cursor_x <= row_right {
                let relative = cursor_x - ((w - 220.0) / 2.0).floor();
                let column = if (79.0..115.0).contains(&relative) {
                    Some(0)
                } else if (129.0..165.0).contains(&relative) {
                    Some(1)
                } else if (179.0..215.0).contains(&relative) {
                    Some(2)
                } else {
                    None
                };
                // `clickedHeader` consumes the press even between columns:
                // it clears the held column and stops the list dragging.
                self.stats_ui.header_pressed_1_8 = column;
                if let Some(column) = column {
                    let which = if tab == 1 { 0 } else { 1 };
                    let state = next_sort_1_8(self.stats_ui.sort_1_8[which], column);
                    self.stats_ui.sort_1_8[which] = state;
                    self.sort_stats_1_8(which, state);
                    self.audio.play_button_click();
                }
                return;
            }
        }
        // A press in the list starts `yDrag`; the scrollbar's pixels-per-pixel
        // factor is `maxScroll / (height - thumb)`.
        if cursor_y >= m.top && cursor_y <= m.bottom {
            let band = m.bottom - m.top;
            let factor = if cursor_x >= m.bar_x && cursor_x <= m.bar_x + 6.0 && m.max_scroll > 0.0 {
                -m.max_scroll / (band - m.thumb).max(1.0)
            } else {
                1.0
            };
            self.stats_ui.drag_1_8 = Some((cursor_y, factor));
        }
    }
    fn stats_scroll_1_8(&mut self, amount: f32) {
        if amount == 0.0 {
            return;
        }
        let (w, h) = self.gui_size();
        let m = self.stats_1_8_layout(w, h);
        // `isMouseInList`: the wheel only scrolls while over the band.
        if self.cursor.1 < m.top || self.cursor.1 > m.bottom {
            return;
        }
        let tab = self.stats_ui.tab_1_8;
        let direction = if amount > 0.0 { -1.0 } else { 1.0 };
        let step = (m.entry_h / 2.0).floor() * direction;
        let scroll = &mut self.stats_ui.scroll_1_8[tab];
        *scroll = (*scroll + step).clamp(0.0, m.max_scroll);
    }
    fn stats_drag_1_8(&mut self) {
        let Some((last_y, factor)) = self.stats_ui.drag_1_8 else {
            return;
        };
        let (w, h) = self.gui_size();
        let m = self.stats_1_8_layout(w, h);
        // `ListWidget.handleMouse` ignores pointer movement outside the list.
        if self.cursor.1 < m.top || self.cursor.1 > m.bottom {
            return;
        }
        let tab = self.stats_ui.tab_1_8;
        let scroll = &mut self.stats_ui.scroll_1_8[tab];
        *scroll = (*scroll - (self.cursor.1 - last_y) * factor).clamp(0.0, m.max_scroll);
        self.stats_ui.drag_1_8 = Some((self.cursor.1, factor));
    }
    pub(super) fn stats_click(&mut self, size: [u32; 2]) {
        if horizon_core::target::is_1_8_9() {
            self.stats_click_1_8(size);
            return;
        }
        // A press on the scrollbar holds it and clicks nothing else.
        if self.stats_press_scrollbar(size) {
            return;
        }
        let b = HudBatch::new(size);
        let (w, h) = (b.width, b.height);
        let (x0, tw) = tabs(w);
        for i in 0..3 {
            if ui::hovered(self.cursor, x0 + tw * i as f32, 0.0, tw, TAB_H)
                && self.stats_ui.tab != i
                && self.stats_tab_active(i)
            {
                self.stats_ui.tab = i;
                self.stats_ui.scroll.amount = 0.0;
                self.audio
                    .play("minecraft:ui.button.click", None, 0.25, 1.0);
                return;
            }
        }
        if ui::hovered(
            self.cursor,
            (w / 2.0 - 100.0).floor(),
            h - FOOTER + 6.0,
            200.0,
            20.0,
        ) {
            self.audio
                .play("minecraft:ui.button.click", None, 0.25, 1.0);
            self.screen = Screen::Paused;
            return;
        }
        if self.stats_ui.tab == 1 {
            let left = ((w - 280.0) / 2.0).floor();
            let y = TAB_H + 4.0 - self.stats_ui.scroll.amount;
            for c in 0..COLUMNS.len() {
                let x = left + 75.0 + 40.0 * c as f32 - 18.0;
                if ui::hovered(self.cursor, x, y + 1.0, 18.0, 18.0) {
                    // sortByColumn: descending, ascending, off.
                    self.stats_ui.sort = match self.stats_ui.sort {
                        Some((sc, -1)) if sc == c => Some((c, 1)),
                        Some((sc, _)) if sc == c => None,
                        _ => Some((c, -1)),
                    };
                    self.audio
                        .play("minecraft:ui.button.click", None, 0.25, 1.0);
                }
            }
        }
    }
    pub(super) fn stats_scroll(&mut self, amount: f32) {
        if horizon_core::target::is_1_8_9() {
            self.stats_scroll_1_8(amount);
            return;
        }
        let (row_h, _) = self.stats_rows();
        let h = self.gui_size().1;
        let content = self.stats_content();
        self.stats_ui
            .scroll
            .wheel(amount, row_h / 2.0, TAB_H, h - FOOTER, content);
    }
    /// A press on the statistics list's scrollbar holds it
    /// (`AbstractSelectionList.updateScrolling`).
    pub(super) fn stats_press_scrollbar(&mut self, size: [u32; 2]) -> bool {
        if horizon_core::target::is_1_8_9() {
            return false;
        }
        let b = HudBatch::new(size);
        let (w, h) = (b.width, b.height);
        let content = self.stats_content();
        self.stats_ui
            .scroll
            .press(self.cursor, Self::stats_bar(w), TAB_H, h - FOOTER, content)
    }
    /// The pointer moved while the scrollbar is held.
    pub(super) fn stats_drag(&mut self) {
        if horizon_core::target::is_1_8_9() {
            self.stats_drag_1_8();
            return;
        }
        if !self.stats_ui.scroll.held() {
            return;
        }
        let h = self.gui_size().1;
        let content = self.stats_content();
        self.stats_ui
            .scroll
            .drag(self.cursor.1, TAB_H, h - FOOTER, content);
    }
    /// The button came up: the scrollbar lets go.
    pub(super) fn stats_release(&mut self) {
        if horizon_core::target::is_1_8_9() {
            self.stats_ui.drag_1_8 = None;
            self.stats_ui.header_pressed_1_8 = None;
            return;
        }
        self.stats_ui.scroll.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1.8's `Stats.MINE`: every block item without a disabled or aliased row.
    #[test]
    fn one_eight_block_rows_are_the_block_items_minus_the_aliases() {
        let rows: Vec<i16> = BLOCK_ITEMS_1_8
            .iter()
            .copied()
            .filter(|id| !matches!(id, 7 | 52 | 96 | 167)) // disableStats
            .filter(|id| !matches!(id, 2 | 60 | 62 | 91)) // methods_8293/8294
            .collect();
        assert_eq!(rows, MINE_1_8);
    }

    /// 1.8's `Stats.ITEM`: every registered item that is not a `BlockItem`,
    /// so ids 256..=431 (426 has no item) and the twelve records.
    #[test]
    fn one_eight_item_rows_are_the_non_block_items() {
        let rows: Vec<i16> = (256..=431)
            .filter(|id| *id != 426)
            .chain(2256..=2267)
            .collect();
        assert_eq!(rows, ITEM_IDS_1_8);
    }

    #[test]
    fn one_eight_general_tab_is_the_simple_stats_in_order() {
        assert_eq!(GENERAL_1_8.len(), 48);
        let keys: Vec<&str> = GENERAL_1_8.iter().map(|(key, _, _)| *key).collect();
        assert_eq!(keys.first(), Some(&"stat.leaveGame"));
        assert_eq!(keys.get(1), Some(&"stat.playOneMinute"));
        assert_eq!(keys.get(2), Some(&"stat.timeSinceDeath"));
        assert_eq!(keys.get(15), Some(&"stat.jump"));
        assert_eq!(keys.last(), Some(&"stat.chestOpened"));
        // The crafting table shows `stat.workbenchInteraction`, not the id
        // the `SimpleStat` is registered under.
        assert!(keys.contains(&"stat.workbenchInteraction"));
        assert!(!keys.contains(&"stat.craftingTableInteraction"));
    }

    #[test]
    fn one_eight_mob_rows_are_the_spawn_eggs_in_registration_order() {
        assert_eq!(SPAWN_EGGS_1_8.len(), 27);
        assert_eq!(SPAWN_EGGS_1_8[0].0, "Creeper");
        assert_eq!(SPAWN_EGGS_1_8[1].0, "Skeleton");
        assert_eq!(SPAWN_EGGS_1_8[16].0, "Pig");
        assert_eq!(SPAWN_EGGS_1_8[26].0, "Villager");
    }

    /// The double `clickedHeader` a press gets: ascending, descending, off.
    #[test]
    fn one_eight_header_presses_start_ascending_then_flip_then_clear() {
        let mut state = None;
        state = next_sort_1_8(state, 1);
        assert_eq!(state, Some((1, 1)));
        state = next_sort_1_8(state, 1);
        assert_eq!(state, Some((1, -1)));
        state = next_sort_1_8(state, 1);
        assert_eq!(state, None);
        state = next_sort_1_8(state, 1);
        assert_eq!(state, Some((1, 1)));
        // A different column starts at ascending too.
        state = next_sort_1_8(state, 2);
        assert_eq!(state, Some((2, 1)));
    }

    /// `CustomStatsListWidget`'s comparator: the direction applies to the
    /// values, a missing statistic sorts last either way, and ties keep the
    /// lower id first.
    #[test]
    fn one_eight_stats_comparator_sorts_missing_last_and_ties_by_id() {
        use std::cmp::Ordering::{Greater, Less};
        assert_eq!(compare_stats_1_8((10, Some(1)), (11, Some(2)), 1), Less);
        assert_eq!(compare_stats_1_8((10, Some(1)), (11, Some(2)), -1), Greater);
        assert_eq!(compare_stats_1_8((10, Some(1)), (11, Some(1)), 1), Less);
        assert_eq!(compare_stats_1_8((10, Some(1)), (11, Some(1)), -1), Less);
        assert_eq!(compare_stats_1_8((10, None), (11, Some(0)), 1), Greater);
        assert_eq!(compare_stats_1_8((10, None), (11, Some(0)), -1), Greater);
        assert_eq!(compare_stats_1_8((10, Some(0)), (11, None), 1), Less);
        assert_eq!(compare_stats_1_8((10, Some(0)), (11, None), -1), Less);
        assert_eq!(compare_stats_1_8((10, None), (11, None), -1), Less);
    }

    #[test]
    fn one_eight_time_formatter_spells_minutes_m() {
        use Format::{Distance, Tenths, Time};
        assert_eq!(format_1_8(Time, 0), "0.0 s");
        assert_eq!(format_1_8(Time, 1), "0.05 s");
        assert_eq!(format_1_8(Time, 600), "30.0 s");
        assert_eq!(format_1_8(Time, 601), "0.50 m");
        assert_eq!(format_1_8(Time, 720_000), "10.00 h");
        assert_eq!(format_1_8(Time, 4_320_000), "2.50 d");
        assert_eq!(format_1_8(Time, 630_720_000), "1.00 y");
        assert_eq!(format_1_8(Tenths, 5), "0.50");
        assert_eq!(format_1_8(Tenths, 15), "1.50");
        assert_eq!(format_1_8(Distance, 5), "5 cm");
        assert_eq!(format_1_8(Distance, 60), "0.60 m");
        assert_eq!(format_1_8(Distance, 123_456), "1.23 km");
        assert_eq!(format_1_8(Format::Default, 1234), "1,234");
        assert_eq!(format_1_8(Format::Default, -1234), "-1,234");
    }
}
