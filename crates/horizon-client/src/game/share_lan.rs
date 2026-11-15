//! The 1.8.9 pause menu (GameMenuScreen) and its "Open to LAN" screen
//! (OpenToLanScreen). Both replace their 26.3 counterparts on the 1.8 target;
//! their coordinates and labels come from the 1.8.9 bytecode.
use super::{Game, Screen, lan::LanSettings};
use horizon_client::{
    Result,
    hud::HudBatch,
    text::Text,
    ui,
    widgets::{self, ScreenInfo},
};
use winit::keyboard::KeyCode;

/// `OpenToLanScreen.gameMode`: the `LevelInfo.GameMode` names, indexed by the
/// enum's id (`byName` turns the chosen one back into an id).
const LAN_MODES: [&str; 4] = ["survival", "creative", "adventure", "spectator"];

/// `OpenToLanScreen.buttonClicked` id 104: survival -> spectator -> creative
/// -> adventure -> survival (the bytecode's string comparisons).
fn next_mode(mode: i32) -> i32 {
    match mode {
        3 => 1,
        1 => 2,
        2 => 0,
        _ => 3,
    }
}

/// OpenToLanScreen's state.
pub(super) struct ShareToLan {
    /// A `LevelInfo.GameMode` id, starting at survival.
    mode: i32,
    allow_commands: bool,
    focus: Option<u8>,
}

impl ShareToLan {
    /// `new OpenToLanScreen(parent)`: the mode starts at survival and cheats
    /// are off (the Java fields' defaults).
    fn new() -> Self {
        Self {
            mode: 0,
            allow_commands: false,
            focus: None,
        }
    }
}

/// `GameMenuScreen.init`: 200px buttons, two 98px columns 8px apart, with
/// `i = -16` taken off every row. `can_publish` is vanilla's
/// `isSingleplayer() && !getIntegratedServer().getPublic()`.
fn pause_buttons(
    w: f32,
    h: f32,
    remote: bool,
    can_publish: bool,
) -> Vec<(f32, f32, f32, &'static str, u8, bool)> {
    let x = (w / 2.0).floor();
    let y = (h / 4.0).floor();
    vec![
        (x - 100.0, y + 8.0, 200.0, "menu.returnToGame", 0, true),
        (x - 100.0, y + 32.0, 98.0, "gui.achievements", 4, true),
        (x + 2.0, y + 32.0, 98.0, "gui.stats", 3, true),
        (x - 100.0, y + 80.0, 98.0, "menu.options", 1, true),
        (x + 2.0, y + 80.0, 98.0, "menu.shareToLan", 7, can_publish),
        (
            x - 100.0,
            y + 104.0,
            200.0,
            if remote {
                "menu.disconnect"
            } else {
                "menu.returnToMenu"
            },
            2,
            true,
        ),
    ]
}

/// `OpenToLanScreen.init`: the start and cancel buttons at `height - 28`, the
/// game mode and cheat buttons at y 100.
fn share_buttons(w: f32, h: f32) -> [(u8, f32, f32, f32); 4] {
    let x = (w / 2.0).floor();
    [
        (101, x - 155.0, h - 28.0, 150.0),
        (102, x + 5.0, h - 28.0, 150.0),
        (104, x - 155.0, 100.0, 150.0),
        (103, x + 5.0, 100.0, 150.0),
    ]
}

impl ShareToLan {
    /// `updateButtonTexts`: a label followed by its current value.
    fn label(&self, id: u8) -> Text {
        match id {
            104 => Text::translate("selectWorld.gameMode", &[])
                .append(Text::literal(" "))
                .append(Text::translate(
                    &format!("selectWorld.gameMode.{}", LAN_MODES[self.mode as usize]),
                    &[],
                )),
            103 => Text::translate("selectWorld.allowCommands", &[])
                .append(Text::literal(" "))
                .append(Text::translate(
                    if self.allow_commands {
                        "options.on"
                    } else {
                        "options.off"
                    },
                    &[],
                )),
            101 => Text::translate("lanServer.start", &[]),
            102 => Text::translate("gui.cancel", &[]),
            _ => Text::empty(),
        }
    }
}

/// `OpenToLanScreen.buttonClicked`: cycle the mode in survival, creative,
/// adventure, spectator order; toggle cheats; start; or go back.
fn share_action(game: &mut Game<'_>, id: u8) -> Result<()> {
    match id {
        104 => {
            if let Some(ui) = game.share_lan.as_mut() {
                ui.mode = next_mode(ui.mode);
            }
        }
        103 => {
            if let Some(ui) = game.share_lan.as_mut() {
                ui.allow_commands = !ui.allow_commands;
            }
        }
        102 => game.share_lan = None,
        101 => game.share_lan_start()?,
        _ => {}
    }
    Ok(())
}

impl Game<'_> {
    /// The layout size the pause screens paint at.
    fn pause_size(&self) -> (f32, f32) {
        let size = self
            .renderer
            .as_ref()
            .map_or([1, 1], |r| [r.config.width, r.config.height]);
        let b = HudBatch::new(size);
        (b.width, b.height)
    }

    /// `mc.isSingleplayer() && !mc.getIntegratedServer().getPublic()`: only
    /// the window that owns the integrated server can publish it, and only
    /// while it is not published yet.
    fn can_publish_lan(&self) -> bool {
        self.integrated.is_some() && self.lan_port().is_none()
    }

    pub(super) fn pause_click_1_8_9(&mut self) -> Result<()> {
        if self.share_lan.is_some() {
            return self.share_click();
        }
        let (w, h) = self.pause_size();
        let hit = pause_buttons(w, h, self.external_server(), self.can_publish_lan())
            .into_iter()
            .find(|b| b.5 && ui::hovered(self.cursor, b.0, b.1, b.2, 20.0))
            .map(|b| b.4);
        if let Some(id) = hit {
            self.audio.play_button_click();
            if id == 7 {
                self.pause_focus = Some(id);
                self.share_lan = Some(ShareToLan::new());
            } else {
                self.pause_focus = Some(id);
                self.execute_pause_action(id)?;
            }
        }
        Ok(())
    }

    pub(super) fn pause_key_1_8_9(&mut self, key: KeyCode) -> Result<()> {
        if self.share_lan.is_some() {
            return self.share_key(key);
        }
        let (w, h) = self.pause_size();
        let ids: Vec<u8> = pause_buttons(w, h, self.external_server(), self.can_publish_lan())
            .into_iter()
            .filter(|b| b.5)
            .map(|b| b.4)
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        match key {
            KeyCode::Tab => {
                let reverse = self.has_shift_down();
                let pos = self
                    .pause_focus
                    .and_then(|id| ids.iter().position(|&i| i == id));
                self.pause_focus = Some(match (pos, reverse) {
                    (Some(0), true) | (None, true) => ids[ids.len() - 1],
                    (Some(p), true) => ids[p - 1],
                    (Some(p), false) if p + 1 < ids.len() => ids[p + 1],
                    _ => ids[0],
                });
            }
            KeyCode::ArrowDown | KeyCode::ArrowRight => {
                let pos = self
                    .pause_focus
                    .and_then(|id| ids.iter().position(|&i| i == id))
                    .map_or(0, |p| (p + 1) % ids.len());
                self.pause_focus = Some(ids[pos]);
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                let pos = self
                    .pause_focus
                    .and_then(|id| ids.iter().position(|&i| i == id))
                    .map_or(0, |p| (p + ids.len() - 1) % ids.len());
                self.pause_focus = Some(ids[pos]);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(id) = self.pause_focus {
                    self.audio.play_button_click();
                    if id == 7 {
                        self.share_lan = Some(ShareToLan::new());
                    } else {
                        self.execute_pause_action(id)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn share_click(&mut self) -> Result<()> {
        let (w, h) = self.pause_size();
        let hit = share_buttons(w, h)
            .into_iter()
            .find(|b| ui::hovered(self.cursor, b.1, b.2, b.3, 20.0))
            .map(|b| b.0);
        if let Some(id) = hit {
            if let Some(ui) = self.share_lan.as_mut() {
                ui.focus = Some(id);
            }
            self.audio.play_button_click();
            share_action(self, id)?;
        }
        Ok(())
    }

    fn share_key(&mut self, key: KeyCode) -> Result<()> {
        const IDS: [u8; 4] = [104, 103, 101, 102];
        let Some(focus) = self.share_lan.as_ref().map(|ui| ui.focus) else {
            return Ok(());
        };
        let pos = focus.and_then(|id| IDS.iter().position(|&i| i == id));
        match key {
            KeyCode::Tab | KeyCode::ArrowDown | KeyCode::ArrowRight => {
                let next = pos.map_or(0, |p| (p + 1) % IDS.len());
                if let Some(ui) = self.share_lan.as_mut() {
                    ui.focus = Some(IDS[next]);
                }
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                let next = pos.map_or(0, |p| (p + IDS.len() - 1) % IDS.len());
                if let Some(ui) = self.share_lan.as_mut() {
                    ui.focus = Some(IDS[next]);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(id) = focus {
                    self.audio.play_button_click();
                    share_action(self, id)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `OpenToLanScreen.buttonClicked` id 101: the screen closes, the world is
    /// told the chosen defaults, and the chat carries the result.
    fn share_lan_start(&mut self) -> Result<()> {
        let Some(ui) = self.share_lan.take() else {
            return Ok(());
        };
        self.pause_focus = None;
        self.screen = Screen::Playing;
        self.pause_menu = true;
        self.capture(true);
        let mode = ui.mode;
        let allow = ui.allow_commands;
        match self.share_lan_publish(mode, allow) {
            Ok(port) => {
                self.system_message(Text::translate(
                    "commands.publish.started",
                    &[Text::literal(port.to_string())],
                ));
            }
            Err(e) => {
                log::warn!("Horizon: Open to LAN: {e}");
                // OpenToLanScreen.buttonClicked builds a LiteralText here, so
                // 1.8.9 shows the raw key rather than the translated sentence.
                self.system_message(Text::literal("commands.publish.failed"));
            }
        }
        Ok(())
    }

    /// `IntegratedServer.method_3000` (shareToLAN): bind the port, then set
    /// the mode new players join in and whether everyone may use commands.
    /// Publishing first matters: the hub takes its settings from the publish
    /// call, and 1.8's cheats flag is the PlayerManager's, not the world's.
    fn share_lan_publish(&mut self, mode: i32, allow: bool) -> std::result::Result<u16, String> {
        let settings = LanSettings {
            port: 0,
            // PlayerManager.setCheatsAllowed: with cheats on, everyone who
            // joins may use commands, as if opped.
            guest_commands: allow,
            force_game_mode: false,
        };
        if let Some(server) = &self.integrated {
            let out = server
                .call(move |g| {
                    let port = g.publish(settings)?;
                    g.share_lan_settings(mode, allow);
                    Ok(port)
                })
                .ok_or("the integrated server stopped")?;
            return out;
        }
        if self.serving() {
            let port = self.publish(settings)?;
            self.share_lan_settings(mode, allow);
            return Ok(port);
        }
        self.lan_start(settings).map_err(|e| e.to_string())?;
        self.share_lan_settings(mode, allow);
        Ok(self.lan.as_ref().map_or(0, |l| l.settings.port))
    }

    /// `PlayerManager.setGameMode` and `setCheatsAllowed`. `setCheatsAllowed`
    /// is the LAN session's flag: it stacks on the single-player owner's
    /// world cheats (`PlayerManager.isOperator`) and is never saved.
    fn share_lan_settings(&mut self, mode: i32, allow: bool) {
        let world_cheats = self.level.allow_commands == Some(true);
        self.owner_allow_commands(world_cheats || allow);
        if let Some(host) = self.lan.as_mut() {
            host.hub.config.default_game_mode = mode & 3;
            host.hub.config.guest_commands = allow;
            host.hub.resend_commands();
        }
    }

    pub(super) fn draw_pause_menu_1_8_9(&mut self, batch: &mut HudBatch) {
        if let Some(ui) = self.share_lan.take() {
            self.draw_share_lan(batch, &ui);
            self.share_lan = Some(ui);
            return;
        }
        let atlas = &self.gui.atlas;
        let (w, h) = (batch.width, batch.height);
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, true);
        let title = Text::translate("menu.game", &[]);
        widgets::begin_screen(|| ScreenInfo::new("pause", title.plain()));
        let tw = atlas.font.width_text(&title);
        batch.text_styled(
            atlas,
            &title,
            ((w - tw) / 2.0).floor(),
            40.0,
            u32::MAX,
            true,
        );
        for (x, y, bw, key, id, active) in
            pause_buttons(w, h, self.external_server(), self.can_publish_lan())
        {
            let hover = active
                && (ui::hovered(self.cursor, x, y, bw, 20.0) || self.pause_focus == Some(id));
            ui::button(
                batch,
                atlas,
                x,
                y,
                bw,
                20.0,
                &Text::translate(key, &[]),
                active,
                hover,
            );
        }
    }

    /// `OpenToLanScreen.render`: the title at y 50, the subtitle at y 82, then
    /// the four buttons.
    fn draw_share_lan(&self, batch: &mut HudBatch, ui: &ShareToLan) {
        let atlas = &self.gui.atlas;
        let (w, h) = (batch.width, batch.height);
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, true);
        widgets::begin_screen(|| ScreenInfo::new("share_to_lan", "LAN World".to_owned()));
        for (key, y) in [("lanServer.title", 50.0), ("lanServer.otherPlayers", 82.0)] {
            let text = Text::translate(key, &[]);
            let tw = atlas.font.width_text(&text);
            batch.text_styled(atlas, &text, ((w - tw) / 2.0).floor(), y, u32::MAX, true);
        }
        for (id, x, y, bw) in share_buttons(w, h) {
            let hover = ui::hovered(self.cursor, x, y, bw, 20.0) || ui.focus == Some(id);
            ui::button(batch, atlas, x, y, bw, 20.0, &ui.label(id), true, hover);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GameMenuScreen.init` in ints: 320x240 puts the first button at
    /// (60, 68), the two columns at x 60/162 and the last row at y 164.
    #[test]
    fn the_1_8_pause_menu_uses_the_vanilla_grid() {
        let b = pause_buttons(320.0, 240.0, false, true);
        assert_eq!(
            b,
            vec![
                (60.0, 68.0, 200.0, "menu.returnToGame", 0, true),
                (60.0, 92.0, 98.0, "gui.achievements", 4, true),
                (162.0, 92.0, 98.0, "gui.stats", 3, true),
                (60.0, 140.0, 98.0, "menu.options", 1, true),
                (162.0, 140.0, 98.0, "menu.shareToLan", 7, true),
                (60.0, 164.0, 200.0, "menu.returnToMenu", 2, true),
            ]
        );
        // A server's window disconnects instead of saving the world.
        let remote = pause_buttons(320.0, 240.0, true, false);
        assert_eq!(remote[5].3, "menu.disconnect");
        // A guest cannot publish someone else's world.
        assert!(!remote[4].5);
    }

    /// `OpenToLanScreen.init`: two rows, each 150 wide with 10 pixels
    /// between them, the start row on the bottom edge.
    #[test]
    fn the_1_8_lan_screen_uses_the_vanilla_grid() {
        assert_eq!(
            share_buttons(320.0, 240.0),
            [
                (101, 5.0, 212.0, 150.0),
                (102, 165.0, 212.0, 150.0),
                (104, 5.0, 100.0, 150.0),
                (103, 165.0, 100.0, 150.0),
            ]
        );
    }

    #[test]
    fn the_game_mode_cycles_like_vanilla() {
        assert_eq!(
            LAN_MODES,
            ["survival", "creative", "adventure", "spectator"]
        );
        // OpenToLanScreen.buttonClicked compares names: survival -> spectator,
        // spectator -> creative, creative -> adventure, adventure -> survival.
        assert_eq!(next_mode(0), 3);
        assert_eq!(next_mode(3), 1);
        assert_eq!(next_mode(1), 2);
        assert_eq!(next_mode(2), 0);
    }
}
