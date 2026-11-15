//! CreateWorldScreen: the Game, World and More tabs under a MenuTabBar, with
//! Create New World and Cancel in a 33-pixel footer.
use super::{Button, Title};
use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    text::Text,
    ui,
};

/// MenuTabBar.HEIGHT: the header above the tab area.
const TAB_BAR: f32 = 24.0;
/// HeaderAndFooterLayout's default footer height.
const FOOTER: f32 = 33.0;
const TABS: [&str; 3] = [
    "createWorld.tab.game.title",
    "createWorld.tab.world.title",
    "createWorld.tab.more.title",
];
/// Difficulty ids, in the cycle button's order.
pub const DIFFICULTIES: [&str; 4] = ["peaceful", "easy", "normal", "hard"];

/// WorldCreationUiState beyond the name and seed.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldOptions {
    /// The open tab: Game, World or More.
    pub tab: usize,
    /// Survival locked to Hard, one life (`SelectedGameMode.HARDCORE`).
    pub hardcore: bool,
    /// Index in `DIFFICULTIES`.
    pub difficulty: usize,
    /// Allow Cheats as toggled; unset follows the game mode.
    pub commands: Option<bool>,
    /// 1.8's CreateWorldScreen: the More World Options page is open instead
    /// of the name/game mode page.
    pub more: bool,
    /// The world preset the world is created with.
    pub world_type: crate::save::WorldPreset,
    /// CreateFlatWorldScreen's layers and biome, kept with the world type.
    pub flat_settings: Option<super::flat::FlatSettings>,
    pub structures: bool,
    pub bonus_chest: bool,
    /// `getGameRules`: the Game Rules screen's values, saved with the world.
    pub rules: crate::gamerules::GameRules,
    /// The Data Packs screen's selection, lowest priority first.
    pub data_packs: Vec<String>,
    /// The folder of packs for the new world (vanilla's `mcworld-`
    /// temporary folder), made when Data Packs first opens.
    pub pack_dir: Option<std::path::PathBuf>,
}
impl Default for WorldOptions {
    fn default() -> Self {
        Self {
            tab: 0,
            hardcore: false,
            difficulty: 2,
            commands: None,
            more: false,
            world_type: crate::save::WorldPreset::Normal,
            flat_settings: None,
            structures: true,
            bonus_chest: false,
            rules: crate::gamerules::GameRules::default(),
            data_packs: vec!["vanilla".into()],
            pack_dir: None,
        }
    }
}
impl WorldOptions {
    /// Re-Create: the saved world's choices.
    pub(super) fn of_world(
        level: &crate::save::Level,
        commands: bool,
        dir: &std::path::Path,
    ) -> Self {
        let generation = crate::save::WorldGen::read(dir).unwrap_or_default();
        let (pack_dir, data_packs) = crate::datapack::for_recreate(dir);
        Self {
            hardcore: level.hardcore,
            difficulty: DIFFICULTIES
                .iter()
                .position(|d| *d == level.difficulty)
                .unwrap_or(2),
            commands: Some(commands),
            world_type: generation.preset,
            flat_settings: (generation.preset == crate::save::WorldPreset::Flat)
                .then(|| super::flat::FlatSettings::from_world(dir))
                .flatten(),
            structures: generation.structures,
            bonus_chest: generation.bonus_chest,
            rules: crate::gamerules::GameRules::load(dir),
            data_packs,
            pack_dir,
            ..Self::default()
        }
    }
    /// `getDifficulty`: Hardcore is always Hard.
    pub fn difficulty(&self) -> usize {
        if self.hardcore { 3 } else { self.difficulty }
    }
    /// `isAllowCommands`: never in Hardcore, else on for Creative until toggled.
    pub fn commands(&self, creative: bool) -> bool {
        !self.hardcore && self.commands.unwrap_or(creative)
    }
    /// `isBonusChest`: never in Hardcore.
    pub fn bonus_chest(&self) -> bool {
        !self.hardcore && self.bonus_chest
    }
    pub fn world_gen(&self) -> crate::save::WorldGen {
        crate::save::WorldGen {
            preset: self.world_type,
            structures: self.structures,
            bonus_chest: self.bonus_chest(),
        }
    }
    /// The custom layers, as `write_world_gen_settings_with` takes them.
    pub fn flat_settings(&self) -> Option<horizon_world::nbt::Tag> {
        self.flat_settings
            .as_ref()
            .map(super::flat::FlatSettings::to_tag)
    }
    /// A press on one of the open tab's cycle buttons or switches.
    pub(super) fn press(&mut self, id: u8, creative: &mut bool) {
        self.cycle(id, creative, 1);
    }
    /// A step on one of the open tab's cycle buttons or switches (mouse wheel or keyboard).
    pub(super) fn cycle(&mut self, id: u8, creative: &mut bool, step: i32) {
        match id {
            // Survival, Hardcore, Creative.
            20 => {
                let modes = [(false, false), (false, true), (true, false)];
                let cur = (*creative, self.hardcore);
                let idx = modes.iter().position(|&m| m == cur).unwrap_or(0);
                let next_idx = ((idx as i32 + step).rem_euclid(modes.len() as i32)) as usize;
                (*creative, self.hardcore) = modes[next_idx];
            }
            23 => {
                let len = DIFFICULTIES.len() as i32;
                self.difficulty = ((self.difficulty as i32 + step).rem_euclid(len)) as usize;
            }
            24 => self.commands = Some(!self.commands(*creative)),
            25 => {
                if horizon_core::target::is_1_8_9() {
                    let index = LEGACY_PRESETS
                        .iter()
                        .position(|p| *p == self.world_type)
                        .unwrap_or(0);
                    let len = LEGACY_PRESETS.len() as i32;
                    self.world_type =
                        LEGACY_PRESETS[((index as i32 + step).rem_euclid(len)) as usize];
                } else {
                    self.world_type = if step >= 0 {
                        self.world_type.next()
                    } else {
                        self.world_type.prev()
                    };
                }
                if self.world_type == crate::save::WorldPreset::Flat {
                    // setWorldType starts the generator from the flat preset.
                    self.flat_settings = Some(super::flat::FlatSettings::default());
                }
            }
            // 1.8's More World Options / Done.
            32 => self.more = !self.more,
            27 => self.structures = !self.structures,
            28 => self.bonus_chest = !self.bonus_chest,
            _ => {}
        }
    }
}

/// `Mth.roundToward`: up to a multiple of `step`.
fn round_toward(value: f32, step: f32) -> f32 {
    (value / step).ceil() * step
}
/// MenuTabBar.arrangeElements: the first tab's x and every tab's width.
fn tabs(w: f32) -> (f32, f32) {
    let width = w.min(400.0) - 28.0;
    (
        round_toward(((w - width) / 2.0).trunc(), 2.0),
        round_toward((width / TABS.len() as f32).trunc(), 2.0),
    )
}
/// The tab under the pointer.
pub(super) fn tab_at(w: f32, (x, y): (f32, f32)) -> Option<usize> {
    let (x0, width) = tabs(w);
    ((0.0..TAB_BAR).contains(&y) && x >= x0)
        .then(|| ((x - x0) / width) as usize)
        .filter(|i| *i < TABS.len())
}
/// GridLayoutTab.doLayout: the open tab's grid, aligned at (0.5, 1/6) in the
/// area between the tab bar and the footer.
fn grid(w: f32, h: f32, tab: usize) -> (f32, f32) {
    // Game: the labelled name box (9 + 4 + 20) and three buttons, 8 apart.
    // World: a button row, the labelled seed box and the SwitchGrid
    // (4 + 20 + 4 + 20 under its empty spacer row). More: three buttons.
    let (gw, gh) = match tab {
        0 => (210.0, 117.0),
        1 => (310.0, 117.0),
        _ => (210.0, 76.0),
    };
    let area = h - TAB_BAR - FOOTER;
    (
        ((w - gw) * 0.5).trunc(),
        TAB_BAR + ((area - gh) * 0.166_666_67).trunc(),
    )
}
/// The text box of the screen: 0 the name (Game tab or 1.8's first page),
/// 1 the seed (World tab or 1.8's More World Options page).
pub(super) fn field_rect(w: f32, h: f32, o: &WorldOptions) -> Option<(u8, [f32; 4])> {
    if horizon_core::target::is_1_8_9() {
        let cx = (w / 2.0).floor();
        return Some(if o.more {
            (1, [cx - 100.0, 60.0, 200.0, 20.0])
        } else {
            (0, [cx - 100.0, 60.0, 200.0, 20.0])
        });
    }
    let (gx, gy) = grid(w, h, o.tab);
    match o.tab {
        0 => Some((0, [gx + 1.0, gy + 13.0, 208.0, 20.0])),
        1 => Some((1, [gx, gy + 41.0, 308.0, 20.0])),
        _ => None,
    }
}
/// The text box under the pointer: 0 the name, 1 the seed.
pub(super) fn field_at(w: f32, h: f32, o: &WorldOptions, (x, y): (f32, f32)) -> Option<u8> {
    let (id, [fx, fy, fw, fh]) = field_rect(w, h, o)?;
    ui::hovered((x, y), fx, fy, fw, fh).then_some(id)
}

/// 1.8's world types, in the order its button cycles: Default, Superflat,
/// Large Biomes and Amplified. The Customized and Debug types have no
/// generator in Horizon, so they are left out.
const LEGACY_PRESETS: [crate::save::WorldPreset; 4] = [
    crate::save::WorldPreset::Normal,
    crate::save::WorldPreset::Flat,
    crate::save::WorldPreset::LargeBiomes,
    crate::save::WorldPreset::Amplified,
];
/// `WorldType.getTranslationKey`: `generator.<name>`, the names 1.8's
/// `en_US.lang` uses (`generator.default`, `generator.flat`...).
fn legacy_preset_label(preset: crate::save::WorldPreset) -> &'static str {
    match preset {
        crate::save::WorldPreset::Normal => "generator.default",
        crate::save::WorldPreset::Flat => "generator.flat",
        crate::save::WorldPreset::LargeBiomes => "generator.largeBiomes",
        crate::save::WorldPreset::Amplified => "generator.amplified",
        // Not offered by the 1.8 button.
        crate::save::WorldPreset::SingleBiomeSurface => "generator.default",
        crate::save::WorldPreset::Debug => "generator.debug_all_block_states",
    }
}
fn legacy_game_mode(creative: bool, hardcore: bool) -> &'static str {
    match (creative, hardcore) {
        (true, _) => "selectWorld.gameMode.creative",
        (false, true) => "selectWorld.gameMode.hardcore",
        (false, false) => "selectWorld.gameMode.survival",
    }
}
/// CreateWorldScreen's two pages, added in the order initGui adds them (the
/// footer last).
fn legacy_buttons(w: f32, h: f32, creative: bool, o: &WorldOptions) -> Vec<Button> {
    let b = |x: f32, y: f32, w: f32, label: String, enabled: bool, id: u8| Button {
        x,
        y,
        w,
        label,
        enabled,
        id,
        icon: None,
        trailing: None,
    };
    let cx = (w / 2.0).floor();
    let mut buttons = Vec::new();
    if o.more {
        // More World Options: regenerate structures, world type and its
        // Customize screen, cheats and the bonus chest, then Done.
        buttons.push(b(
            cx - 155.0,
            100.0,
            150.0,
            format!("{} {}", tr("selectWorld.mapFeatures"), on_off(o.structures)),
            true,
            27,
        ));
        buttons.push(b(
            cx + 5.0,
            100.0,
            150.0,
            format!(
                "{} {}",
                tr("selectWorld.mapType"),
                tr(legacy_preset_label(o.world_type))
            ),
            true,
            25,
        ));
        buttons.push(b(
            cx + 5.0,
            120.0,
            150.0,
            tr("selectWorld.customizeType"),
            o.world_type == crate::save::WorldPreset::Flat,
            26,
        ));
        buttons.push(b(
            cx - 155.0,
            151.0,
            150.0,
            format!(
                "{} {}",
                tr("selectWorld.allowCommands"),
                on_off(o.commands(creative))
            ),
            !o.hardcore,
            24,
        ));
        buttons.push(b(
            cx + 5.0,
            151.0,
            150.0,
            format!(
                "{} {}",
                tr("selectWorld.bonusItems"),
                on_off(o.bonus_chest())
            ),
            !o.hardcore,
            28,
        ));
        buttons.push(b(
            cx - 75.0,
            187.0,
            150.0,
            crate::lang::get_or("gui.done", "Done"),
            true,
            32,
        ));
    } else {
        buttons.push(b(
            cx - 75.0,
            115.0,
            150.0,
            format!(
                "{}: {}",
                tr("selectWorld.gameMode"),
                tr(legacy_game_mode(creative, o.hardcore))
            ),
            true,
            20,
        ));
        buttons.push(b(
            cx - 75.0,
            187.0,
            150.0,
            crate::lang::get_or("selectWorld.moreWorldOptions", "More World Options..."),
            true,
            32,
        ));
    }
    buttons.push(b(
        cx - 155.0,
        h - 28.0,
        150.0,
        crate::lang::get_or("selectWorld.create", "Create New World"),
        true,
        21,
    ));
    buttons.push(b(
        cx + 5.0,
        h - 28.0,
        150.0,
        crate::lang::get_or("gui.cancel", "Cancel"),
        true,
        22,
    ));
    buttons
}

/// 1.8's English text for the keys the classic screen asks for. The loaded
/// language still wins; without one (tests, a resource pack that stops
/// before `lang/`), the screen says the vanilla words instead of its keys.
fn english(key: &str) -> &'static str {
    match key {
        "selectWorld.create" => "Create New World",
        "selectWorld.enterName" => "World Name",
        "selectWorld.enterSeed" => "Seed for the World Generator",
        "selectWorld.seedInfo" => "Leave blank for a random seed",
        "selectWorld.resultFolder" => "Will be saved in:",
        "selectWorld.gameMode" => "Game Mode",
        "selectWorld.gameMode.survival" => "Survival",
        "selectWorld.gameMode.survival.line1" => "Search for resources, crafting, gain",
        "selectWorld.gameMode.survival.line2" => "levels, health and hunger",
        "selectWorld.gameMode.hardcore" => "Hardcore",
        "selectWorld.gameMode.hardcore.line1" => "Same as survival mode, locked at hardest",
        "selectWorld.gameMode.hardcore.line2" => "difficulty, and one life only",
        "selectWorld.gameMode.creative" => "Creative",
        "selectWorld.gameMode.creative.line1" => "Unlimited resources, free flying and",
        "selectWorld.gameMode.creative.line2" => "destroy blocks instantly",
        "selectWorld.moreWorldOptions" => "More World Options...",
        "selectWorld.mapFeatures" => "Generate Structures:",
        "selectWorld.mapFeatures.info" => "Villages, dungeons etc",
        "selectWorld.mapType" => "World Type:",
        "selectWorld.customizeType" => "Customize",
        "selectWorld.allowCommands" => "Allow Cheats:",
        "selectWorld.allowCommands.info" => "Commands like /gamemode, /xp",
        "selectWorld.bonusItems" => "Bonus Chest:",
        "generator.default" => "Default",
        "generator.flat" => "Superflat",
        "generator.largeBiomes" => "Large Biomes",
        "generator.amplified" => "AMPLIFIED",
        "generator.amplified.info" => "Notice: Just for fun, requires beefy computer",
        "gui.done" => "Done",
        "gui.cancel" => "Cancel",
        "options.on" => "ON",
        "options.off" => "OFF",
        _ => "",
    }
}
fn tr(key: &str) -> String {
    let text = crate::lang::get_or(key, english(key));
    if text.is_empty() {
        Text::translate(key, &[]).plain()
    } else {
        text
    }
}
/// `CycleButton`'s "Label: Value" (`options.generic_value`).
fn labelled(key: &str, value: String) -> String {
    Text::translate(
        "options.generic_value",
        &[Text::translate(key, &[]), Text::literal(value)],
    )
    .plain()
}
fn on_off(on: bool) -> String {
    tr(if on { "options.on" } else { "options.off" })
}
fn game_mode(creative: bool, hardcore: bool) -> &'static str {
    match (creative, hardcore) {
        (true, _) => "selectWorld.gameMode.creative",
        (false, true) => "selectWorld.gameMode.hardcore",
        (false, false) => "selectWorld.gameMode.survival",
    }
}

/// The open tab's buttons and the footer's.
pub(super) fn buttons(w: f32, h: f32, creative: bool, o: &WorldOptions) -> Vec<Button> {
    if horizon_core::target::is_1_8_9() {
        return legacy_buttons(w, h, creative, o);
    }
    let b = |x: f32, y: f32, w: f32, label: String, enabled: bool, id: u8| Button {
        x,
        y,
        w,
        label,
        enabled,
        id,
        icon: None,
        trailing: None,
    };
    let (gx, gy) = grid(w, h, o.tab);
    let mut buttons = match o.tab {
        0 => vec![
            b(
                gx,
                gy + 41.0,
                210.0,
                labelled("selectWorld.gameMode", tr(game_mode(creative, o.hardcore))),
                true,
                20,
            ),
            b(
                gx,
                gy + 69.0,
                210.0,
                labelled(
                    "options.difficulty",
                    tr(&format!(
                        "options.difficulty.{}",
                        DIFFICULTIES[o.difficulty()]
                    )),
                ),
                !o.hardcore,
                23,
            ),
            b(
                gx,
                gy + 97.0,
                210.0,
                labelled("selectWorld.allowCommands", on_off(o.commands(creative))),
                !o.hardcore,
                24,
            ),
        ],
        1 => vec![
            b(
                gx,
                gy,
                150.0,
                labelled("selectWorld.mapType", tr(o.world_type.label())),
                true,
                25,
            ),
            // CreateFlatWorldScreen: only a flat generator has a preset editor.
            b(
                gx + 160.0,
                gy,
                150.0,
                tr("selectWorld.customizeType"),
                o.world_type == crate::save::WorldPreset::Flat,
                26,
            ),
            // SwitchGrid: 44-wide value-only buttons at the grid's right.
            b(gx + 266.0, gy + 73.0, 44.0, on_off(o.structures), true, 27),
            b(
                gx + 266.0,
                gy + 97.0,
                44.0,
                on_off(o.bonus_chest()),
                !o.hardcore,
                28,
            ),
        ],
        // The More tab, as vanilla lays it out: Game Rules, Experiments
        // (the resources' feature packs), Data Packs.
        _ => [
            "selectWorld.gameRules",
            "selectWorld.experiments",
            "selectWorld.dataPacks",
        ]
        .iter()
        .enumerate()
        .map(|(i, key)| b(gx, gy + 28.0 * i as f32, 210.0, tr(key), true, 29 + i as u8))
        .collect(),
    };
    // The footer: two 150-wide buttons 8 apart, centred in its 33 pixels.
    let x = ((w - 308.0) * 0.5).trunc();
    buttons.push(b(
        x,
        h - 27.0,
        150.0,
        crate::lang::get_or("selectWorld.create", "Create New World"),
        true,
        21,
    ));
    buttons.push(b(
        x + 158.0,
        h - 27.0,
        150.0,
        crate::lang::get_or("gui.cancel", "Cancel"),
        true,
        22,
    ));
    buttons
}

/// What button `b` of the screen narrates: the cycle buttons their message
/// and the value a click moves to, the SwitchGrid switches their label (and
/// info) around their value, each with its tooltip; in the order the screen
/// adds them (the tab's widgets after the tab bar, the footer last).
pub(super) fn button_narration(
    b: &Button,
    creative: bool,
    o: &WorldOptions,
) -> crate::widgets::Narratable {
    use crate::widgets::{Narratable, join_for_narration as join};
    let rect = [b.x, b.y, b.w, 20.0];
    let cycle = |next: String| Narratable::cycle(b.label.clone(), next, rect, b.enabled);
    // SwitchGrid: a value-only CycleButton named `""`, narrated as
    // joinForNarration(label, its default narration[, info]).
    let switch = |label: &str, info: Option<&str>, value: bool| {
        let own = Text::translate(
            "options.generic_value",
            &[Text::literal(""), Text::literal(on_off(value))],
        )
        .plain();
        let own = Text::translate("gui.narrate.button", &[Text::literal(own)]).plain();
        let title = match info {
            Some(info) => join(&[&tr(label), &own, &tr(info)]),
            None => join(&[&tr(label), &own]),
        };
        let n = Narratable::new(
            crate::widgets::What::NamedCycle {
                title,
                next: on_off(!value),
                active: b.enabled,
            },
            rect,
        );
        match info {
            Some(info) => n.hint(tr(info)),
            None => n,
        }
    };
    let narratable = match b.id {
        // Survival, Hardcore, Creative.
        20 => {
            let next = match (creative, o.hardcore) {
                (false, false) => game_mode(false, true),
                (false, true) => game_mode(true, false),
                _ => game_mode(false, false),
            };
            cycle(tr(next)).hint(tr(&format!("{}.info", game_mode(creative, o.hardcore))))
        }
        23 => cycle(tr(&format!(
            "options.difficulty.{}",
            DIFFICULTIES[(o.difficulty + 1) % DIFFICULTIES.len()]
        )))
        .hint(tr(&format!(
            "options.difficulty.{}.info",
            DIFFICULTIES[o.difficulty()]
        ))),
        24 => cycle(on_off(!o.commands(creative))).hint(tr("selectWorld.allowCommands.info")),
        25 => {
            let next = if horizon_core::target::is_1_8_9() {
                let index = LEGACY_PRESETS
                    .iter()
                    .position(|p| *p == o.world_type)
                    .unwrap_or(0);
                LEGACY_PRESETS[(index + 1) % LEGACY_PRESETS.len()]
            } else {
                o.world_type.next()
            };
            let n = cycle(tr(if horizon_core::target::is_1_8_9() {
                legacy_preset_label(next)
            } else {
                next.label()
            }));
            if o.world_type == crate::save::WorldPreset::Amplified {
                n.hint(tr(if horizon_core::target::is_1_8_9() {
                    "generator.amplified.info"
                } else {
                    "generator.minecraft.amplified.info"
                }))
            } else {
                n
            }
        }
        27 => switch(
            "selectWorld.mapFeatures",
            Some("selectWorld.mapFeatures.info"),
            o.structures,
        ),
        28 => switch("selectWorld.bonusItems", None, o.bonus_chest()),
        _ => Narratable::button(b.label.clone(), rect, b.enabled),
    };
    // The World tab adds its type and Customize row before the seed box,
    // which Horizon draws first; the footer is tab order group 1.
    narratable.order(match b.id {
        25 | 26 => -1,
        21 | 22 => 1,
        _ => 0,
    })
}

impl Title {
    /// Everything but the buttons, which `draw` adds after.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_create(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        cursor: (f32, f32),
        name: &str,
        seed: &str,
        creative: bool,
        focus: u8,
        o: &WorldOptions,
    ) {
        let (w, h) = (batch.width.ceil(), batch.height.ceil());
        if horizon_core::target::is_1_8_9() {
            // CreateWorldScreen: no tabs; the name page and the More World
            // Options page replace each other. `GuiScreen.drawDefaultBackground`
            // (dirt) behind the title at y 20.
            ui::menu_background(batch, atlas, 0.0, 0.0, w, h, false);
            let cx = (w / 2.0).floor();
            let gray = rgba(160, 160, 160, 255);
            let title = Text::translate("selectWorld.create", &[]);
            let tw = atlas.font.width_text(&title).floor();
            batch.text_styled(atlas, &title, cx - (tw / 2.0).floor(), 20.0, u32::MAX, true);
            let label = |batch: &mut HudBatch, text: Text, x: f32, y: f32, color: u32| {
                batch.text_styled(atlas, &text, x, y, color, true);
            };
            if o.more {
                label(
                    batch,
                    Text::translate("selectWorld.enterSeed", &[]),
                    cx - 100.0,
                    47.0,
                    gray,
                );
                self.create_field(
                    atlas,
                    batch,
                    (super::WORLD_SEED, seed, 32),
                    [cx - 100.0, 60.0, 200.0],
                    focus == 1,
                    None,
                );
                label(
                    batch,
                    Text::translate("selectWorld.seedInfo", &[]),
                    cx - 100.0,
                    85.0,
                    gray,
                );
                // The buttons' own help lines, as `drawScreen` prints them.
                label(
                    batch,
                    Text::translate("selectWorld.mapFeatures.info", &[]),
                    cx - 150.0,
                    122.0,
                    gray,
                );
                if !o.hardcore {
                    label(
                        batch,
                        Text::translate("selectWorld.allowCommands.info", &[]),
                        cx - 150.0,
                        172.0,
                        gray,
                    );
                }
                if o.world_type == crate::save::WorldPreset::Amplified {
                    label(
                        batch,
                        Text::translate("generator.amplified.info", &[]),
                        cx + 7.0,
                        122.0,
                        gray,
                    );
                }
            } else {
                label(
                    batch,
                    Text::translate("selectWorld.enterName", &[]),
                    cx - 100.0,
                    47.0,
                    gray,
                );
                self.create_field(
                    atlas,
                    batch,
                    (super::WORLD_NAME, name, 32),
                    [cx - 100.0, 60.0, 200.0],
                    focus == 0,
                    None,
                );
                label(
                    batch,
                    Text::empty()
                        .append(Text::translate("selectWorld.resultFolder", &[]))
                        .append(Text::literal(format!(" {}", self.target_folder(name)))),
                    cx - 100.0,
                    85.0,
                    gray,
                );
                let mode = legacy_game_mode(creative, o.hardcore);
                label(
                    batch,
                    Text::translate(&format!("{mode}.line1"), &[]),
                    cx - 100.0,
                    137.0,
                    gray,
                );
                label(
                    batch,
                    Text::translate(&format!("{mode}.line2"), &[]),
                    cx - 100.0,
                    149.0,
                    gray,
                );
            }
            return;
        }
        // extractMenuBackground: the tab header strip, the menu background under it.
        ui::tiled(
            batch,
            atlas,
            "gui/tab_header_background",
            0.0,
            0.0,
            w,
            TAB_BAR,
            16.0,
            u32::MAX,
        );
        ui::menu_background(batch, atlas, 0.0, TAB_BAR, w, h - TAB_BAR, false);
        // MenuTabBar: header separators either side of the tabs.
        let (x0, tw) = tabs(w);
        let right = x0 + tw * TABS.len() as f32;
        // TabNavigationBar: the tab under the mouse, else the open one, each a
        // GridLayoutTab with no extra narration. The screen adds it first.
        crate::widgets::narratable(|| {
            use crate::widgets::{ListKind, Narratable, Row};
            let tab = |i: usize| {
                let mut row = Row::new(i, tr(TABS[i]));
                row.hint = Some(String::new());
                row
            };
            Narratable::list(
                ListKind::Tabs,
                TABS.len(),
                tab_at(w, cursor).map(tab),
                Some(tab(o.tab)),
                [x0, 0.0, right - x0, TAB_BAR],
            )
            .key(crate::widgets::key_of(&"create_world/tabs"))
            .order(-2)
        });
        for (x, width) in [(0.0, x0), (right, w - right)] {
            ui::tiled_sized(
                batch,
                atlas,
                "gui/header_separator",
                x,
                TAB_BAR - 2.0,
                width,
                2.0,
                [32.0, 2.0],
                u32::MAX,
            );
        }
        for (i, key) in TABS.iter().enumerate() {
            let x = x0 + tw * i as f32;
            let selected = o.tab == i;
            let label = Text::translate(key, &[]);
            crate::widgets::record(&label.plain(), [x, 0.0, tw, TAB_BAR], true);
            let sprite = match (selected, ui::hovered(cursor, x, 0.0, tw, TAB_BAR)) {
                (true, true) => "gui/sprites/widget/tab_selected_highlighted",
                (true, false) => "gui/sprites/widget/tab_selected",
                (false, true) => "gui/sprites/widget/tab_highlighted",
                (false, false) => "gui/sprites/widget/tab",
            };
            ui::nine_slice(batch, atlas, sprite, x, 0.0, tw, TAB_BAR, u32::MAX);
            let lw = atlas.font.width_text(&label).floor();
            if selected {
                ui::menu_background(batch, atlas, x + 2.0, 2.0, tw - 4.0, TAB_BAR - 2.0, false);
                // renderFocusUnderline.
                let uw = lw.min(tw - 4.0);
                batch.fill(
                    atlas,
                    x + ((tw - uw) / 2.0).floor(),
                    TAB_BAR - 2.0,
                    uw,
                    1.0,
                    u32::MAX,
                );
            }
            // The label sits 3 pixels lower on unselected tabs.
            let top = if selected { 0.0 } else { 3.0 };
            batch.text_styled(
                atlas,
                &label,
                x + (tw / 2.0).floor() - (lw / 2.0).floor(),
                ((top + TAB_BAR - 9.0) / 2.0).floor() + 1.0,
                u32::MAX,
                true,
            );
        }
        let (gx, gy) = grid(w, h, o.tab);
        let label = |batch: &mut HudBatch, key: &str, x: f32, y: f32| {
            batch.text_styled(atlas, &Text::translate(key, &[]), x, y, u32::MAX, true);
        };
        match o.tab {
            0 => {
                // CommonLayouts.labeledElement, centred in the 210 column.
                label(batch, "selectWorld.enterName", gx + 1.0, gy);
                self.create_field(
                    atlas,
                    batch,
                    (super::WORLD_NAME, name, 32),
                    [gx + 1.0, gy + 13.0, 208.0],
                    focus == 0,
                    None,
                );
                // Its tooltip: the folder the world will be saved in.
                crate::widgets::last_narratable(|n| {
                    n.hint = Some(
                        Text::translate(
                            "selectWorld.targetFolder",
                            &[Text::literal(self.target_folder(name))],
                        )
                        .plain(),
                    );
                });
            }
            1 => {
                label(batch, "selectWorld.enterSeed", gx, gy + 28.0);
                self.create_field(
                    atlas,
                    batch,
                    (super::WORLD_SEED, seed, 32),
                    [gx, gy + 41.0, 308.0],
                    focus == 1,
                    Some("selectWorld.seedInfo"),
                );
                // SwitchGrid labels, centred on their switch rows.
                label(batch, "selectWorld.mapFeatures", gx, gy + 78.0);
                label(batch, "selectWorld.bonusItems", gx, gy + 102.0);
            }
            _ => {}
        }
        // The footer separator, drawn over the screen.
        ui::tiled_sized(
            batch,
            atlas,
            "gui/footer_separator",
            0.0,
            h - FOOTER - 2.0,
            w,
            2.0,
            [32.0, 2.0],
            u32::MAX,
        );
    }
    /// An EditBox; `hint` shows while it is empty and unfocused.
    fn create_field(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        (id, value, max): (crate::clipboard::FieldId, &str, usize),
        [x, y, width]: [f32; 3],
        focused: bool,
        hint: Option<&str>,
    ) {
        let rect = [x, y, width, 20.0];
        self.caret
            .borrow_mut()
            .draw(batch, atlas, (id, value, max), rect, focused, self.started);
        crate::widgets::name_last_edit_box(|| {
            tr(if id == super::WORLD_NAME {
                "selectWorld.enterName"
            } else {
                "selectWorld.enterSeed"
            })
        });
        let field = ui::EditBox::new(x, y, width, 20.0);
        if let Some(hint) = hint
            && value.is_empty()
            && !focused
        {
            batch.text_styled(
                atlas,
                &Text::translate(hint, &[]).with_color(crate::text::DARK_GRAY),
                field.text_x(),
                field.text_y(),
                u32::MAX,
                true,
            );
        }
    }
    /// The hovered widget's tooltip, wrapped to Tooltip's 170 pixels.
    pub(super) fn create_tooltip(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        cursor: (f32, f32),
        name: &str,
        creative: bool,
        o: &WorldOptions,
    ) {
        let (w, h) = (batch.width.ceil(), batch.height.ceil());
        if horizon_core::target::is_1_8_9() {
            // 1.8 has no hover tooltips on this screen; its help lines are
            // drawn under the buttons instead.
            return;
        }
        let (gx, gy) = grid(w, h, o.tab);
        let over = |x: f32, y: f32, width: f32| ui::hovered(cursor, x, y, width, 20.0);
        let text = match o.tab {
            0 if over(gx + 1.0, gy + 13.0, 208.0) => Some(Text::translate(
                "selectWorld.targetFolder",
                &[Text::literal(self.target_folder(name)).italic()],
            )),
            0 if over(gx, gy + 41.0, 210.0) => Some(Text::translate(
                &format!("{}.info", game_mode(creative, o.hardcore)),
                &[],
            )),
            0 if over(gx, gy + 69.0, 210.0) => Some(Text::translate(
                &format!("options.difficulty.{}.info", DIFFICULTIES[o.difficulty()]),
                &[],
            )),
            0 if over(gx, gy + 97.0, 210.0) => {
                Some(Text::translate("selectWorld.allowCommands.info", &[]))
            }
            // WorldTab: the Amplified preset's help text on the type button.
            1 if o.world_type == crate::save::WorldPreset::Amplified && over(gx, gy, 150.0) => {
                Some(Text::translate("generator.minecraft.amplified.info", &[]))
            }
            1 if over(gx + 266.0, gy + 73.0, 44.0) => {
                Some(Text::translate("selectWorld.mapFeatures.info", &[]))
            }
            _ => None,
        };
        if let Some(text) = text {
            ui::tooltip(batch, atlas, &atlas.font.split(&text, 170.0), cursor);
        }
    }
}
