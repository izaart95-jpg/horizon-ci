//! The 1.8.9 Achievements screen (`AchievementsScreen`) and its popup
//! (`AchievementNotification`).
//!
//! 1.8 has 34 achievements on a fixed 24-pixel grid, drawn over a generated
//! block map; a challenge's frame is purple and a locked one fades with its
//! distance to the nearest taken ancestor. The popup slides in from the top
//! right when a server reports a new achievement.
use super::{Game, Screen};
use horizon_client::{
    hud::{HudBatch, rgba},
    items::ItemStack,
    lang,
    text::Text,
    ui,
    widgets::{self, ScreenInfo},
};
use horizon_core::random::{LegacyRandom, RandomSource};
use horizon_world::nbt::Tag;
use std::time::{Duration, Instant};

/// One `AchievementsAndCriterions` entry, in the order vanilla creates them
/// (later entries draw over earlier ones).
pub(super) struct Achievement {
    /// `Stat.name` after `achievement.`; its translation keys are
    /// `achievement.<key>` and `achievement.<key>.desc`.
    pub(super) key: &'static str,
    column: i32,
    row: i32,
    /// The logo's 1.8 raw item name and damage.
    item: &'static str,
    damage: i16,
    /// The parent's index in [`ACHIEVEMENTS`].
    parent: Option<usize>,
    challenge: bool,
}

impl Achievement {
    pub(super) fn title(&self) -> Text {
        Text::translate(&format!("achievement.{}", self.key), &[])
    }
    /// `Achievement.getDescription`: the translated description, with the
    /// Taking Inventory hint's `%1$s` filled by the inventory key.
    pub(super) fn description(&self, inventory_key: &str) -> String {
        if self.key == "openInventory" {
            let key = horizon_client_core::key_display::display_name_1_8_9(inventory_key);
            return Text::translate("achievement.openInventory.desc", &[key]).plain();
        }
        Text::translate(&format!("achievement.{}.desc", self.key), &[]).plain()
    }
    /// The achievement's logo stack.
    fn icon(&self) -> Option<ItemStack> {
        horizon_items::legacy_1_8_9::named_stack(self.item, 1, self.damage, &Tag::End)
    }
}

/// `AchievementsAndCriterions`' static block.
pub(super) const ACHIEVEMENTS: [Achievement; 34] = [
    Achievement {
        key: "openInventory",
        column: 0,
        row: 0,
        item: "minecraft:book",
        damage: 0,
        parent: None,
        challenge: false,
    },
    Achievement {
        key: "mineWood",
        column: 2,
        row: 1,
        item: "minecraft:log",
        damage: 0,
        parent: Some(0),
        challenge: false,
    },
    Achievement {
        key: "buildWorkBench",
        column: 4,
        row: -1,
        item: "minecraft:crafting_table",
        damage: 0,
        parent: Some(1),
        challenge: false,
    },
    Achievement {
        key: "buildPickaxe",
        column: 4,
        row: 2,
        item: "minecraft:wooden_pickaxe",
        damage: 0,
        parent: Some(2),
        challenge: false,
    },
    Achievement {
        key: "buildFurnace",
        column: 3,
        row: 4,
        item: "minecraft:furnace",
        damage: 0,
        parent: Some(3),
        challenge: false,
    },
    Achievement {
        key: "acquireIron",
        column: 1,
        row: 4,
        item: "minecraft:iron_ingot",
        damage: 0,
        parent: Some(4),
        challenge: false,
    },
    Achievement {
        key: "buildHoe",
        column: 2,
        row: -3,
        item: "minecraft:wooden_hoe",
        damage: 0,
        parent: Some(2),
        challenge: false,
    },
    Achievement {
        key: "makeBread",
        column: -1,
        row: -3,
        item: "minecraft:bread",
        damage: 0,
        parent: Some(6),
        challenge: false,
    },
    Achievement {
        key: "bakeCake",
        column: 0,
        row: -5,
        item: "minecraft:cake",
        damage: 0,
        parent: Some(6),
        challenge: false,
    },
    Achievement {
        key: "buildBetterPickaxe",
        column: 6,
        row: 2,
        item: "minecraft:stone_pickaxe",
        damage: 0,
        parent: Some(3),
        challenge: false,
    },
    Achievement {
        key: "cookFish",
        column: 2,
        row: 6,
        item: "minecraft:cooked_fish",
        damage: 0,
        parent: Some(4),
        challenge: false,
    },
    Achievement {
        key: "onARail",
        column: 2,
        row: 3,
        item: "minecraft:rail",
        damage: 0,
        parent: Some(5),
        challenge: true,
    },
    Achievement {
        key: "buildSword",
        column: 6,
        row: -1,
        item: "minecraft:wooden_sword",
        damage: 0,
        parent: Some(2),
        challenge: false,
    },
    Achievement {
        key: "killEnemy",
        column: 8,
        row: -1,
        item: "minecraft:bone",
        damage: 0,
        parent: Some(12),
        challenge: false,
    },
    Achievement {
        key: "killCow",
        column: 7,
        row: -3,
        item: "minecraft:leather",
        damage: 0,
        parent: Some(12),
        challenge: false,
    },
    Achievement {
        key: "flyPig",
        column: 9,
        row: -3,
        item: "minecraft:saddle",
        damage: 0,
        parent: Some(14),
        challenge: true,
    },
    Achievement {
        key: "snipeSkeleton",
        column: 7,
        row: 0,
        item: "minecraft:bow",
        damage: 0,
        parent: Some(13),
        challenge: true,
    },
    Achievement {
        key: "diamonds",
        column: -1,
        row: 5,
        item: "minecraft:diamond_ore",
        damage: 0,
        parent: Some(5),
        challenge: false,
    },
    Achievement {
        key: "diamondsToYou",
        column: -1,
        row: 2,
        item: "minecraft:diamond",
        damage: 0,
        parent: Some(17),
        challenge: false,
    },
    Achievement {
        key: "portal",
        column: -1,
        row: 7,
        item: "minecraft:obsidian",
        damage: 0,
        parent: Some(17),
        challenge: false,
    },
    Achievement {
        key: "ghast",
        column: -4,
        row: 8,
        item: "minecraft:ghast_tear",
        damage: 0,
        parent: Some(19),
        challenge: true,
    },
    Achievement {
        key: "blazeRod",
        column: 0,
        row: 9,
        item: "minecraft:blaze_rod",
        damage: 0,
        parent: Some(19),
        challenge: false,
    },
    Achievement {
        key: "potion",
        column: 2,
        row: 8,
        item: "minecraft:potion",
        damage: 0,
        parent: Some(21),
        challenge: false,
    },
    Achievement {
        key: "theEnd",
        column: 3,
        row: 10,
        item: "minecraft:ender_eye",
        damage: 0,
        parent: Some(21),
        challenge: true,
    },
    Achievement {
        key: "theEnd2",
        column: 4,
        row: 13,
        item: "minecraft:dragon_egg",
        damage: 0,
        parent: Some(23),
        challenge: true,
    },
    Achievement {
        key: "enchantments",
        column: -4,
        row: 4,
        item: "minecraft:enchanting_table",
        damage: 0,
        parent: Some(17),
        challenge: false,
    },
    Achievement {
        key: "overkill",
        column: -4,
        row: 1,
        item: "minecraft:diamond_sword",
        damage: 0,
        parent: Some(25),
        challenge: true,
    },
    Achievement {
        key: "bookcase",
        column: -3,
        row: 6,
        item: "minecraft:bookshelf",
        damage: 0,
        parent: Some(25),
        challenge: false,
    },
    Achievement {
        key: "breedCow",
        column: 7,
        row: -5,
        item: "minecraft:wheat",
        damage: 0,
        parent: Some(14),
        challenge: false,
    },
    Achievement {
        key: "spawnWither",
        column: 7,
        row: 12,
        item: "minecraft:skull",
        damage: 1,
        parent: Some(24),
        challenge: false,
    },
    Achievement {
        key: "killWither",
        column: 7,
        row: 10,
        item: "minecraft:nether_star",
        damage: 0,
        parent: Some(29),
        challenge: false,
    },
    Achievement {
        key: "fullBeacon",
        column: 7,
        row: 8,
        item: "minecraft:beacon",
        damage: 0,
        parent: Some(30),
        challenge: true,
    },
    Achievement {
        key: "exploreAllBiomes",
        column: 4,
        row: 8,
        item: "minecraft:diamond_boots",
        damage: 0,
        parent: Some(23),
        challenge: true,
    },
    Achievement {
        key: "overpowered",
        column: 6,
        row: 4,
        item: "minecraft:golden_apple",
        damage: 1,
        parent: Some(9),
        challenge: true,
    },
];

/// The achievement a key names.
pub(super) fn by_key(key: &str) -> Option<usize> {
    ACHIEVEMENTS.iter().position(|a| a.key == key)
}

/// `StatHandler.method_8298`: the achievement is taken.
fn taken(stats: &super::stats::Stats, index: usize) -> bool {
    stats.get("achievement", ACHIEVEMENTS[index].key) > 0
}

/// `StatHandler.method_1735`: the parent is taken, or there is none.
fn unlockable(stats: &super::stats::Stats, index: usize) -> bool {
    ACHIEVEMENTS[index]
        .parent
        .is_none_or(|parent| taken(stats, parent))
}

/// `StatHandler.method_8303`: taken is 0; otherwise the number of ancestors
/// until a taken one (or the root).
fn distance(stats: &super::stats::Stats, index: usize) -> i32 {
    if taken(stats, index) {
        return 0;
    }
    let mut distance = 0;
    let mut parent = ACHIEVEMENTS[index].parent;
    while let Some(p) = parent {
        if taken(stats, p) {
            break;
        }
        parent = ACHIEVEMENTS[p].parent;
        distance += 1;
    }
    distance
}

/// `MinecraftClient.getSession().getUuid().hashCode()`: Java `String.hashCode`
/// of the dashed UUID string.
fn session_hash(uuid: [i32; 4]) -> i32 {
    let hex: String = uuid.iter().map(|v| format!("{:08x}", *v as u32)).collect();
    let text = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    text.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// The block an achievement-map tile shows (`method_6761`): the tile's
/// `random.nextInt(1 + r + z) + (r + z) / 2` and the row's depth.
fn map_block(value: i32, depth: i32, diamond: bool) -> &'static str {
    if value > 37 || depth == 35 {
        "block/bedrock"
    } else if value == 22 {
        if diamond {
            "block/diamond_ore"
        } else {
            "block/redstone_ore"
        }
    } else if value == 10 {
        "block/iron_ore"
    } else if value == 8 {
        "block/coal_ore"
    } else if value > 4 {
        "block/stone"
    } else if value > 0 {
        "block/dirt"
    } else {
        "block/sand"
    }
}

/// `AchievementNotification`'s vertical offset for a progress `d` of the
/// three seconds it lives: `None` once it is over, else the y it slides from.
fn notification_offset(d: f64, permanent: bool) -> Option<f64> {
    let d = if permanent {
        d.min(0.5)
    } else {
        if !(0.0..=1.0).contains(&d) {
            return None;
        }
        d
    };
    let mut e = d * 2.0;
    if e > 1.0 {
        e = 2.0 - e;
    }
    e *= 4.0;
    e = 1.0 - e;
    if e < 0.0 {
        e = 0.0;
    }
    e *= e;
    e *= e;
    Some(-(e * 36.0).trunc())
}

/// The achievements a stack taken from a crafting grid earns, in
/// `SlotCrafting.onTake`'s order.
fn crafted_achievements(item: &str) -> Vec<&'static str> {
    let pickaxe = item.ends_with("_pickaxe");
    let mut keys = Vec::new();
    if item == "minecraft:crafting_table" {
        keys.push("buildWorkBench");
    }
    if pickaxe {
        keys.push("buildPickaxe");
    }
    if item == "minecraft:furnace" {
        keys.push("buildFurnace");
    }
    if item.ends_with("_hoe") {
        keys.push("buildHoe");
    }
    if item == "minecraft:bread" {
        keys.push("makeBread");
    }
    if item == "minecraft:cake" {
        keys.push("bakeCake");
    }
    // `ItemPickaxe.getToolMaterial() != Item.ToolMaterial.WOOD`
    if pickaxe && item != "minecraft:wooden_pickaxe" {
        keys.push("buildBetterPickaxe");
    }
    if item.ends_with("_sword") {
        keys.push("buildSword");
    }
    if item == "minecraft:enchanting_table" {
        keys.push("enchantments");
    }
    if item == "minecraft:bookshelf" {
        keys.push("bookcase");
    }
    // `Item.itemGoldenApple` with damage 1, the notch apple.
    if item == "minecraft:enchanted_golden_apple" {
        keys.push("overpowered");
    }
    keys
}

/// `FurnaceResultSlot.checkTakeAchievements`: an iron ingot or either cooked
/// fish is enough.
fn furnace_achievements(item: &str) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if item == "minecraft:iron_ingot" {
        keys.push("acquireIron");
    }
    if matches!(item, "minecraft:cooked_cod" | "minecraft:cooked_salmon") {
        keys.push("cookFish");
    }
    keys
}

/// Horizon's custom-payload channel for achievement counters on the internal
/// protocol: the current one has no achievement registry to name them with.
pub(super) const CHANNEL: &str = "horizon:achievements";

/// `ServerPlayer.changeDimension`'s achievement: entering the End, the exit
/// portal out of it, and every other portal trip.
pub(super) fn dimension_achievement(from: super::Dim, target: super::Dim) -> &'static str {
    match (from, target) {
        (super::Dim::Overworld, super::Dim::End) => "theEnd",
        (super::Dim::End, super::Dim::Overworld) => "theEnd2",
        _ => "portal",
    }
}

/// A payload of `(key, counter)` entries: `kind` 0 syncs silently, 1 pops a
/// new one up.
fn achievement_payload(
    kind: u8,
    entries: &[(&str, i32)],
) -> horizon_protocol::packets::configuration::CustomPayload {
    let mut w = horizon_protocol::buf::Writer::new();
    w.u8(kind);
    w.varint(entries.len() as i32);
    for (key, count) in entries {
        w.string(key);
        w.varint(*count);
    }
    horizon_protocol::packets::configuration::CustomPayload {
        channel: CHANNEL.to_owned(),
        data: w.data,
    }
}

/// The whole achievement half of a statistics answer (a window opening the
/// achievements screen asks for one).
pub(super) fn achievement_sync(
    counters: &[(String, i32)],
) -> horizon_protocol::packets::play::Clientbound {
    let entries: Vec<(&str, i32)> = counters
        .iter()
        .map(|(key, count)| (key.as_str(), *count))
        .collect();
    horizon_protocol::packets::play::Clientbound::CustomPayload(achievement_payload(0, &entries))
}

/// What the achievement popup currently shows.
struct Shown {
    index: usize,
    title: String,
    name: String,
    /// When it starts sliding in (`displayRaw` waits 2.5 s).
    at: Instant,
    permanent: bool,
}

/// `AchievementNotification`, plus the per-connection `statsReceived` flag.
#[derive(Default)]
pub(super) struct Notification {
    shown: Option<Shown>,
    /// `ClientPlayNetworkHandler.field_7920`: the first statistics response
    /// suppresses popups and arms the Taking Inventory hint.
    pub(super) stats_received: bool,
}

impl Notification {
    /// `AchievementNotification.display`.
    pub(super) fn display(&mut self, index: usize) {
        self.shown = Some(Shown {
            index,
            title: lang::get("achievement.get"),
            name: ACHIEVEMENTS[index].title().plain(),
            at: Instant::now(),
            permanent: false,
        });
    }
    /// `AchievementNotification.displayRaw`: a permanent hint, starting 2.5 s
    /// from now.
    pub(super) fn display_raw(&mut self, index: usize, title: String, name: String) {
        self.shown = Some(Shown {
            index,
            title,
            name,
            at: Instant::now() + Duration::from_millis(2500),
            permanent: true,
        });
    }
    /// `AchievementNotification.reset`.
    pub(super) fn reset(&mut self) {
        self.shown = None;
    }
}

/// The screen's pan and zoom (`AchievementsScreen`'s fields).
pub(super) struct AchUi {
    /// `downloadingStats`: true until the server answers the stats request.
    pub(super) downloading: bool,
    /// `field_7789`.
    zoom: f32,
    /// `field_1312`/`field_1313`: the pan the last tick started from.
    last: (f64, f64),
    /// `field_1314`/`field_1315`: the current pan.
    pan: (f64, f64),
    /// `field_1316`/`field_1317`: where the pan eases to.
    target: (f64, f64),
    /// `field_1310`/`field_1311`: the last drag position inside the map.
    drag: Option<(f32, f32)>,
    /// `field_1322`: the drag has recorded its first position and pans from
    /// the next movement on.
    panning: bool,
    /// When the last tick ran, for the render's partial tick.
    tick_at: Option<Instant>,
    /// The achievements' logos.
    icons: Vec<Option<ItemStack>>,
}

impl Default for AchUi {
    fn default() -> Self {
        Self {
            downloading: false,
            zoom: 1.0,
            last: (-82.0, -70.0),
            pan: (-82.0, -70.0),
            target: (-82.0, -70.0),
            drag: None,
            panning: false,
            tick_at: None,
            icons: Vec::new(),
        }
    }
}

impl AchUi {
    fn new(downloading: bool) -> Self {
        Self {
            downloading,
            icons: ACHIEVEMENTS.iter().map(Achievement::icon).collect(),
            ..Self::default()
        }
    }
    /// `StatsScreen.onStatsReady`.
    pub(super) fn on_stats_ready(&mut self) {
        self.downloading = false;
    }
}

/// `(width - 256) / 2`, `(height - 202) / 2` in whole GUI pixels.
fn window(w: f32, h: f32) -> (i32, i32) {
    ((w as i32 - 256) / 2, (h as i32 - 202) / 2)
}

/// `AchievementsScreen`'s pan bounds: `minColumn * 24 - 112`,
/// `minRow * 24 - 112`, `maxColumn * 24 - 77`, `maxRow * 24 - 77`.
const PAN_MIN: (f64, f64) = (-208.0, -232.0);
const PAN_MAX: (f64, f64) = (138.0, 234.0);

impl Game<'_> {
    /// `MinecraftClient.openScreen(new AchievementsScreen(parent, stats))`.
    pub(super) fn open_achievements(&mut self) {
        self.load_stats();
        // A 1.8 server answers `ClientStatus.REQUEST_STATS`; the integrated
        // server thread speaks the current protocol, so it never does.
        let downloading = self.remote.as_ref().is_some_and(|r| r.legacy_protocol);
        self.ach_ui = AchUi::new(downloading);
        if self.remote.is_some() {
            self.send(horizon_protocol::packets::play::Serverbound::ClientCommand(
                horizon_protocol::packets::play::ClientCommand { action: 1 },
            ));
        }
        self.screen = Screen::Advancements;
        self.capture(false);
    }
    /// `EntityPlayer.addStat(achievement, 1)`'s counter: it goes up every
    /// time. Returns whether this take is the first. 1.8 has no achievements
    /// under the current protocol, only these counters.
    fn add_achievement(&mut self, key: &str) -> bool {
        if !horizon_core::target::is_1_8_9() || by_key(key).is_none() {
            return false;
        }
        let first = self.stats.get("achievement", key) == 0;
        self.stats.add("achievement", key, 1);
        first
    }
    /// `EntityPlayer.addStat` for this game's own player: a first take also
    /// pops the notification up.
    pub(super) fn grant_achievement(&mut self, key: &str) -> bool {
        let first = self.add_achievement(key);
        if first && let Some(index) = by_key(key) {
            self.ach_notify.display(index);
        }
        first
    }
    /// `EntityPlayer.addStat` for the player acting in this game. A LAN or
    /// dedicated server has no window of its own: the acting guest's window
    /// hears it as a payload. Everywhere else this game draws the popup.
    pub(super) fn award_achievement(&mut self, key: &'static str) {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        if self.lan_acting.is_some() {
            // Inside a guest's turn (their `as_guest`, or a command run for
            // them): the payload leaves with their packets.
            let earned = self.server_achievement(key);
            if !earned.is_empty() {
                self.guest_packets.push(
                    horizon_protocol::packets::play::Clientbound::CustomPayload(
                        achievement_payload(1, &earned),
                    ),
                );
            }
            return;
        }
        self.grant_achievement(key);
    }
    /// `ItemStack.onCraftedBy`'s crafted counter, then the achievements
    /// `SlotCrafting.onTake` (`furnace == false`) or `SlotFurnace`'s take
    /// (`furnace == true`) grants for the taken stack. Returns each
    /// achievement with its new counter, for the window's statistics answer.
    pub(super) fn server_result_taken(
        &mut self,
        furnace: bool,
        item: &str,
        count: i32,
    ) -> Vec<(&'static str, i32)> {
        if !horizon_core::target::is_1_8_9() {
            return Vec::new();
        }
        self.stats.add("crafted", item, count);
        let keys = if furnace {
            furnace_achievements(item)
        } else {
            crafted_achievements(item)
        };
        keys.into_iter()
            .filter_map(|key| {
                self.add_achievement(key);
                by_key(key).map(|_| (key, self.stats.get("achievement", key)))
            })
            .collect()
    }
    /// `EntityPlayer.addStat(achievement, 1)` on a server for a trigger other
    /// than a crafting take: the new counter when it changed, for the
    /// window's statistics answer.
    pub(super) fn server_achievement(&mut self, key: &'static str) -> Vec<(&'static str, i32)> {
        if !horizon_core::target::is_1_8_9() || by_key(key).is_none() {
            return Vec::new();
        }
        self.add_achievement(key);
        vec![(key, self.stats.get("achievement", key))]
    }
    /// `EntityPlayer.addStat` for a result taken from this game's own menu (the
    /// in-frame single player, which is its own server): the counters move and
    /// a first take pops up.
    pub(super) fn on_result_taken(&mut self, furnace: bool, item: &str, count: i32) {
        for (key, taken) in self.server_result_taken(furnace, item, count) {
            self.pop_achievement(key, taken);
        }
    }
    /// Draws the notification of a first take on the game that shows the
    /// window (`AchievementMap`/`GuiAchievement` queue the popup from the
    /// statistics answer; this Game is the one drawing it).
    pub(super) fn pop_achievement(&mut self, key: &str, count: i32) {
        if count == 1
            && let Some(index) = by_key(key)
        {
            self.ach_notify.display(index);
        }
    }
    /// `EntityItem.playerTouch`'s achievements for the picker: a log, leather,
    /// a diamond and a blaze rod each have one.
    pub(super) fn pickup_achievements(item: &str) -> Vec<&'static str> {
        let mut keys = Vec::new();
        if matches!(
            item,
            "minecraft:oak_log"
                | "minecraft:spruce_log"
                | "minecraft:birch_log"
                | "minecraft:jungle_log"
                | "minecraft:acacia_log"
                | "minecraft:dark_oak_log"
        ) {
            keys.push("mineWood");
        }
        if item == "minecraft:leather" {
            keys.push("killCow");
        }
        if item == "minecraft:diamond" {
            keys.push("diamonds");
        }
        if item == "minecraft:blaze_rod" {
            keys.push("blazeRod");
        }
        keys
    }
    /// `EntityItem.playerTouch`'s `diamondsToYou`: a diamond another player
    /// threw, picked up by a different player, credits the thrower. The
    /// thrower may be the game's own player, a LAN guest's record, or this
    /// window's own player on the server thread.
    pub(super) fn diamond_to_you(&mut self, thrower: [i32; 4], picker: [i32; 4]) {
        if thrower == picker || !horizon_core::target::is_1_8_9() {
            return;
        }
        if self.lan.is_none() {
            // Single player: the only thrower is this player.
            return;
        }
        let Some(mut host) = self.lan.take() else {
            return;
        };
        self.credit_diamond_thrower(&mut host, thrower);
        self.lan = Some(host);
    }
    /// `diamond_to_you` with the LAN host already split off `self.lan`.
    pub(super) fn credit_diamond_thrower(
        &mut self,
        host: &mut super::lan::LanHost,
        thrower: [i32; 4],
    ) {
        let guest = host
            .hub
            .guests
            .iter()
            .position(|g| horizon_client::lan::playerdata::uuid_ints(g.uuid) == thrower);
        let Some(gi) = guest else {
            // The host's own player threw it, if it is still here.
            if thrower == self.level.ensure_player_uuid() {
                self.grant_achievement("diamondsToYou");
            }
            return;
        };
        let conn = host.hub.guests[gi].conn;
        let Some(play) = host.play.get_mut(&conn) else {
            return;
        };
        play.stats.add("achievement", "diamondsToYou", 1);
        let count = play.stats.get("achievement", "diamondsToYou");
        self.send_result_achievements(host, conn, &[("diamondsToYou", count)]);
    }
    /// The achievements the server sends a window after a result take (the
    /// current protocol has no achievement registry to name them with).
    pub(super) fn send_result_achievements(
        &self,
        host: &mut super::lan::LanHost,
        conn: u32,
        entries: &[(&'static str, i32)],
    ) {
        if entries.is_empty() {
            return;
        }
        let packet = horizon_protocol::packets::play::Clientbound::CustomPayload(
            achievement_payload(1, entries),
        );
        host.hub.send_to(conn, &packet);
    }
    /// The window's side of [`Self::send_result_achievements`]: save each
    /// counter and pop the newly taken achievement up.
    pub(super) fn receive_achievements(&mut self, data: &[u8]) {
        let mut r = horizon_protocol::buf::Reader::new(data);
        let Ok(kind) = r.u8() else {
            return;
        };
        let Ok(n) = r.varint() else {
            return;
        };
        for _ in 0..n.clamp(0, 64) {
            let Ok(key) = r.string(64) else {
                return;
            };
            let Ok(count) = r.varint() else {
                return;
            };
            let Some(index) = by_key(&key) else {
                continue;
            };
            let first = self.stats.get("achievement", &key) == 0 && count > 0;
            self.stats
                .values
                .entry("minecraft:achievement".to_owned())
                .or_default()
                .insert(key.clone(), count);
            if first && kind == 1 {
                self.ach_notify.display(index);
                if key == "openInventory" {
                    self.settings.extras.show_inventory_achievement_hint = false;
                    self.settings.save(&Self::options_path());
                }
            }
        }
    }
    /// `Minecraft.runTick`'s inventory key: `C16PacketClientStatus.OPEN_INVENTORY`.
    /// The 1.8 client sends it whenever the inventory key opens the inventory
    /// screen (creative included), and a horse's inventory has its own
    /// `EntityAction`, sent by the mount path.
    pub(super) fn open_inventory_achievement(&mut self) {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        if self.remote.is_some() {
            // 1.8's action 2; ours is 3 (2 is request-game-rules).
            self.send(horizon_protocol::packets::play::Serverbound::ClientCommand(
                horizon_protocol::packets::play::ClientCommand { action: 3 },
            ));
        } else {
            // This game is the server (in-frame, or the LAN host's own player).
            self.grant_achievement("openInventory");
        }
    }
    /// `Screen.keyPressed`'s Escape and the inventory key: back to the game
    /// (1.8 does not return to the pause screen).
    pub(super) fn close_achievements(&mut self) {
        self.ach_ui.drag = None;
        self.screen = Screen::Playing;
        self.capture(true);
    }
    /// `AchievementsScreen.tick`: ease the pan toward its target.
    pub(super) fn tick_achievements(&mut self) {
        if !horizon_core::target::is_1_8_9()
            || self.screen != Screen::Advancements
            || self.ach_ui.downloading
        {
            return;
        }
        let ui = &mut self.ach_ui;
        ui.last = ui.pan;
        let (dx, dy) = (ui.target.0 - ui.pan.0, ui.target.1 - ui.pan.1);
        if dx * dx + dy * dy < 4.0 {
            ui.pan.0 += dx;
            ui.pan.1 += dy;
        } else {
            ui.pan.0 += dx * 0.85;
            ui.pan.1 += dy * 0.85;
        }
        ui.tick_at = Some(Instant::now());
    }
    /// `AchievementsScreen.render`: the generated map, connectors, frames,
    /// the frame overlay, the Done button and the hovered tooltip.
    pub(super) fn draw_achievements(&mut self, batch: &mut HudBatch) {
        widgets::begin_screen(|| {
            ScreenInfo::new(
                "achievements",
                super::screen_info::tr("gui.achievements", &[]),
            )
        });
        let (w, h) = (batch.width, batch.height);
        let in_world = self.player.is_some();
        {
            let atlas = &self.gui.atlas;
            ui::menu_background(batch, atlas, 0.0, 0.0, w, h, in_world);
        }
        if self.ach_ui.downloading {
            let atlas = &self.gui.atlas;
            let text = Text::translate("multiplayer.downloadingStats", &[]);
            ui::centered_text(batch, atlas, &text, 0.0, (h / 2.0).floor(), w, u32::MAX);
            // StatsListener.PROGRESS_BAR_STAGES, one every 150 ms.
            const STAGES: [&str; 7] = [
                "oooooo", "Oooooo", "oOoooo", "ooOooo", "oooOoo", "ooooOo", "oooooO",
            ];
            let stage = (self.started.elapsed().as_millis() / 150) as usize % STAGES.len();
            let text = Text::literal(STAGES[stage]);
            ui::centered_text(
                batch,
                atlas,
                &text,
                0.0,
                (h / 2.0 + 18.0).floor(),
                w,
                u32::MAX,
            );
            return;
        }
        // `field_1316`/`field_1317` are clamped every frame.
        self.ach_ui.target.0 = self.ach_ui.target.0.clamp(PAN_MIN.0, PAN_MAX.0);
        self.ach_ui.target.1 = self.ach_ui.target.1.clamp(PAN_MIN.1, PAN_MAX.1);
        let partial = self.ach_ui.tick_at.map_or(1.0, |at| {
            (at.elapsed().as_secs_f64() / 0.05).clamp(0.0, 1.0)
        });
        let k = (self.ach_ui.last.0 + (self.ach_ui.pan.0 - self.ach_ui.last.0) * partial).floor()
            as i32;
        let l = (self.ach_ui.last.1 + (self.ach_ui.pan.1 - self.ach_ui.last.1) * partial).floor()
            as i32;
        let k = k.clamp(PAN_MIN.0 as i32, PAN_MAX.0 as i32);
        let l = l.clamp(PAN_MIN.1 as i32, PAN_MAX.1 as i32);
        let (m, n) = window(w, h);
        let (o, p) = (m + 16, n + 17);
        let zoom = self.ach_ui.zoom;
        let stats = &self.stats;
        let icons = &self.ach_ui.icons;
        let atlas = &self.gui.atlas;
        // The generated block map: 16-pixel tiles from (k, l).
        let q = (k + 288) >> 4;
        let r = (l + 288) >> 4;
        let s = (k + 288) % 16;
        let t = (l + 288) % 16;
        let step = 16.0 / zoom;
        // `MinecraftClient.getSession().getUuid()`: the connection's profile,
        // or the world's player while no connection exists.
        let uuid = self.remote.as_ref().map_or_else(
            || self.level.player_uuid.unwrap_or([0; 4]),
            |r| {
                [
                    (r.profile >> 96) as u32 as i32,
                    (r.profile >> 64) as u32 as i32,
                    (r.profile >> 32) as u32 as i32,
                    r.profile as u32 as i32,
                ]
            },
        );
        let seed = session_hash(uuid);
        let mut z = 0;
        while (z as f32) * step - (t as f32) > -16.0 && (z as f32) * step - (t as f32) < 155.0 {
            let depth = r + z;
            let shade = 0.6 - depth as f32 / 25.0 * 0.3;
            let tint = rgba(
                (shade * 255.0) as u8,
                (shade * 255.0) as u8,
                (shade * 255.0) as u8,
                255,
            );
            let mut ab = 0;
            while (ab as f32) * step - (s as f32) > -16.0 && (ab as f32) * step - (s as f32) < 224.0
            {
                let mut random = LegacyRandom::new(i64::from(
                    seed.wrapping_add(q + ab)
                        .wrapping_add((r + z).wrapping_mul(16)),
                ));
                let value = random.next_int_bound(1 + r + z).unwrap_or(0) + (r + z) / 2;
                let diamond = random.next_int_bound(2).unwrap_or(0) == 0;
                let sprite = map_block(value, depth, diamond);
                let x = o as f32 + (ab * 16 - s) as f32 / zoom;
                let y = p as f32 + (z * 16 - t) as f32 / zoom;
                batch.sprite_scaled(atlas, sprite, x, y, step, step, tint);
                ab += 1;
            }
            z += 1;
        }
        // The connectors: a horizontal run at the child's row and a vertical
        // one at the parent's column, then the corner caps.
        for (i, achievement) in ACHIEVEMENTS.iter().enumerate() {
            let Some(parent) = achievement.parent else {
                continue;
            };
            let parent = &ACHIEVEMENTS[parent];
            let ax = achievement.column * 24 - k + 11;
            let ay = achievement.row * 24 - l + 11;
            let bx = parent.column * 24 - k + 11;
            let by = parent.row * 24 - l + 11;
            let is_taken = taken(stats, i);
            let is_unlockable = unlockable(stats, i);
            if distance(stats, i) > 4 {
                continue;
            }
            let color = if is_taken {
                rgba(160, 160, 160, 255)
            } else if is_unlockable {
                rgba(0, 255, 0, 255)
            } else {
                rgba(0, 0, 0, 255)
            };
            let (x0, x1) = if ax <= bx { (ax, bx) } else { (bx, ax) };
            batch.fill(
                atlas,
                o as f32 + x0 as f32 / zoom,
                p as f32 + ay as f32 / zoom,
                (x1 - x0 + 1) as f32 / zoom,
                1.0 / zoom,
                color,
            );
            let (y0, y1) = if ay <= by { (ay, by) } else { (by, ay) };
            batch.fill(
                atlas,
                o as f32 + bx as f32 / zoom,
                p as f32 + y0 as f32 / zoom,
                1.0 / zoom,
                (y1 - y0 + 1) as f32 / zoom,
                color,
            );
            let cap = if ax > bx {
                (
                    "gui/sprites/achievement/connector_left",
                    ax - 18,
                    ay - 5,
                    7,
                    11,
                )
            } else if ax < bx {
                (
                    "gui/sprites/achievement/connector_right",
                    ax + 11,
                    ay - 5,
                    7,
                    11,
                )
            } else if ay > by {
                (
                    "gui/sprites/achievement/connector_up",
                    ax - 5,
                    ay - 18,
                    11,
                    7,
                )
            } else if ay < by {
                (
                    "gui/sprites/achievement/connector_down",
                    ax - 5,
                    ay + 11,
                    11,
                    7,
                )
            } else {
                continue;
            };
            batch.sprite_scaled(
                atlas,
                cap.0,
                o as f32 + cap.1 as f32 / zoom,
                p as f32 + cap.2 as f32 / zoom,
                cap.3 as f32 / zoom,
                cap.4 as f32 / zoom,
                u32::MAX,
            );
        }
        // The frames and their logos; the last frame under the cursor wins.
        let (mouse_x, mouse_y) = self.cursor;
        let map_x = (mouse_x - o as f32) * zoom;
        let map_y = (mouse_y - p as f32) * zoom;
        let mut hovered = None;
        for (i, achievement) in ACHIEVEMENTS.iter().enumerate() {
            let ax = achievement.column * 24 - k;
            let ay = achievement.row * 24 - l;
            if ax < -24 || ay < -24 || ax as f32 > 224.0 * zoom || ay as f32 > 155.0 * zoom {
                continue;
            }
            let is_taken = taken(stats, i);
            let is_unlockable = unlockable(stats, i);
            let distance = distance(stats, i);
            let shade = if is_taken {
                0.75
            } else if is_unlockable {
                1.0
            } else if distance < 3 {
                0.3
            } else if distance == 3 {
                0.2
            } else if distance == 4 {
                0.1
            } else {
                continue;
            };
            let grey = (shade * 255.0) as u8;
            let frame = if achievement.challenge {
                "gui/sprites/achievement/frame_challenge"
            } else {
                "gui/sprites/achievement/frame"
            };
            batch.sprite_scaled(
                atlas,
                frame,
                o as f32 + (ax - 2) as f32 / zoom,
                p as f32 + (ay - 2) as f32 / zoom,
                26.0 / zoom,
                26.0 / zoom,
                rgba(grey, grey, grey, 255),
            );
            if let Some(Some(icon)) = icons.get(i) {
                // A locked achievement's logo is always dark.
                let item_shade = if is_unlockable { shade } else { 0.1 };
                self.draw_item_model_tinted(
                    batch,
                    icon,
                    o as f32 + (ax + 3) as f32 / zoom,
                    p as f32 + (ay + 3) as f32 / zoom,
                    item_shade,
                    1.0 / zoom,
                );
            }
            if map_x >= ax as f32
                && map_x <= (ax + 22) as f32
                && map_y >= ay as f32
                && map_y <= (ay + 22) as f32
            {
                hovered = Some(i);
            }
        }
        // The frame overlay (its inside is transparent) covers the map's
        // overflow, then the Done button.
        batch.sprite(
            atlas,
            "gui/achievement/achievement_background",
            m as f32,
            n as f32,
        );
        let done = Text::translate("gui.done", &[]);
        let button = (
            (w / 2.0).floor() + 24.0,
            (h / 2.0).floor() + 74.0,
            80.0,
            20.0,
        );
        ui::button(
            batch,
            atlas,
            button.0,
            button.1,
            button.2,
            button.3,
            &done,
            true,
            ui::hovered(self.cursor, button.0, button.1, button.2, button.3),
        );
        if let Some(index) = hovered {
            self.draw_achievement_tooltip(batch, index);
        }
        // `method_6762` draws the title after the tooltip.
        batch.text(
            atlas,
            &lang::get("gui.achievements"),
            (m + 15) as f32,
            (n + 5) as f32,
            rgba(64, 64, 64, 255),
            false,
        );
    }
    /// The hovered achievement's tooltip (`method_6761`'s tail).
    fn draw_achievement_tooltip(&self, batch: &mut HudBatch, index: usize) {
        let achievement = &ACHIEVEMENTS[index];
        let stats = &self.stats;
        let is_taken = taken(stats, index);
        let is_unlockable = unlockable(stats, index);
        let distance = distance(stats, index);
        let atlas = &self.gui.atlas;
        let (mouse_x, mouse_y) = self.cursor;
        let x = mouse_x + 12.0;
        let y = mouse_y - 4.0;
        let mut title = achievement.title().plain();
        let grey = rgba(160, 160, 160, 255);
        if is_unlockable {
            let inventory_key = self.settings.keys.get("key.inventory");
            let description = achievement.description(inventory_key);
            let width = atlas.font.width(&title).max(120.0);
            let lines = atlas
                .font
                .split_formatted(&Text::literal(&description), width);
            let height = 9.0 * lines.len() as f32;
            let extra = if is_taken { 12.0 } else { 0.0 };
            batch.fill(
                atlas,
                x - 3.0,
                y - 3.0,
                width + 6.0,
                height + 18.0 + extra,
                rgba(0, 0, 0, 192),
            );
            for (i, line) in lines.iter().enumerate() {
                batch.text_styled(atlas, line, x, y + 12.0 + 9.0 * i as f32, grey, false);
            }
            if is_taken {
                batch.text(
                    atlas,
                    &lang::get("achievement.taken"),
                    x,
                    y + height + extra + 4.0,
                    rgba(144, 144, 255, 255),
                    true,
                );
            }
        } else if distance <= 3 {
            if distance == 3 {
                title = lang::get("achievement.unknown");
            }
            let width = atlas.font.width(&title).max(120.0);
            let requires = Text::translate(
                "achievement.requires",
                &[achievement
                    .parent
                    .map(|p| ACHIEVEMENTS[p].title())
                    .unwrap_or_else(Text::empty)],
            )
            .plain();
            let lines = atlas.font.split_formatted(&Text::literal(&requires), width);
            let height = 9.0 * lines.len() as f32;
            batch.fill(
                atlas,
                x - 3.0,
                y - 3.0,
                width + 6.0,
                height + 18.0,
                rgba(0, 0, 0, 192),
            );
            for (i, line) in lines.iter().enumerate() {
                batch.text_styled(
                    atlas,
                    line,
                    x,
                    y + 12.0 + 9.0 * i as f32,
                    rgba(112, 80, 80, 255),
                    false,
                );
            }
        } else {
            return;
        }
        let color = if is_unlockable {
            if achievement.challenge {
                rgba(255, 255, 128, 255)
            } else {
                u32::MAX
            }
        } else if achievement.challenge {
            rgba(128, 128, 64, 255)
        } else {
            rgba(128, 128, 128, 255)
        };
        batch.text(atlas, &title, x, y, color, true);
    }
    /// The achievement popup, drawn every frame over the screen.
    pub(super) fn draw_achievement_notification(&mut self, batch: &mut HudBatch) {
        if self.player.is_none() {
            return;
        }
        let Some(shown) = self.ach_notify.shown.as_ref() else {
            return;
        };
        let d = shown.at.elapsed().as_secs_f64() / 3.0;
        let Some(y) = notification_offset(d, shown.permanent) else {
            self.ach_notify.shown = None;
            return;
        };
        let (index, title, name, permanent) = (
            shown.index,
            shown.title.clone(),
            shown.name.clone(),
            shown.permanent,
        );
        let atlas = &self.gui.atlas;
        let x = batch.width - 160.0;
        let y = y as f32;
        batch.sprite(atlas, "gui/sprites/toast/advancement", x, y);
        if permanent {
            let lines = atlas.font.split_formatted(&Text::literal(&name), 120.0);
            for (i, line) in lines.iter().enumerate() {
                batch.text_styled(
                    atlas,
                    line,
                    x + 30.0,
                    y + 7.0 + 9.0 * i as f32,
                    u32::MAX,
                    true,
                );
            }
        } else {
            batch.text(
                atlas,
                &title,
                x + 30.0,
                y + 7.0,
                rgba(255, 255, 0, 255),
                true,
            );
            batch.text(atlas, &name, x + 30.0, y + 18.0, u32::MAX, true);
        }
        if let Some(icon) = ACHIEVEMENTS[index].icon() {
            self.draw_item(batch, &icon, x + 8.0, y + 8.0);
        }
    }
    /// A left button press or release on the achievements screen.
    pub(super) fn achievements_mouse(&mut self, pressed: bool, size: [u32; 2]) {
        if !pressed {
            self.ach_ui.drag = None;
            self.ach_ui.panning = false;
            return;
        }
        if self.ach_ui.downloading {
            return;
        }
        let b = HudBatch::new(size);
        let (w, h) = (b.width, b.height);
        let (x, y) = ((w / 2.0).floor() + 24.0, (h / 2.0).floor() + 74.0);
        if ui::hovered(self.cursor, x, y, 80.0, 20.0) {
            self.audio.play_button_click();
            self.close_achievements();
            return;
        }
        let (m, n) = window(w, h);
        let (cx, cy) = self.cursor;
        if cx >= (m + 8) as f32
            && cx < (m + 232) as f32
            && cy >= (n + 17) as f32
            && cy < (n + 172) as f32
        {
            // `field_1322 = 1`: the next movement pans from this point.
            self.ach_ui.drag = Some((cx, cy));
            self.ach_ui.panning = true;
        }
    }
    /// The pointer moved while the left button is held (`field_1322` in
    /// vanilla's `render`: only a position inside the viewport pans).
    pub(super) fn achievements_drag(&mut self) {
        let Some((last_x, last_y)) = self.ach_ui.drag else {
            return;
        };
        if !self.ach_ui.panning {
            return;
        }
        let (w, h) = self.gui_size();
        let (m, n) = window(w, h);
        let (cx, cy) = self.cursor;
        // Outside the viewport vanilla leaves `field_1310`/`field_1311` at the
        // last inside position, so re-entering pans from there.
        if !(cx >= (m + 8) as f32
            && cx < (m + 232) as f32
            && cy >= (n + 17) as f32
            && cy < (n + 172) as f32)
        {
            return;
        }
        let (dx, dy) = (cx - last_x, cy - last_y);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        let zoom = f64::from(self.ach_ui.zoom);
        self.ach_ui.pan.0 -= f64::from(dx) * zoom;
        self.ach_ui.pan.1 -= f64::from(dy) * zoom;
        self.ach_ui.target = self.ach_ui.pan;
        self.ach_ui.last = self.ach_ui.pan;
        self.ach_ui.drag = Some((cx, cy));
    }
    /// The wheel over the achievements screen: ±0.25 zoom, recentred.
    pub(super) fn achievements_scroll(&mut self, _x: f32, y: f32) {
        if y == 0.0 {
            return;
        }
        let old = self.ach_ui.zoom;
        let new = (old + if y > 0.0 { -0.25 } else { 0.25 }).clamp(1.0, 2.0);
        if new == old {
            return;
        }
        self.ach_ui.zoom = new;
        let change = f64::from(new - old);
        self.ach_ui.pan.0 -= change * 128.0;
        self.ach_ui.pan.1 -= change * 101.0;
        self.ach_ui.target = self.ach_ui.pan;
        self.ach_ui.last = self.ach_ui.pan;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The static block's 34 entries: parents exist, the tree spans the
    /// columns and rows its pan bounds name, and 10 are challenges.
    #[test]
    fn one_eight_achievement_tree_matches_vanilla() {
        assert_eq!(ACHIEVEMENTS.len(), 34);
        assert_eq!(ACHIEVEMENTS[0].key, "openInventory");
        assert_eq!(ACHIEVEMENTS[33].key, "overpowered");
        let mut min_column = i32::MAX;
        let mut max_column = i32::MIN;
        let mut min_row = i32::MAX;
        let mut max_row = i32::MIN;
        let mut challenges = 0;
        for (i, achievement) in ACHIEVEMENTS.iter().enumerate() {
            if let Some(parent) = achievement.parent {
                assert!(parent < i, "{} has a later parent", achievement.key);
            }
            min_column = min_column.min(achievement.column);
            max_column = max_column.max(achievement.column);
            min_row = min_row.min(achievement.row);
            max_row = max_row.max(achievement.row);
            challenges += i32::from(achievement.challenge);
            assert!(
                achievement.icon().is_some(),
                "{} has no logo",
                achievement.key
            );
        }
        assert_eq!((min_column, max_column), (-4, 9));
        assert_eq!((min_row, max_row), (-5, 13));
        assert_eq!(challenges, 10);
        // The bounds the screen clamps to.
        assert_eq!(
            PAN_MIN,
            (
                f64::from(min_column * 24 - 112),
                f64::from(min_row * 24 - 112)
            )
        );
        assert_eq!(
            PAN_MAX,
            (
                f64::from(max_column * 24 - 77 - 1),
                f64::from(max_row * 24 - 77 - 1)
            )
        );
    }

    /// `StatHandler`: taken is 0, an unlockable child is 1.0, and a locked
    /// one counts its ancestors until the nearest taken one.
    #[test]
    fn one_eight_achievement_progress_follows_the_tree() {
        let mut stats = super::super::stats::Stats::default();
        stats.values.insert(
            "minecraft:achievement".into(),
            [("openInventory".to_owned(), 1)].into_iter().collect(),
        );
        assert!(taken(&stats, 0));
        assert_eq!(distance(&stats, 0), 0);
        // mineWood's parent is taken.
        assert!(unlockable(&stats, 1));
        assert!(!taken(&stats, 1));
        // buildWorkBench is two steps from the taken root.
        assert!(!unlockable(&stats, 2));
        assert_eq!(distance(&stats, 2), 1);
        // buildPickaxe: two untaken ancestors (buildWorkBench, mineWood).
        assert_eq!(distance(&stats, 3), 2);
    }

    /// A server's `achievement.openInventory` stat and the current key round
    /// trip; unknown names are ignored like `Stats.getAStat`.
    #[test]
    fn one_eight_achievement_stats_round_trip() {
        assert_eq!(
            super::super::stats::legacy_stat_name("minecraft:achievement", "openInventory")
                .as_deref(),
            Some("achievement.openInventory")
        );
        assert_eq!(
            super::super::stats::stat_1_8_9("achievement.openInventory"),
            Some(("minecraft:achievement", "openInventory".to_owned()))
        );
        assert_eq!(
            super::super::stats::legacy_stat_name("minecraft:achievement", "notAnAchievement"),
            None
        );
        assert_eq!(
            super::super::stats::stat_1_8_9("achievement.notAnAchievement"),
            None
        );
    }

    /// The popup slides in, holds and slides out over its three seconds; the
    /// permanent hint waits 2.5 s and then stays.
    #[test]
    fn one_eight_achievement_notification_slides() {
        assert_eq!(notification_offset(0.0, false), Some(-36.0));
        assert_eq!(notification_offset(0.25, false), Some(0.0));
        assert_eq!(notification_offset(0.5, false), Some(0.0));
        assert_eq!(notification_offset(0.75, false), Some(0.0));
        assert_eq!(notification_offset(1.0, false), Some(-36.0));
        assert_eq!(notification_offset(1.001, false), None);
        assert_eq!(notification_offset(-0.5, false), None);
        // The raw hint's clock starts 2.5 s in: still off-screen, then held.
        assert!(notification_offset(-0.833, true).unwrap() < -36.0);
        assert_eq!(notification_offset(0.5, true), Some(0.0));
        assert_eq!(notification_offset(2.0, true), Some(0.0));
    }

    /// `method_6761`'s block choice.
    #[test]
    fn one_eight_achievement_map_layers() {
        assert_eq!(map_block(38, 0, false), "block/bedrock");
        assert_eq!(map_block(0, 35, false), "block/bedrock");
        assert_eq!(map_block(22, 3, true), "block/diamond_ore");
        assert_eq!(map_block(22, 3, false), "block/redstone_ore");
        assert_eq!(map_block(10, 3, false), "block/iron_ore");
        assert_eq!(map_block(8, 3, false), "block/coal_ore");
        assert_eq!(map_block(5, 3, false), "block/stone");
        assert_eq!(map_block(1, 3, false), "block/dirt");
        assert_eq!(map_block(0, 3, false), "block/sand");
    }

    /// Java `String.hashCode` of the dashed session UUID.
    #[test]
    fn one_eight_achievement_map_seed_is_the_session_hash() {
        // 069a79f4-44e9-4726-a5be-fca90e38aaf5 is Notch's UUID; the expected
        // value is Java's String.hashCode of that text.
        let uuid = [
            0x069a_79f4u32 as i32,
            0x44e9_4726u32 as i32,
            0xa5be_fca9u32 as i32,
            0x0e38_aaf5u32 as i32,
        ];
        assert_eq!(session_hash(uuid), -687_387_784);
    }

    /// `SlotCrafting.checkTakeAchievements`' grants, in its order: a wooden
    /// pickaxe is `buildPickaxe` alone, another material the better one too.
    #[test]
    fn one_eight_crafting_takes_grant_the_slot_achievements() {
        assert_eq!(
            crafted_achievements("minecraft:wooden_pickaxe"),
            vec!["buildPickaxe"]
        );
        assert_eq!(
            crafted_achievements("minecraft:stone_pickaxe"),
            vec!["buildPickaxe", "buildBetterPickaxe"]
        );
        assert_eq!(
            crafted_achievements("minecraft:crafting_table"),
            vec!["buildWorkBench"]
        );
        assert_eq!(
            crafted_achievements("minecraft:furnace"),
            vec!["buildFurnace"]
        );
        assert_eq!(
            crafted_achievements("minecraft:golden_hoe"),
            vec!["buildHoe"]
        );
        assert_eq!(crafted_achievements("minecraft:bread"), vec!["makeBread"]);
        assert_eq!(crafted_achievements("minecraft:cake"), vec!["bakeCake"]);
        assert_eq!(
            crafted_achievements("minecraft:diamond_sword"),
            vec!["buildSword"]
        );
        assert_eq!(
            crafted_achievements("minecraft:enchanting_table"),
            vec!["enchantments"]
        );
        assert_eq!(
            crafted_achievements("minecraft:bookshelf"),
            vec!["bookcase"]
        );
        assert_eq!(
            crafted_achievements("minecraft:enchanted_golden_apple"),
            vec!["overpowered"]
        );
        // A plain golden apple is not the notch one.
        assert!(crafted_achievements("minecraft:golden_apple").is_empty());
        assert!(crafted_achievements("minecraft:diamond").is_empty());
    }

    /// `FurnaceResultSlot.checkTakeAchievements`: an iron ingot or either
    /// cooked fish; a raw fish and everything else grant nothing.
    #[test]
    fn one_eight_furnace_takes_grant_acquire_iron_and_cook_fish() {
        assert_eq!(
            furnace_achievements("minecraft:iron_ingot"),
            vec!["acquireIron"]
        );
        assert_eq!(
            furnace_achievements("minecraft:cooked_cod"),
            vec!["cookFish"]
        );
        assert_eq!(
            furnace_achievements("minecraft:cooked_salmon"),
            vec!["cookFish"]
        );
        assert!(furnace_achievements("minecraft:cod").is_empty());
        assert!(furnace_achievements("minecraft:gold_ingot").is_empty());
    }

    /// `EntityItem.playerTouch`: one achievement per picked stack.
    #[test]
    fn one_eight_pickups_grant_their_items_achievement() {
        assert_eq!(pickup_achievements("minecraft:oak_log"), vec!["mineWood"]);
        assert_eq!(
            pickup_achievements("minecraft:dark_oak_log"),
            vec!["mineWood"]
        );
        assert_eq!(pickup_achievements("minecraft:leather"), vec!["killCow"]);
        assert_eq!(pickup_achievements("minecraft:diamond"), vec!["diamonds"]);
        assert_eq!(pickup_achievements("minecraft:blaze_rod"), vec!["blazeRod"]);
        assert!(pickup_achievements("minecraft:dirt").is_empty());
        // Planks are not logs, though both are wood.
        assert!(pickup_achievements("minecraft:oak_planks").is_empty());
    }

    /// `EntityPlayerMP.changeDimension`: entering the End, leaving it and
    /// every other portal trip.
    #[test]
    fn one_eight_dimension_travel_grants_its_achievement() {
        use super::super::Dim;
        assert_eq!(dimension_achievement(Dim::Overworld, Dim::End), "theEnd");
        assert_eq!(dimension_achievement(Dim::End, Dim::Overworld), "theEnd2");
        assert_eq!(dimension_achievement(Dim::Overworld, Dim::Nether), "portal");
        assert_eq!(dimension_achievement(Dim::Nether, Dim::Overworld), "portal");
        assert_eq!(dimension_achievement(Dim::Nether, Dim::End), "portal");
        assert_eq!(dimension_achievement(Dim::End, Dim::Nether), "portal");
    }

    /// The payload's wire form: kind, count, then each key length, key and
    /// counter.
    #[test]
    fn one_eight_achievement_payload_layout() {
        let payload = achievement_payload(1, &[("ghast", 3), ("overkill", 1)]);
        assert_eq!(payload.channel, "horizon:achievements");
        let mut expected = vec![1u8, 2, 5];
        expected.extend_from_slice(b"ghast");
        expected.push(3);
        expected.push(8);
        expected.extend_from_slice(b"overkill");
        expected.push(1);
        assert_eq!(payload.data, expected);
    }
}
