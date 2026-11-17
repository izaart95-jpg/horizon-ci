//! Languages (ClientLanguage / LanguageManager): `lang/<code>.json` from the
//! resource root and enabled packs layered over `en_us`, the language list
//! from `pack.mcmeta` `language` sections, and a process-wide current
//! language used by every screen.
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub region: String,
    pub bidirectional: bool,
}
impl LanguageInfo {
    /// LanguageInfo.toComponent: "Name (Region)".
    pub fn label(&self) -> String {
        format!("{} ({})", self.name, self.region)
    }
}
#[derive(Default)]
pub struct Language {
    pub code: String,
    /// `LanguageInfo.bidirectional`: text without a strong character reads
    /// right to left (`ClientLanguage.defaultRightToLeft`).
    bidirectional: bool,
    map: HashMap<String, String>,
}
pub const DEFAULT: &str = "en_us";

mod legacy;
mod names_1_8_9;

static CURRENT: RwLock<Option<Arc<Language>>> = RwLock::new(None);
/// The current language's `bidirectional`, read for every drawn string.
static BIDIRECTIONAL: AtomicBool = AtomicBool::new(false);

/// A source of resource files: the prepared root plus packs, highest priority last.
pub trait ResourceSource {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// Whether `path` is there, without reading it where the source can tell.
    fn exists(&self, path: &str) -> bool {
        self.read(path).is_some()
    }
    /// The namespaces under `assets/` (resource packs may add their own).
    fn namespaces(&self) -> Vec<String> {
        vec!["minecraft".into()]
    }
}
/// Horizon's own strings, compiled in so they need no prepared resources.
struct Bundled;
impl ResourceSource for Bundled {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        let body = match path {
            "assets/horizon/lang/en_us.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/en_us.json")
            }
            "assets/horizon/lang/zh_cn.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/zh_cn.json")
            }
            "assets/horizon/lang/ru_ru.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/ru_ru.json")
            }
            "assets/horizon/lang/pl_pl.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/pl_pl.json")
            }
            "assets/horizon/lang/uk_ua.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/uk_ua.json")
            }
            "assets/horizon/lang/tr_tr.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/tr_tr.json")
            }
            "assets/horizon/lang/es_es.json"
            | "assets/horizon/lang/es_ar.json"
            | "assets/horizon/lang/es_cl.json"
            | "assets/horizon/lang/es_ec.json"
            | "assets/horizon/lang/es_mx.json"
            | "assets/horizon/lang/es_uy.json"
            | "assets/horizon/lang/es_ve.json" => {
                include_str!("../../horizon-client/assets/horizon/lang/es_es.json")
            }
            _ => return None,
        };
        Some(body.as_bytes().to_vec())
    }
    fn namespaces(&self) -> Vec<String> {
        vec!["horizon".into()]
    }
}
impl ResourceSource for std::path::PathBuf {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.join(path)).ok()
    }
    fn exists(&self, path: &str) -> bool {
        self.join(path).is_file()
    }
    fn namespaces(&self) -> Vec<String> {
        std::fs::read_dir(self.join("assets"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(str::to_owned))
            .collect()
    }
}

/// 1.8's language code for `code`: `Locale` spells it `language_COUNTRY`, the
/// language lowercased and the region uppercased (`en_us` is `en_US`).
pub fn legacy_code(code: &str) -> String {
    match code.split_once('_') {
        Some((language, region)) => format!(
            "{}_{}",
            language.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        ),
        None => code.to_ascii_lowercase(),
    }
}

/// The 1.8 location of a language: `assets/<namespace>/lang/<code>.lang`.
fn legacy_path(namespace: &str, code: &str) -> String {
    format!("assets/{namespace}/lang/{}.lang", legacy_code(code))
}

/// 1.8's `.lang` parser (`Locale.loadLocaleData`): `key=value` lines.
///
/// Vanilla does not trim lines: an empty line, or one whose first character is
/// `#`, is skipped, and so is a line without an `=`; a line is split at its
/// first `=` only, empty keys and values included. The value's `%d`/`%f`
/// conversion specifiers (`%1$d`, `%.2f`) are then rewritten to the `%s`/
/// `%1$s` ones `Locale.formatMessage` hands to `String.format`, the whole
/// value through one `replaceAll("%(\\d+\\$)?[\\d\\.]*[df]", "%$1s")`. Bytes
/// that are not UTF-8 become U+FFFD, as Java's UTF-8 reader does.
pub fn parse_legacy_lang(bytes: &[u8]) -> HashMap<String, String> {
    String::from_utf8_lossy(bytes)
        // BufferedReader.readLine ends a line at `\n`, `\r` or `\r\n`; the
        // empty line between a `\r\n` pair is skipped like any other.
        .split(['\n', '\r'])
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_owned(), legacy_format(value)))
        })
        .collect()
}

/// `Locale.pattern` (`%(\d+\$)?[\d\.]*[df]`) replaced with `%$1s`.
fn legacy_format(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            let c = value[i..].chars().next().unwrap_or('\u{fffd}');
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        let mut j = i + 1;
        let digits = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        let argument = if j > digits && bytes.get(j) == Some(&b'$') {
            j += 1;
            Some(&value[digits..j])
        } else {
            None
        };
        while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
            j += 1;
        }
        if bytes.get(j).is_some_and(|b| *b == b'd' || *b == b'f') {
            out.push('%');
            out.push_str(argument.unwrap_or(""));
            out.push('s');
            i = j + 1;
        } else {
            out.push('%');
            i += 1;
        }
    }
    out
}

/// DeprecatedTranslationsInfo.applyToMap: drop removed keys, then move each
/// renamed key's text to its new key (a missing old key removes the new one).
pub fn apply_deprecated(map: &mut HashMap<String, String>, json: &[u8]) {
    let Ok(info) = serde_json::from_slice::<serde_json::Value>(json) else {
        return;
    };
    for key in info["removed"].as_array().into_iter().flatten() {
        if let Some(k) = key.as_str() {
            map.remove(k);
        }
    }
    for (from, to) in info["renamed"].as_object().into_iter().flatten() {
        let Some(to) = to.as_str() else { continue };
        match map.remove(from) {
            Some(value) => {
                map.insert(to.to_owned(), value);
            }
            None => {
                map.remove(to);
            }
        }
    }
}

impl Language {
    /// ClientLanguage.loadFrom: en_us first, then the selected language, each
    /// from every namespace and every source in priority order (later sources
    /// override).
    pub fn load(sources: &[&dyn ResourceSource], code: &str) -> Self {
        let mut map = HashMap::new();
        let mut codes = vec![DEFAULT];
        if code != DEFAULT {
            codes.push(code);
        }
        // Horizon's own strings first, so a pack or the prepared root can still
        // override them. Vanilla has no key for "Controller vibration".
        let bundled = Bundled;
        let mut all: Vec<&dyn ResourceSource> = vec![&bundled];
        all.extend_from_slice(sources);
        let sources = all.as_slice();
        let namespaces: std::collections::BTreeSet<String> =
            sources.iter().flat_map(|s| s.namespaces()).collect();
        for c in codes {
            for namespace in &namespaces {
                for source in sources {
                    // 26.3's JSON first; 1.8's `key=value` file only when this
                    // source has no JSON for the language.
                    match source.read(&format!("assets/{namespace}/lang/{c}.json")) {
                        Some(bytes) => {
                            if let Ok(entries) =
                                serde_json::from_slice::<HashMap<String, String>>(&bytes)
                            {
                                map.extend(entries);
                            }
                        }
                        None => {
                            if let Some(bytes) = source.read(&legacy_path(namespace, c)) {
                                map.extend(parse_legacy_lang(&bytes));
                            }
                        }
                    }
                }
            }
        }
        // DeprecatedTranslationsInfo.loadFromDefaultResource().applyToMap: the
        // game's own deprecated.json, not a resource pack's.
        if let Some(bytes) = sources
            .iter()
            .find_map(|s| s.read("assets/minecraft/lang/deprecated.json"))
        {
            apply_deprecated(&mut map, &bytes);
        }
        // A 1.8 resource root has no entry for the keys the newer screens use;
        // English text beats drawing the raw key.
        for &(key, value) in legacy::FALLBACKS {
            map.entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
        // 1.8 names the enchantments by legacy keys; a locale keeps its own
        // translation by copying it onto the current description key.
        for &(key, legacy) in legacy::ENCHANTMENT_LEGACY_KEYS {
            if let Some(value) = map.get(legacy).cloned() {
                map.entry(key.to_owned()).or_insert(value);
            }
        }
        for &(key, value) in legacy::ENCHANTMENT_NAMES {
            map.entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
        // The 1.8 resource root keys its items `item.diamond.name`, but the
        // current item data names them `item.minecraft.diamond`; without these
        // the display name falls back to the lowercased id. A locale names the
        // item under the 1.8 key, so its value moves onto the current key
        // before the English table fills what the file cannot answer.
        if map.contains_key("tile.stone.stone.name") {
            for &(key, legacy) in names_1_8_9::LEGACY_KEYS_1_8_9 {
                if let Some(value) = map.get(legacy).cloned() {
                    map.entry(key.to_owned()).or_insert(value);
                }
            }
            // `ItemMonsterPlacer.getItemStackDisplayName` is the item's own
            // "Spawn" plus `entity.<Name>.name`; 1.8 has one item key for all
            // 27 eggs, so compose each with the locale's entity name.
            if let Some(spawn) = map.get("item.monsterPlacer.name").cloned() {
                for &(key, entity) in legacy::EGG_ENTITY_KEYS_1_8_9 {
                    if let Some(name) = map.get(entity).cloned() {
                        let composed = format!("{} {}", spawn.trim(), name.trim());
                        map.entry(key.to_owned()).or_insert(composed);
                    }
                }
            }
            for &(key, value) in names_1_8_9::NAMES_1_8_9 {
                map.entry(key.to_owned())
                    .or_insert_with(|| value.to_owned());
            }
        }
        let bidirectional = available(sources)
            .iter()
            .any(|info| info.code == code && info.bidirectional);
        Self {
            code: code.to_owned(),
            bidirectional,
            map,
        }
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }
    /// The resolved map, for `horizon_protocol::text::plain`: a server's
    /// components carry keys, and the network layer resolves them with this.
    pub fn map(&self) -> &HashMap<String, String> {
        &self.map
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Languages declared by `pack.mcmeta` files (the vanilla asset root first).
pub fn available(sources: &[&dyn ResourceSource]) -> Vec<LanguageInfo> {
    let mut found: BTreeMap<String, LanguageInfo> = BTreeMap::new();
    found.insert(
        DEFAULT.into(),
        LanguageInfo {
            code: DEFAULT.into(),
            name: "English".into(),
            region: "US".into(),
            bidirectional: false,
        },
    );
    for source in sources {
        for path in ["assets/pack.mcmeta", "pack.mcmeta"] {
            let Some(bytes) = source.read(path) else {
                continue;
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                continue;
            };
            for (code, info) in value["language"].as_object().into_iter().flatten() {
                // Only languages whose file exists somewhere are selectable.
                // A 1.8 pack names them `en_US` and ships `.lang` files.
                let file = format!("assets/minecraft/lang/{code}.json");
                let legacy = legacy_path("minecraft", code);
                if code.eq_ignore_ascii_case(DEFAULT) {
                    continue;
                }
                if !sources.iter().any(|s| s.exists(&file) || s.exists(&legacy)) {
                    continue;
                }
                found.insert(
                    code.clone(),
                    LanguageInfo {
                        code: code.clone(),
                        name: info["name"].as_str().unwrap_or(code).to_owned(),
                        region: info["region"].as_str().unwrap_or("").to_owned(),
                        bidirectional: info["bidirectional"].as_bool().unwrap_or(false),
                    },
                );
            }
        }
    }
    found.into_values().collect()
}

pub fn set(language: Language) {
    BIDIRECTIONAL.store(language.bidirectional, Ordering::Relaxed);
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(language));
}
pub fn current() -> Option<Arc<Language>> {
    CURRENT.read().unwrap_or_else(|e| e.into_inner()).clone()
}
/// `ClientLanguage.defaultRightToLeft` of the current language.
pub fn bidirectional() -> bool {
    BIDIRECTIONAL.load(Ordering::Relaxed)
}
pub fn code() -> String {
    current().map_or_else(|| DEFAULT.to_owned(), |l| l.code.clone())
}
/// Language.getOrDefault: the translation, or the key itself.
pub fn get(key: &str) -> String {
    current()
        .and_then(|l| l.get(key).map(str::to_owned))
        .unwrap_or_else(|| key.to_owned())
}
/// The translation, or `fallback` when the key is missing.
pub fn get_or(key: &str, fallback: &str) -> String {
    current()
        .and_then(|l| l.get(key).map(str::to_owned))
        .unwrap_or_else(|| fallback.to_owned())
}
pub fn has(key: &str) -> bool {
    current().is_some_and(|l| l.get(key).is_some())
}
/// I18n.get with plain string arguments.
pub fn tr(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    crate::text::Text::tr(key, args).plain()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Mem(HashMap<String, Vec<u8>>);
    impl ResourceSource for Mem {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.get(path).cloned()
        }
    }
    #[test]
    fn bundled_horizon_strings_follow_the_language() {
        // No prepared resources at all: the namespace is compiled in.
        let empty = Mem(HashMap::new());
        let en = Language::load(&[&empty], "en_us");
        assert_eq!(en.get("horizon.options.controller"), Some("Controller"));
        let ru = Language::load(&[&empty], "ru_ru");
        assert_eq!(ru.get("horizon.options.controller"), Some("Геймпад"));
        // Every bundled string carries both languages: a key added to one file
        // and forgotten in the other shows as a key name on screen.
        for key in en.map().keys() {
            assert!(ru.get(key).is_some(), "{key} has no ru_ru value");
        }
        assert_eq!(en.get("horizon.pages.mods"), Some("Mods"));
        assert_eq!(ru.get("horizon.pages.mods"), Some("Моды"));
        let zh = Language::load(&[&empty], "zh_cn");
        assert_eq!(zh.get("horizon.options.controller"), Some("手柄"));
        assert_eq!(zh.get("horizon.pages.mods"), Some("模组"));
        assert_eq!(zh.get("Preset"), Some("预设"));
        assert_eq!(zh.get("%s chunks"), Some("%s 个区块"));
        // Not every bundle is complete: the en_us layer is in every map, so a
        // missing key falls back to English (`horizon.mods.not_built` is not
        // in ru_ru either).
        assert_eq!(zh.get("LOD Distance"), Some("LOD 距离"));
        assert_eq!(ru.get("horizon.mods.not_built"), Some(" (Not built)"));
        let pl = Language::load(&[&empty], "pl_pl");
        assert_eq!(pl.get("horizon.options.controller"), Some("Kontroler"));
        assert_eq!(pl.get("horizon.options.vibration"), Some("Wibracje"));
        let uk = Language::load(&[&empty], "uk_ua");
        assert_eq!(
            uk.get("horizon.options.on_screen_keyboard"),
            Some("Екранна клавіатура")
        );
        let tr = Language::load(&[&empty], "tr_tr");
        assert_eq!(tr.get("horizon.options.controller"), Some("Kontrolcü"));
        let es = Language::load(&[&empty], "es_es");
        assert_eq!(es.get("horizon.options.controller"), Some("Mando"));
        let es_ve = Language::load(&[&empty], "es_ve");
        assert_eq!(
            es_ve.get("horizon.options.on_screen_keyboard"),
            Some("Teclado en pantalla")
        );
    }

    #[test]
    fn layers_selected_language_over_english() {
        let base = Mem(HashMap::from([
            ("assets/minecraft/lang/en_us.json".into(), br#"{"a":"Apple","b":"Banana"}"#.to_vec()),
            ("assets/minecraft/lang/fr_fr.json".into(), br#"{"a":"Pomme"}"#.to_vec()),
            ("assets/pack.mcmeta".into(), r#"{"language":{"fr_fr":{"name":"Français","region":"France"},"de_de":{"name":"Deutsch","region":"Deutschland"}}}"#.as_bytes().to_vec()),
        ]));
        let pack = Mem(HashMap::from([(
            "assets/minecraft/lang/fr_fr.json".into(),
            br#"{"b":"Banane"}"#.to_vec(),
        )]));
        let sources: [&dyn ResourceSource; 2] = [&base, &pack];
        let fr = Language::load(&sources, "fr_fr");
        assert_eq!(fr.get("a"), Some("Pomme"));
        assert_eq!(fr.get("b"), Some("Banane"));
        let langs = available(&sources);
        // de_de has no file, so only English and French are offered.
        assert_eq!(
            langs.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(),
            ["en_us", "fr_fr"]
        );
        assert_eq!(langs[1].label(), "Français (France)");
    }

    #[test]
    fn one_eight_lang_lines_follow_locale() {
        // Locale.loadLocaleData: no trimming, first `=` only, `#` only at the
        // start of a line, lines without `=` skipped, `%d`/`%f` become `%s`.
        let text = concat!(
            "# a comment\n",
            "\n",
            "   \n",
            "pack.name=Item Pack\n",
            "item.apple=Apple\n",
            "item.bread=Bread=Baguette\n",
            "crlf=Carriage\r\n",
            "dup=first\n",
            "dup=second\n",
            "empty=\n",
            " = \n",
            "no separator\n",
            "format.d=%d items\n",
            "format.position=%1$d\n",
            "format.decimal=%.2f\n",
            "format.escaped=%%d\n",
            "hash=value # not a comment\n",
            "unicode=Привет\n",
            "literal=\\u00e9\n",
        );
        let map = parse_legacy_lang(text.as_bytes());
        assert_eq!(map.get("pack.name").map(String::as_str), Some("Item Pack"));
        assert_eq!(map.get("item.apple").map(String::as_str), Some("Apple"));
        assert_eq!(
            map.get("item.bread").map(String::as_str),
            Some("Bread=Baguette")
        );
        // BufferedReader.readLine also ends lines at a lone `\r`.
        assert_eq!(map.get("crlf").map(String::as_str), Some("Carriage"));
        // A later line with the same key wins, as `Map.put` does.
        assert_eq!(map.get("dup").map(String::as_str), Some("second"));
        assert_eq!(map.get("empty").map(String::as_str), Some(""));
        // Vanilla trims no line, so the spaces around `=` stay.
        assert_eq!(map.get(" ").map(String::as_str), Some(" "));
        for skipped in ["no separator", "   ", "# a comment"] {
            assert!(!map.contains_key(skipped), "{skipped:?} was read");
        }
        assert_eq!(map.get("format.d").map(String::as_str), Some("%s items"));
        assert_eq!(map.get("format.position").map(String::as_str), Some("%1$s"));
        assert_eq!(map.get("format.decimal").map(String::as_str), Some("%s"));
        // The pattern cannot match the first `%`, so only the second `%d` moves.
        assert_eq!(map.get("format.escaped").map(String::as_str), Some("%%s"));
        assert_eq!(
            map.get("hash").map(String::as_str),
            Some("value # not a comment")
        );
        assert_eq!(map.get("unicode").map(String::as_str), Some("Привет"));
        // Not a Java properties file: a backslash is only a backslash.
        assert_eq!(map.get("literal").map(String::as_str), Some("\\u00e9"));
        // Java's UTF-8 reader replaces malformed bytes instead of failing.
        let broken = parse_legacy_lang(b"key=caf\xff\n");
        assert_eq!(broken.get("key").map(String::as_str), Some("caf\u{fffd}"));
    }

    /// 1.8 keys an enchantment `enchantment.damage.all`, not the current
    /// `enchantment.minecraft.sharpness`: a locale's own translation moves to
    /// the current key, and only a missing one falls back to English.
    #[test]
    fn a_one_eight_locale_names_the_enchantments_in_its_own_language() {
        let pack = Mem(HashMap::from([(
            "assets/minecraft/lang/de_DE.lang".into(),
            "tile.stone.stone.name=Stein\nenchantment.damage.all=Sch\u{e4}rfe\nenchantment.protect.all=Schutz\n"
                .as_bytes()
                .to_vec(),
        )]));
        let sources: [&dyn ResourceSource; 1] = [&pack];
        let de = Language::load(&sources, "de_de");
        assert_eq!(
            de.get("enchantment.minecraft.sharpness"),
            Some("Sch\u{e4}rfe")
        );
        assert_eq!(de.get("enchantment.minecraft.protection"), Some("Schutz"));
        // Smite is not in this locale's file, so the English fallback stands.
        assert_eq!(de.get("enchantment.minecraft.smite"), Some("Smite"));
    }

    /// 1.8 keys an item `tile.clay.name`, and one locale file names them all.
    /// `Language::load` moves a locale's value onto the current item key and
    /// composes a spawn egg from `item.monsterPlacer` and the entity name;
    /// only a name the file lacks falls back to the 1.8 English table.
    #[test]
    fn a_one_eight_locale_names_the_items_in_its_own_language() {
        let pack = Mem(HashMap::from([(
            "assets/minecraft/lang/de_DE.lang".into(),
            "tile.stone.stone.name=Stein\ntile.clay.name=Ton\nitem.clay.name=Tonkugel\n\
             tile.dirt.default.name=Erde\nitem.banner.red.name=Rotes Banner\n\
             item.monsterPlacer.name=Spawn\nentity.Pig.name=Schwein\n"
                .as_bytes()
                .to_vec(),
        )]));
        let sources: [&dyn ResourceSource; 1] = [&pack];
        let de = Language::load(&sources, "de_de");
        assert_eq!(de.get("block.minecraft.clay"), Some("Ton"));
        assert_eq!(de.get("item.minecraft.clay_ball"), Some("Tonkugel"));
        assert_eq!(de.get("block.minecraft.dirt"), Some("Erde"));
        assert_eq!(de.get("block.minecraft.red_banner"), Some("Rotes Banner"));
        assert_eq!(
            de.get("item.minecraft.pig_spawn_egg"),
            Some("Spawn Schwein")
        );
        // Not in the locale: the 1.8 English table answers.
        assert_eq!(de.get("block.minecraft.stone"), Some("Stone"));
    }

    #[test]
    fn a_one_eight_language_file_is_the_fallback_and_layers_over_the_base() {
        let base = Mem(HashMap::from([(
            "assets/minecraft/lang/en_us.json".into(),
            br#"{"a":"Apple","b":"Banana"}"#.to_vec(),
        )]));
        let pack = Mem(HashMap::from([(
            "assets/minecraft/lang/en_US.lang".into(),
            b"a=Apfel\nc=Kirsche\n".to_vec(),
        )]));
        let sources: [&dyn ResourceSource; 2] = [&base, &pack];
        let en = Language::load(&sources, "en_us");
        // The pack's 1.8 file layers over the base's JSON, and its keys win.
        assert_eq!(en.get("a"), Some("Apfel"));
        assert_eq!(en.get("b"), Some("Banana"));
        assert_eq!(en.get("c"), Some("Kirsche"));
        // Within one source the 26.3 file is the one read: a modern pack that
        // also ships a 1.8 file keeps resolving as it did.
        let both = Mem(HashMap::from([
            (
                "assets/minecraft/lang/en_us.json".into(),
                br#"{"a":"json"}"#.to_vec(),
            ),
            (
                "assets/minecraft/lang/en_US.lang".into(),
                b"a=lang\n".to_vec(),
            ),
        ]));
        assert_eq!(Language::load(&[&both], "en_us").get("a"), Some("json"));
    }

    #[test]
    fn one_eight_languages_are_listed_from_their_lang_files() {
        let pack = Mem(HashMap::from([
            (
                "assets/pack.mcmeta".into(),
                r#"{"language":{"de_DE":{"name":"Deutsch","region":"Deutschland"},"fr_FR":{"name":"Français","region":"France"}}}"#
                    .as_bytes()
                    .to_vec(),
            ),
            (
                "assets/minecraft/lang/de_DE.lang".into(),
                b"a=Anmelden\n".to_vec(),
            ),
        ]));
        let langs = available(&[&pack]);
        // fr_FR ships neither a `.json` nor a `.lang` file.
        assert_eq!(
            langs.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(),
            ["de_DE", "en_us"]
        );
        assert_eq!(Language::load(&[&pack], "de_DE").get("a"), Some("Anmelden"));
    }

    #[test]
    fn languages_are_listed_without_reading_their_files() {
        struct NoLangReads;
        impl ResourceSource for NoLangReads {
            fn read(&self, path: &str) -> Option<Vec<u8>> {
                assert!(!path.contains("/lang/"), "read {path}");
                (path == "assets/pack.mcmeta").then(|| {
                    br#"{"language":{"fr_fr":{"name":"French"},"de_de":{"name":"German"}}}"#
                        .to_vec()
                })
            }
            fn exists(&self, path: &str) -> bool {
                path == "assets/minecraft/lang/fr_fr.json"
            }
        }
        let langs = available(&[&NoLangReads]);
        assert_eq!(
            langs.iter().map(|l| l.code.as_str()).collect::<Vec<_>>(),
            ["en_us", "fr_fr"]
        );
    }

    #[test]
    fn a_one_eight_root_names_items_with_the_current_keys() {
        // 1.8's lang file only has `item.diamond.name`; the current item data
        // asks for `item.minecraft.diamond`.
        let old = Mem(HashMap::from([(
            "assets/minecraft/lang/en_US.lang".into(),
            b"tile.stone.stone.name=Stone\nitem.diamond.name=Diamond\n".to_vec(),
        )]));
        let en = Language::load(&[&old], "en_us");
        assert_eq!(en.get("item.minecraft.diamond"), Some("Diamond"));
        assert_eq!(en.get("block.minecraft.red_wool"), Some("Red Wool"));
        assert_eq!(en.get("block.minecraft.red_banner"), Some("Red Banner"));
        assert_eq!(
            en.get("item.minecraft.diamond_sword"),
            Some("Diamond Sword")
        );
        // A current root already carries these keys, and keeps its own text.
        let modern = Mem(HashMap::from([(
            "assets/minecraft/lang/en_us.json".into(),
            br#"{"item.minecraft.diamond":"Diamant"}"#.to_vec(),
        )]));
        let en = Language::load(&[&modern], "en_us");
        assert_eq!(en.get("item.minecraft.diamond"), Some("Diamant"));
        assert_eq!(en.get("block.minecraft.red_wool"), None);
    }

    #[test]
    fn a_one_eight_root_still_names_the_enchantments() {
        // 1.8's lang file keys the enchantments by `enchantment.damage.all`
        // and friends, so the current description keys have to come from the
        // English fallback.
        let base = Mem(HashMap::new());
        let en = Language::load(&[&base], "en_us");
        assert_eq!(en.get("enchantment.minecraft.sharpness"), Some("Sharpness"));
        assert_eq!(
            en.get("enchantment.minecraft.sweeping_edge"),
            Some("Sweeping Edge")
        );
        assert_eq!(
            en.get("enchantment.minecraft.vanishing_curse"),
            Some("Curse of Vanishing")
        );
    }
}
