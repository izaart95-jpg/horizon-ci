//! Key display names (`InputConstants.Key.getDisplayName`), shared by key
//! mappings and `keybind` text components.
use crate::text::Text;
use std::{collections::BTreeMap, sync::RwLock};

/// The keys that write a symbol, with what they write on a US layout.
const SYMBOLS: [(&str, &str); 11] = [
    ("key.keyboard.minus", "-"),
    ("key.keyboard.equal", "="),
    ("key.keyboard.left.bracket", "["),
    ("key.keyboard.right.bracket", "]"),
    ("key.keyboard.backslash", "\\"),
    ("key.keyboard.semicolon", ";"),
    ("key.keyboard.apostrophe", "'"),
    ("key.keyboard.grave.accent", "`"),
    ("key.keyboard.comma", ","),
    ("key.keyboard.period", "."),
    ("key.keyboard.slash", "/"),
];

/// What the player's keyboard layout writes on each key that writes
/// something, by key name, as the window reported it (`learn_layout`).
static LAYOUT: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

/// The keyboard layout writes `text` on the key `key` (`key.keyboard.q`):
/// its name from now on, as `SDL_GetKeyFromScancode` names it with SDL's
/// default keycode options. Only letter and symbol keys take it; a letter
/// key keeps its Latin letter on a non-Latin layout (`latin_letters`), and
/// the number row keeps its digits (`french_numbers`).
pub fn learn_layout(key: &str, text: &str) {
    let Some(rest) = key.strip_prefix("key.keyboard.") else {
        return;
    };
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return;
    };
    let letter_key = rest.len() == 1 && rest.as_bytes()[0].is_ascii_lowercase();
    let latin = c.is_ascii() || ('\u{C0}'..='\u{24F}').contains(&c);
    if c.is_control()
        || c.is_whitespace()
        || !(letter_key || SYMBOLS.iter().any(|(k, _)| *k == key))
        || (letter_key && c.is_alphabetic() && !latin)
    {
        return;
    }
    if let Ok(mut layout) = LAYOUT.write() {
        layout.insert(key.to_owned(), c.to_uppercase().collect());
    }
}

/// InputConstants.Key.getDisplayName: what the layout writes on the key,
/// upper-case, else the US layout's character, else by translation.
pub fn display_name(key: &str) -> Text {
    if let Some(label) = LAYOUT.read().ok().and_then(|l| l.get(key).cloned()) {
        return Text::literal(label);
    }
    if let Some(rest) = key.strip_prefix("key.keyboard.")
        && rest.chars().count() == 1
    {
        let c = rest.to_uppercase();
        return Text::literal(c);
    }
    if let Some((_, s)) = SYMBOLS.iter().find(|(k, _)| *k == key) {
        return Text::literal(*s);
    }
    if let Some(n) = key.strip_prefix("key.mouse.")
        && n.parse::<u32>().is_ok()
        && !crate::lang::has(key)
    {
        return Text::translate("key.mouse", &[Text::literal(n)]);
    }
    Text::translate(key, &[])
}

/// `GameSettings.getKeyDisplayString` (1.8): the name is LWJGL 2's
/// `Keyboard.getKeyName`, not a 1.13 translation key, and a mouse button is
/// `key.mouseButton` with the button number (left is Button 1).
pub fn display_name_1_8_9(key: &str) -> Text {
    if let Some(rest) = key.strip_prefix("key.mouse.") {
        let button = match rest {
            "left" => 1,
            "right" => 2,
            "middle" => 3,
            other => other.parse::<u32>().unwrap_or(0),
        };
        return Text::translate("key.mouseButton", &[Text::literal(button.to_string())]);
    }
    let Some(name) = key.strip_prefix("key.keyboard.") else {
        return Text::translate(key, &[]);
    };
    // Letters and the number row are their own upper-case name.
    if name.chars().count() == 1 {
        return Text::literal(name.to_uppercase());
    }
    let name = match name {
        "unknown" => "NONE",
        "escape" => "ESCAPE",
        "backspace" => "BACK",
        "enter" => "RETURN",
        "tab" => "TAB",
        "space" => "SPACE",
        "minus" => "MINUS",
        "equal" => "EQUALS",
        "left.bracket" => "LBRACKET",
        "right.bracket" => "RBRACKET",
        "backslash" => "BACKSLASH",
        "semicolon" => "SEMICOLON",
        "apostrophe" => "APOSTROPHE",
        "grave.accent" => "GRAVE",
        "comma" => "COMMA",
        "period" => "PERIOD",
        "slash" => "SLASH",
        "caps.lock" => "CAPITAL",
        "left.control" => "LCONTROL",
        "left.shift" => "LSHIFT",
        "left.alt" => "LMENU",
        "left.win" => "LWIN",
        "right.control" => "RCONTROL",
        "right.shift" => "RSHIFT",
        "right.alt" => "RMENU",
        "right.win" => "RWIN",
        "menu" => "APPS",
        "print.screen" => "SYSRQ",
        "scroll.lock" => "SCROLL",
        "pause" => "PAUSE",
        "insert" => "INSERT",
        "home" => "HOME",
        "page.up" => "PRIOR",
        "delete" => "DELETE",
        "end" => "END",
        "page.down" => "NEXT",
        "right" => "RIGHT",
        "left" => "LEFT",
        "down" => "DOWN",
        "up" => "UP",
        "num.lock" => "NUMLOCK",
        "keypad.divide" => "DIVIDE",
        "keypad.multiply" => "MULTIPLY",
        "keypad.subtract" => "SUBTRACT",
        "keypad.add" => "ADD",
        "keypad.enter" => "NUMPADENTER",
        "keypad.period" => "DECIMAL",
        "keypad.1" => "NUMPAD1",
        "keypad.2" => "NUMPAD2",
        "keypad.3" => "NUMPAD3",
        "keypad.4" => "NUMPAD4",
        "keypad.5" => "NUMPAD5",
        "keypad.6" => "NUMPAD6",
        "keypad.7" => "NUMPAD7",
        "keypad.8" => "NUMPAD8",
        "keypad.9" => "NUMPAD9",
        "keypad.0" => "NUMPAD0",
        other if other.starts_with('f') => {
            // F1-F24 keep their own name in LWJGL 2.
            if other[1..].parse::<u8>().is_ok() {
                return Text::literal(other.to_uppercase());
            }
            return Text::literal("NONE");
        }
        _ => "NONE",
    };
    Text::literal(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_show_what_the_layout_writes() {
        // The layout table is shared: these keys are this test's alone.
        assert_eq!(display_name("key.keyboard.q").plain(), "Q");
        // AZERTY: A on the Q key, M on the semicolon key, a dead ^ on the
        // left bracket, a comma on the M key.
        learn_layout("key.keyboard.q", "a");
        learn_layout("key.keyboard.semicolon", "m");
        learn_layout("key.keyboard.left.bracket", "^");
        learn_layout("key.keyboard.m", ",");
        assert_eq!(display_name("key.keyboard.q").plain(), "A");
        assert_eq!(display_name("key.keyboard.semicolon").plain(), "M");
        assert_eq!(display_name("key.keyboard.left.bracket").plain(), "^");
        assert_eq!(display_name("key.keyboard.m").plain(), ",");
        // The number row keeps its digits; a Cyrillic letter, nothing
        // printable and keys that write nothing are left alone.
        learn_layout("key.keyboard.1", "&");
        learn_layout("key.keyboard.f", "а");
        learn_layout("key.keyboard.g", "\u{8}");
        learn_layout("key.keyboard.tab", "\t");
        learn_layout("key.keyboard.h", "ab");
        assert_eq!(display_name("key.keyboard.1").plain(), "1");
        assert_eq!(display_name("key.keyboard.f").plain(), "F");
        assert_eq!(display_name("key.keyboard.g").plain(), "G");
        assert_eq!(display_name("key.keyboard.h").plain(), "H");
        assert_eq!(display_name("key.keyboard.slash").plain(), "/");
    }

    #[test]
    fn one_eight_key_names_are_lwjgl_names() {
        // `GameSettings.getKeyDisplayString` in 1.8: LWJGL 2's key names.
        assert_eq!(display_name_1_8_9("key.keyboard.w").plain(), "W");
        assert_eq!(display_name_1_8_9("key.keyboard.7").plain(), "7");
        assert_eq!(
            display_name_1_8_9("key.keyboard.left.shift").plain(),
            "LSHIFT"
        );
        assert_eq!(
            display_name_1_8_9("key.keyboard.left.control").plain(),
            "LCONTROL"
        );
        assert_eq!(display_name_1_8_9("key.keyboard.enter").plain(), "RETURN");
        assert_eq!(display_name_1_8_9("key.keyboard.equal").plain(), "EQUALS");
        assert_eq!(display_name_1_8_9("key.keyboard.page.up").plain(), "PRIOR");
        assert_eq!(display_name_1_8_9("key.keyboard.f5").plain(), "F5");
        assert_eq!(
            display_name_1_8_9("key.keyboard.keypad.3").plain(),
            "NUMPAD3"
        );
        assert_eq!(display_name_1_8_9("key.keyboard.unknown").plain(), "NONE");
    }
}
