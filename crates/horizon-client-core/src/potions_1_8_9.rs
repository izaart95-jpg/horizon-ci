//! 1.8.9 potions: the damage-value bitmask, its effects, colours, names and
//! brewing reagent strings.
//!
//! A 1.8 potion is an `ItemStack` with `Damage` in `0..=32767`: bits 0..3 are
//! the effect flags, bit 4 the awkward (nether wart) marker, bit 5 glowstone
//! (level II), bit 6 redstone (extended), bit 13 the marker every brewed
//! effect potion carries and bit 14 "splash". The tables here are the 1.8
//! `StatusEffectStrings` results, checked against the 1.8.9 jar
//! (`docs/mc-1.8.9.md`); the datafixer's flattened potion names are mapped
//! back with [`legacy_damage`] so a current `potion_contents` still brews,
//! colours and names like 1.8.

/// A 1.8 `StatusEffect`, in the jar's id order (1..=23).
#[derive(Clone, Copy, Debug)]
pub struct EffectDef18 {
    /// The current effect name (`minecraft:speed`).
    pub id: &'static str,
    /// The 1.8 translation key (`potion.moveSpeed`).
    pub key: &'static str,
    pub color: u32,
    pub instant: bool,
    pub harmful: bool,
    /// `StatusEffect.method_2444`: the icon cell of `inventory.png`
    /// (`(icon % 8) * 18`, `198 + (icon / 8) * 18`), `None` for the instant
    /// effects, which 1.8 registers without an icon.
    pub icon: Option<u8>,
}

const fn fx(
    id: &'static str,
    key: &'static str,
    color: u32,
    instant: bool,
    harmful: bool,
) -> EffectDef18 {
    EffectDef18 {
        id,
        key,
        color,
        instant,
        harmful,
        icon: None,
    }
}

impl EffectDef18 {
    /// `StatusEffect.method_2440(column, row)`: the icon cell.
    const fn icon(self, column: u8, row: u8) -> Self {
        Self {
            icon: Some(column + row * 8),
            ..self
        }
    }
}

/// `StatusEffect.STATUS_EFFECTS`, ids 1..=23.
pub const EFFECTS: &[EffectDef18] = &[
    fx("minecraft:speed", "potion.moveSpeed", 8171462, false, false).icon(0, 0),
    fx(
        "minecraft:slowness",
        "potion.moveSlowdown",
        5926017,
        false,
        true,
    )
    .icon(1, 0),
    fx("minecraft:haste", "potion.digSpeed", 14270531, false, false).icon(2, 0),
    fx(
        "minecraft:mining_fatigue",
        "potion.digSlowDown",
        4866583,
        false,
        true,
    )
    .icon(3, 0),
    fx(
        "minecraft:strength",
        "potion.damageBoost",
        9643043,
        false,
        false,
    )
    .icon(4, 0),
    fx(
        "minecraft:instant_health",
        "potion.heal",
        16262179,
        true,
        false,
    ),
    fx(
        "minecraft:instant_damage",
        "potion.harm",
        4393481,
        true,
        true,
    ),
    fx("minecraft:jump_boost", "potion.jump", 2293580, false, false).icon(2, 1),
    fx("minecraft:nausea", "potion.confusion", 5578058, false, true).icon(3, 1),
    fx(
        "minecraft:regeneration",
        "potion.regeneration",
        13458603,
        false,
        false,
    )
    .icon(7, 0),
    fx(
        "minecraft:resistance",
        "potion.resistance",
        10044730,
        false,
        false,
    )
    .icon(6, 1),
    fx(
        "minecraft:fire_resistance",
        "potion.fireResistance",
        14981690,
        false,
        false,
    )
    .icon(7, 1),
    fx(
        "minecraft:water_breathing",
        "potion.waterBreathing",
        3035801,
        false,
        false,
    )
    .icon(0, 2),
    fx(
        "minecraft:invisibility",
        "potion.invisibility",
        8356754,
        false,
        false,
    )
    .icon(0, 1),
    fx(
        "minecraft:blindness",
        "potion.blindness",
        2039587,
        false,
        true,
    )
    .icon(5, 1),
    fx(
        "minecraft:night_vision",
        "potion.nightVision",
        2039713,
        false,
        false,
    )
    .icon(4, 1),
    fx("minecraft:hunger", "potion.hunger", 5797459, false, true).icon(1, 1),
    fx(
        "minecraft:weakness",
        "potion.weakness",
        4738376,
        false,
        true,
    )
    .icon(5, 0),
    fx("minecraft:poison", "potion.poison", 5149489, false, true).icon(6, 0),
    fx("minecraft:wither", "potion.wither", 3484199, false, true).icon(1, 2),
    fx(
        "minecraft:health_boost",
        "potion.healthBoost",
        16284963,
        false,
        false,
    )
    .icon(2, 2),
    fx(
        "minecraft:absorption",
        "potion.absorption",
        2445989,
        false,
        false,
    )
    .icon(2, 2),
    fx(
        "minecraft:saturation",
        "potion.saturation",
        16262179,
        true,
        false,
    ),
];

/// The effect id (1..=23) of a current effect name.
pub fn effect_id(id: &str) -> Option<u8> {
    let short = id.strip_prefix("minecraft:").unwrap_or(id);
    EFFECTS
        .iter()
        .position(|e| e.id.strip_prefix("minecraft:") == Some(short))
        .map(|i| i as u8 + 1)
}

/// The 1.8 `StatusEffect` of an id.
pub fn effect(id: u8) -> Option<&'static EffectDef18> {
    EFFECTS.get(usize::from(id).checked_sub(1)?)
}

/// The 1.8 translation key of an effect id (`potion.moveSpeed`).
pub fn effect_key(id: u8) -> Option<&'static str> {
    effect(id).map(|e| e.key)
}

/// The current effect name of a 1.8 effect id.
pub fn effect_name(id: u8) -> Option<&'static str> {
    effect(id).map(|e| e.id)
}

pub fn instant(id: u8) -> bool {
    effect(id).is_some_and(|e| e.instant)
}

pub fn harmful(id: u8) -> bool {
    effect(id).is_some_and(|e| e.harmful)
}

/// `StatusEffectStrings.getPotionEffects`: the effects of the low seven bits
/// of a 1.8 potion damage value, in effect-id order, as
/// `(effect id, base duration, amplifier)`. The splash bit scales the
/// durations through [`scaled_duration`].
pub fn effects(damage: i32) -> &'static [(u8, i32, i32)] {
    BASE_EFFECTS[(damage & 127) as usize]
}

/// `StatusEffectStrings.getPotionEffects`'s splash scaling for a non-instant
/// duration: `round(duration * 0.75 + 0.5)`.
pub fn scaled_duration(duration: i32) -> i32 {
    (f64::from(duration) * 0.75 + 0.5).round() as i32
}

/// The duration of one effect of `damage`, splash-scaled when bit 14 is set.
pub fn effect_duration(damage: i32, id: u8, duration: i32) -> i32 {
    if instant(id) || damage & 0x4000 == 0 {
        duration
    } else {
        scaled_duration(duration)
    }
}

pub fn is_throwable(damage: i32) -> bool {
    damage & 0x4000 != 0
}

/// `PotionItem.isThrowable` for a splash-potion stack: the 1.8 bit and the
/// current item agree (the datafixer turns bit 14 into `minecraft:splash_potion`).
pub fn is_splash_item(item: &str) -> bool {
    item == "minecraft:splash_potion"
}

/// `StatusEffectInstance.equals` over two damage values: the effect list,
/// durations (splash-scaled), amplifiers and the splash flag.
pub fn same_effects(a: i32, b: i32) -> bool {
    if is_throwable(a) != is_throwable(b) {
        return false;
    }
    let (ea, eb) = (effects(a), effects(b));
    ea.len() == eb.len()
        && ea.iter().zip(eb).all(|(&(ia, da, aa), &(ib, db, ab))| {
            ia == ib && aa == ab && effect_duration(a, ia, da) == effect_duration(b, ib, db)
        })
}

/// `StatusEffectStrings.method_3472`: apply a reagent's bit-string to a potion
/// damage value.
///
/// `+n` sets bit `n`, `-n` clears it, `&n` requires it (else the whole result
/// is 0) and `!n` toggles it; the final value keeps 15 bits.
pub fn apply(mut damage: i32, ops: &str) -> i32 {
    let mut number = 0i32;
    let mut has_number = false;
    let mut minus = false;
    let mut toggle = false;
    let mut require = false;
    let op = |damage: &mut i32, n: i32, minus: bool, toggle: bool, require: bool| {
        let Some(n) = u32::try_from(n).ok().filter(|&n| n < 31) else {
            return;
        };
        let bit = 1i32 << n;
        if require {
            if *damage & bit == 0 {
                *damage = 0;
            }
        } else if minus {
            *damage &= !bit;
        } else if toggle {
            *damage ^= bit;
        } else {
            *damage |= bit;
        }
    };
    for c in ops.chars() {
        if c.is_ascii_digit() {
            number = number * 10 + (c as i32 - '0' as i32);
            has_number = true;
            continue;
        }
        if has_number {
            op(&mut damage, number, minus, toggle, require);
            minus = false;
            toggle = false;
            require = false;
            has_number = false;
            number = 0;
        }
        match c {
            '!' => toggle = true,
            '-' => minus = true,
            '+' => {}
            '&' => require = true,
            _ => {}
        }
    }
    if has_number {
        op(&mut damage, number, minus, toggle, require);
    }
    damage & 0x7FFF
}

/// The `statusEffectString` of a 1.8 reagent item, by current item name.
pub fn reagent_string(item: &str) -> Option<&'static str> {
    Some(match item.strip_prefix("minecraft:").unwrap_or(item) {
        "sugar" => "-0+1-2-3&4-4+13",
        "ghast_tear" => "+0-1-2-3&4-4+13",
        "spider_eye" => "-0-1+2-3&4-4+13",
        "fermented_spider_eye" => "-0+3-4+13",
        "glistering_melon_slice" => "+0-1+2-3&4-4+13",
        "blaze_powder" => "+0-1-2+3&4-4+13",
        "magma_cream" => "+0+1-2-3&4-4+13",
        "redstone" => "-5+6-7",
        "glowstone_dust" => "+5-6-7",
        "gunpowder" => "+14&13-13",
        "golden_carrot" => "-0+1+2-3+13&4-4",
        "pufferfish" => "+0-1+2+3+13&4-4",
        "rabbit_foot" => "+0+1-2+3&4-4+13",
        "nether_wart" => "+4",
        _ => return None,
    })
}

/// Whether a current item is one of the 1.8 brewing reagents.
pub fn is_reagent(item: &str) -> bool {
    reagent_string(item).is_some()
}

/// The 1.8 `Damage` value of a current potion name, with the splash item's
/// bit. `None` for potions that did not exist in 1.8.
pub fn legacy_damage(potion: Option<&str>, splash: bool) -> Option<i32> {
    let name = potion?.strip_prefix("minecraft:").unwrap_or("");
    let base = match name {
        "water" => 0,
        "awkward" => 16,
        "thick" => 32,
        // The 1.8 mundane potion is brewed, so it carries bit 13 (the sugar,
        // ghast tear or spider eye marker). Damage 64 is its 1.7.10 spelling.
        "mundane" => 0x2000,
        // Bits 1..4: the effect flags. Bit 13 marks every brewed effect potion.
        "regeneration" => 0x2000 | 1,
        "swiftness" => 0x2000 | 2,
        "fire_resistance" => 0x2000 | 3,
        "poison" => 0x2000 | 4,
        "healing" => 0x2000 | 5,
        "night_vision" => 0x2000 | 6,
        "weakness" => 0x2000 | 8,
        "strength" => 0x2000 | 9,
        "slowness" => 0x2000 | 10,
        "leaping" => 0x2000 | 11,
        "harming" => 0x2000 | 12,
        "water_breathing" => 0x2000 | 13,
        "invisibility" => 0x2000 | 14,
        // Redstone: bit 6 (extended).
        "long_regeneration" => 0x2000 | 0x40 | 1,
        "long_swiftness" => 0x2000 | 0x40 | 2,
        "long_fire_resistance" => 0x2000 | 0x40 | 3,
        "long_poison" => 0x2000 | 0x40 | 4,
        "long_night_vision" => 0x2000 | 0x40 | 6,
        "long_weakness" => 0x2000 | 0x40 | 8,
        "long_strength" => 0x2000 | 0x40 | 9,
        "long_slowness" => 0x2000 | 0x40 | 10,
        "long_leaping" => 0x2000 | 0x40 | 11,
        "long_water_breathing" => 0x2000 | 0x40 | 13,
        "long_invisibility" => 0x2000 | 0x40 | 14,
        // Glowstone: bit 5 (level II).
        "strong_regeneration" => 0x2000 | 0x20 | 1,
        "strong_swiftness" => 0x2000 | 0x20 | 2,
        "strong_poison" => 0x2000 | 0x20 | 4,
        "strong_healing" => 0x2000 | 0x20 | 5,
        "strong_strength" => 0x2000 | 0x20 | 9,
        "strong_leaping" => 0x2000 | 0x20 | 11,
        "strong_harming" => 0x2000 | 0x20 | 12,
        _ => return None,
    };
    // Gunpowder clears bit 13; a real splash potion therefore carries only
    // bit 14 (and the effect bits).
    Some(if splash {
        (base & !0x2000) | 0x4000
    } else {
        base
    })
}

/// The current potion name of a 1.8 damage value (the datafixer's `ITEM_POTIONS`
/// low-seven-bit entry), `None` when the low bits name no potion.
///
/// The fixer's `damage & 127` loses bits 13 and 14, so a brewed potion with no
/// effect bits (8192: sugar, ghast tear or spider eye on a water bottle) and
/// its splash form (16384: gunpowder on that bottle) read as water. 1.8 has no
/// splash water, so both are the 1.8 mundane potion.
pub fn potion_name(damage: i32) -> Option<&'static str> {
    if damage & 0x7F == 0 && damage & 0x6000 != 0 {
        return Some("minecraft:mundane");
    }
    horizon_world::datafix::flat_tables::ITEM_POTIONS
        .iter()
        .find(|&&(d, _)| d == damage & 127)
        .map(|&(_, name)| name)
}

/// `StatusEffectStrings.method_3475`: the colour mix of a potion's effects,
/// each weighted by `amplifier + 1`, the 1.8 base colour when it has none.
pub fn potion_color(damage: i32) -> u32 {
    let mut f = 0.0f32;
    let mut g = 0.0f32;
    let mut h = 0.0f32;
    let mut j = 0.0f32;
    for &(id, _, amplifier) in effects(damage) {
        let Some(e) = effect(id) else { continue };
        for _ in 0..=amplifier {
            f += (e.color >> 16 & 255) as f32 / 255.0;
            g += (e.color >> 8 & 255) as f32 / 255.0;
            h += (e.color & 255) as f32 / 255.0;
            j += 1.0;
        }
    }
    if j == 0.0 {
        return 0x385DC6;
    }
    let f = f / j * 255.0;
    let g = g / j * 255.0;
    let h = h / j * 255.0;
    ((f as u32) << 16) | ((g as u32) << 8) | (h as u32)
}

/// `StatusEffectStrings.method_3478`: the `potion.prefix.*` key of a potion
/// without effects (bits 5..1 select one of the 32 names).
pub fn prefix_key(damage: i32) -> &'static str {
    PREFIXES[((damage >> 1) & 31) as usize]
}

const PREFIXES: [&str; 32] = [
    "potion.prefix.mundane",
    "potion.prefix.uninteresting",
    "potion.prefix.bland",
    "potion.prefix.clear",
    "potion.prefix.milky",
    "potion.prefix.diffuse",
    "potion.prefix.artless",
    "potion.prefix.thin",
    "potion.prefix.awkward",
    "potion.prefix.flat",
    "potion.prefix.bulky",
    "potion.prefix.bungling",
    "potion.prefix.buttered",
    "potion.prefix.smooth",
    "potion.prefix.suave",
    "potion.prefix.debonair",
    "potion.prefix.thick",
    "potion.prefix.elegant",
    "potion.prefix.fancy",
    "potion.prefix.charming",
    "potion.prefix.dashing",
    "potion.prefix.refined",
    "potion.prefix.cordial",
    "potion.prefix.sparkling",
    "potion.prefix.potent",
    "potion.prefix.foul",
    "potion.prefix.odorless",
    "potion.prefix.rank",
    "potion.prefix.harsh",
    "potion.prefix.acrid",
    "potion.prefix.gross",
    "potion.prefix.stinky",
];

/// `ChatUtil.ticksToString`: `minutes:seconds`, the seconds two digits.
pub fn format_duration(ticks: i32) -> String {
    let secs = ticks / 20;
    let minutes = secs / 60;
    let seconds = secs % 60;
    format!("{minutes}:{seconds:02}")
}

/// The 1.8 tooltip colour of a line: `Formatting.RED` for a harmful effect,
/// `GRAY` otherwise, the dark purple "When Applied:" header and the blue/red
/// attribute lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineColor {
    Red,
    Gray,
    DarkPurple,
    Blue,
}

/// `PotionItem.getDisplayName`: `Water Bottle` at damage 0, an optional
/// `Splash ` prefix, the first effect's `.postfix` name (custom effects
/// replace the damage's, as `getCustomPotionEffects` does), else the potion
/// prefix and the item's own name. `item_name` is the translated
/// `item.potion.name` (`Potion`).
pub fn display_name(
    damage: i32,
    custom: &[(String, i32, i32)],
    translate: &dyn Fn(&str) -> Option<String>,
    item_name: &str,
) -> String {
    if damage == 0 {
        return translate("item.emptyPotion.name")
            .unwrap_or_else(|| "Water Bottle".into())
            .trim()
            .to_owned();
    }
    let mut name = String::new();
    if is_throwable(damage) {
        name.push_str(
            translate("potion.prefix.grenade")
                .unwrap_or_else(|| "Splash".into())
                .trim(),
        );
        name.push(' ');
    }
    let first = if custom.is_empty() {
        effects(damage).first().map(|&(id, ..)| id)
    } else {
        custom.first().and_then(|(id, ..)| effect_id(id))
    };
    match first {
        Some(id) => {
            let key = effect_key(id).unwrap_or("potion.moveSpeed");
            let postfix = format!("{key}.postfix");
            name.push_str(translate(&postfix).unwrap_or_else(|| key.to_owned()).trim());
        }
        None => {
            name.push_str(
                translate(prefix_key(damage))
                    .unwrap_or_else(|| prefix_key(damage).to_owned())
                    .trim(),
            );
            name.push(' ');
            name.push_str(item_name);
        }
    }
    name
}

/// `PotionItem.appendTooltip`: no lines for a water bottle, one line per
/// effect (`name`, optional potency, `(m:ss)` past a second), then the
/// `When Applied:` attribute block. `custom` is the stack's
/// `CustomPotionEffects`; when present it replaces the damage's effects, as
/// `getCustomPotionEffects` does.
pub fn tooltip(
    damage: i32,
    custom: &[(String, i32, i32)],
    translate: &dyn Fn(&str) -> Option<String>,
) -> Vec<(String, LineColor)> {
    // `appendTooltip` returns before reading the effects of a water bottle,
    // custom ones included.
    if damage == 0 {
        return Vec::new();
    }
    let entries: Vec<(u8, i32, i32)> = if custom.is_empty() {
        effects(damage)
            .iter()
            .map(|&(id, duration, amp)| (id, effect_duration(damage, id, duration), amp))
            .collect()
    } else {
        custom
            .iter()
            .filter_map(|(id, duration, amp)| Some((effect_id(id)?, *duration, *amp)))
            .collect()
    };
    if entries.is_empty() {
        return vec![(
            translate("potion.empty")
                .unwrap_or_else(|| "No Effects".into())
                .trim()
                .to_owned(),
            LineColor::Gray,
        )];
    }
    let mut lines = Vec::new();
    let mut modifiers = Vec::new();
    for &(id, duration, amp) in &entries {
        let Some(def) = effect(id) else { continue };
        let mut line = translate(def.key)
            .unwrap_or_else(|| def.key.to_owned())
            .trim()
            .to_owned();
        if amp > 0 {
            let key = format!("potion.potency.{amp}");
            line.push(' ');
            line.push_str(
                translate(&key)
                    .unwrap_or_else(|| roman(amp as u32 + 1))
                    .trim(),
            );
        }
        if duration > 20 {
            line.push_str(" (");
            line.push_str(&format_duration(duration));
            line.push(')');
        }
        lines.push((
            line,
            if def.harmful {
                LineColor::Red
            } else {
                LineColor::Gray
            },
        ));
        if let Some((attribute, amount, operation)) = attribute_modifier(def.id, amp) {
            modifiers.push((attribute, amount, operation));
        }
    }
    if !modifiers.is_empty() {
        lines.push((String::new(), LineColor::DarkPurple));
        lines.push((
            translate("potion.effects.whenDrank")
                .unwrap_or_else(|| "When Applied:".into())
                .trim()
                .to_owned(),
            LineColor::DarkPurple,
        ));
        for (attribute, amount, operation) in modifiers {
            // `ItemStack.addAttributeTooltip`: multiplied operations show a
            // percentage, `take` shows the absolute value.
            let scale = if operation == 0 { 1.0 } else { 100.0 };
            let shown = amount * scale;
            let kind = if shown > 0.0 { "plus" } else { "take" };
            let key = format!("attribute.modifier.{kind}.{operation}");
            let attribute_name = translate(&format!("attribute.name.{attribute}"))
                .unwrap_or_else(|| attribute.to_owned());
            let line = crate::text::format_translation(
                &translate(&key).unwrap_or(key.clone()),
                &[
                    crate::text::Text::literal(decimal_3(shown.abs())),
                    crate::text::Text::literal(attribute_name),
                ],
            )
            .plain();
            lines.push((
                line,
                if shown > 0.0 {
                    LineColor::Blue
                } else {
                    LineColor::Red
                },
            ));
        }
    }
    lines
}

/// The 1.8 attribute modifier of an effect at an amplifier: (attribute id,
/// amount, operation). `CombatStatusEffect.adjustModifierAmount` gives
/// strength `1.3 * (amplifier + 1)` and weakness `-0.5 * (amplifier + 1)`,
/// unlike the current flat +3/-4; jump boost, haste and the other effects
/// have no 1.8 attribute modifier.
fn attribute_modifier(effect: &str, amp: i32) -> Option<(&'static str, f64, usize)> {
    let level = f64::from(amp + 1);
    Some(match effect.strip_prefix("minecraft:").unwrap_or(effect) {
        "speed" => ("generic.movementSpeed", 0.2 * level, 2),
        "slowness" => ("generic.movementSpeed", -0.15 * level, 2),
        "strength" => ("generic.attackDamage", 1.3 * level, 2),
        "weakness" => ("generic.attackDamage", -0.5 * level, 0),
        "health_boost" => ("generic.maxHealth", 4.0 * level, 0),
        _ => return None,
    })
}

/// `DecimalFormat("#.###")` for the attribute line.
fn decimal_3(value: f64) -> String {
    let mut text = format!("{value:.3}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    text
}

/// The roman numeral `potion.potency` uses; the fallback when a language
/// lacks the key.
fn roman(value: u32) -> String {
    match value {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        _ => return value.to_string(),
    }
    .to_owned()
}

/// `StatusEffectStrings.getPotionEffects` for damage 0..=127, from the 1.8.9
/// jar: `(effect id, duration, amplifier)`.
static BASE_EFFECTS: &[&[(u8, i32, i32)]] = &[
    &[],
    &[(10, 900, 0)],
    &[(1, 3600, 0)],
    &[(12, 3600, 0)],
    &[(19, 900, 0)],
    &[(6, 1, 0)],
    &[(16, 3600, 0)],
    &[],
    &[(18, 1800, 0)],
    &[(5, 3600, 0)],
    &[(2, 1800, 0)],
    &[(8, 3600, 0)],
    &[(7, 1, 0)],
    &[(13, 3600, 0)],
    &[(14, 3600, 0)],
    &[],
    &[],
    &[(10, 900, 0)],
    &[(1, 3600, 0)],
    &[(12, 3600, 0)],
    &[(19, 900, 0)],
    &[(6, 1, 0)],
    &[(16, 3600, 0)],
    &[],
    &[(18, 1800, 0)],
    &[(5, 3600, 0)],
    &[(2, 1800, 0)],
    &[(8, 3600, 0)],
    &[(7, 1, 0)],
    &[(13, 3600, 0)],
    &[(14, 3600, 0)],
    &[],
    &[],
    &[(10, 450, 1)],
    &[(1, 1800, 1)],
    &[(12, 3600, 0)],
    &[(19, 450, 1)],
    &[(6, 1, 1)],
    &[(16, 3600, 0)],
    &[],
    &[(18, 1800, 0)],
    &[(5, 1800, 1)],
    &[(2, 1800, 0)],
    &[(8, 1800, 1)],
    &[(7, 1, 1)],
    &[(13, 3600, 0)],
    &[(14, 3600, 0)],
    &[],
    &[],
    &[(10, 450, 1)],
    &[(1, 1800, 1)],
    &[(12, 3600, 0)],
    &[(19, 450, 1)],
    &[(6, 1, 1)],
    &[(16, 3600, 0)],
    &[],
    &[(18, 1800, 0)],
    &[(5, 1800, 1)],
    &[(2, 1800, 0)],
    &[(8, 1800, 1)],
    &[(7, 1, 1)],
    &[(13, 3600, 0)],
    &[(14, 3600, 0)],
    &[],
    &[],
    &[(10, 2400, 0)],
    &[(1, 9600, 0)],
    &[(12, 9600, 0)],
    &[(19, 2400, 0)],
    &[(6, 1, 0)],
    &[(16, 9600, 0)],
    &[],
    &[(18, 4800, 0)],
    &[(5, 9600, 0)],
    &[(2, 4800, 0)],
    &[(8, 9600, 0)],
    &[(7, 1, 0)],
    &[(13, 9600, 0)],
    &[(14, 9600, 0)],
    &[],
    &[],
    &[(10, 2400, 0)],
    &[(1, 9600, 0)],
    &[(12, 9600, 0)],
    &[(19, 2400, 0)],
    &[(6, 1, 0)],
    &[(16, 9600, 0)],
    &[],
    &[(18, 4800, 0)],
    &[(5, 9600, 0)],
    &[(2, 4800, 0)],
    &[(8, 9600, 0)],
    &[(7, 1, 0)],
    &[(13, 9600, 0)],
    &[(14, 9600, 0)],
    &[],
    &[],
    &[(10, 1200, 1)],
    &[(1, 4800, 1)],
    &[(12, 9600, 0)],
    &[(19, 1200, 1)],
    &[(6, 1, 1)],
    &[(16, 9600, 0)],
    &[],
    &[(18, 4800, 0)],
    &[(5, 4800, 1)],
    &[(2, 4800, 0)],
    &[(8, 4800, 1)],
    &[(7, 1, 1)],
    &[(13, 9600, 0)],
    &[(14, 9600, 0)],
    &[],
    &[],
    &[(10, 1200, 1)],
    &[(1, 4800, 1)],
    &[(12, 9600, 0)],
    &[(19, 1200, 1)],
    &[(6, 1, 1)],
    &[(16, 9600, 0)],
    &[],
    &[(18, 4800, 0)],
    &[(5, 4800, 1)],
    &[(2, 4800, 0)],
    &[(8, 4800, 1)],
    &[(7, 1, 1)],
    &[(13, 9600, 0)],
    &[(14, 9600, 0)],
    &[],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_table_matches_the_1_8_9_jar() {
        // Samples from StatusEffectStrings.getPotionEffects.
        assert_eq!(effects(1), &[(10, 900, 0)]);
        assert_eq!(effects(2), &[(1, 3600, 0)]);
        assert_eq!(effects(33), &[(10, 450, 1)]);
        assert_eq!(effects(66), &[(1, 9600, 0)]);
        assert_eq!(effects(97), &[(10, 1200, 1)]);
        assert_eq!(effects(127), &[]);
        assert_eq!(effect_duration(0x4002, 1, 3600), 2701);
        assert_eq!(effect_duration(0x4001, 10, 900), 676);
        assert_eq!(effect_duration(0x4021, 10, 450), 338);
        assert_eq!(effect_duration(0x4041, 10, 2400), 1801);
        // Instant effects keep their one tick.
        assert_eq!(effect_duration(0x4005, 6, 1), 1);
    }

    #[test]
    fn reagent_strings_apply_like_the_jar() {
        // Water -> awkward -> swiftness -> long / strong / splash.
        let awkward = apply(0, reagent_string("minecraft:nether_wart").unwrap());
        assert_eq!(awkward, 16);
        let swift = apply(awkward, reagent_string("minecraft:sugar").unwrap());
        assert_eq!(swift, 8194);
        assert_eq!(
            apply(swift, reagent_string("minecraft:redstone").unwrap()),
            8258
        );
        assert_eq!(
            apply(swift, reagent_string("minecraft:glowstone_dust").unwrap()),
            8226
        );
        assert_eq!(
            apply(swift, reagent_string("minecraft:gunpowder").unwrap()),
            16386
        );
        // Gunpowder needs the 0x2000 marker: water does nothing.
        assert_eq!(apply(0, reagent_string("minecraft:gunpowder").unwrap()), 0);
        // A second glowstone or redstone is a no-op on the same potion.
        assert_eq!(
            apply(8226, reagent_string("minecraft:glowstone_dust").unwrap()),
            8226
        );
        assert_eq!(
            apply(8258, reagent_string("minecraft:redstone").unwrap()),
            8258
        );
        // The awkward marker is required and then cleared; bit 13 still marks
        // the result, so sugar on water is the 1.8 mundane potion (8192).
        assert_eq!(apply(0, reagent_string("minecraft:sugar").unwrap()), 8192);
    }

    #[test]
    fn legacy_damage_round_trips_every_1_8_potion_name() {
        for &(name, damage) in &[
            ("minecraft:water", 0),
            ("minecraft:awkward", 16),
            ("minecraft:thick", 32),
            ("minecraft:mundane", 8192),
            ("minecraft:regeneration", 8193),
            ("minecraft:swiftness", 8194),
            ("minecraft:strong_swiftness", 8226),
            ("minecraft:long_swiftness", 8258),
            ("minecraft:strong_regeneration", 8225),
            ("minecraft:long_regeneration", 8257),
            ("minecraft:strong_poison", 8228),
            ("minecraft:long_poison", 8260),
            ("minecraft:harming", 8204),
            ("minecraft:strong_harming", 8236),
        ] {
            assert_eq!(legacy_damage(Some(name), false), Some(damage), "{name}");
            assert_eq!(potion_name(damage), Some(name), "{damage}");
        }
        // Splash clears the marker and sets bit 14.
        assert_eq!(
            legacy_damage(Some("minecraft:swiftness"), true),
            Some(16386)
        );
        assert_eq!(
            legacy_damage(Some("minecraft:long_swiftness"), true),
            Some(16450)
        );
        // A splash mundane is gunpowder on a mundane bottle: bit 14 only.
        assert_eq!(legacy_damage(Some("minecraft:mundane"), true), Some(16384));
        assert_eq!(potion_name(16384), Some("minecraft:mundane"));
        // Damage 64 is the same potion's 1.7.10 spelling, and 0 has no bit 13
        // so it stays water.
        assert_eq!(potion_name(64), Some("minecraft:mundane"));
        assert_eq!(potion_name(0), Some("minecraft:water"));
        assert_eq!(potion_name(0x2000 | 0x10), Some("minecraft:awkward"));
        // Potions added after 1.8 have no legacy damage.
        assert_eq!(legacy_damage(Some("minecraft:turtle_master"), false), None);
        assert_eq!(legacy_damage(Some("minecraft:luck"), false), None);
    }

    #[test]
    fn colours_and_prefixes_match_the_jar() {
        assert_eq!(potion_color(8194), 8171462);
        assert_eq!(potion_color(8196), 5149489);
        assert_eq!(potion_color(0), 0x385DC6);
        assert_eq!(prefix_key(0), "potion.prefix.mundane");
        assert_eq!(prefix_key(2), "potion.prefix.uninteresting");
        assert_eq!(prefix_key(16), "potion.prefix.awkward");
        assert_eq!(prefix_key(32), "potion.prefix.thick");
        assert_eq!(prefix_key(64), "potion.prefix.mundane");
        assert_eq!(prefix_key(0x2030), "potion.prefix.potent");
    }

    #[test]
    fn same_effects_sees_splash_and_duration_changes() {
        assert!(same_effects(8194, 8210));
        assert!(!same_effects(8194, 16386));
        assert!(!same_effects(8194, 8258));
        assert!(!same_effects(8194, 8226));
        assert!(same_effects(0, 16));
    }

    #[test]
    fn durations_use_the_1_8_minutes_and_seconds_format() {
        assert_eq!(format_duration(900), "0:45");
        assert_eq!(format_duration(9600), "8:00");
        assert_eq!(format_duration(676), "0:33");
        assert_eq!(format_duration(1), "0:00");
    }

    /// The oracle's `D <damage> name=[...] lines=[...]` output, with 1.8's
    /// `en_US.lang` values behind the translation keys.
    #[test]
    fn display_name_and_tooltip_match_the_1_8_oracle() {
        let translate = |key: &str| {
            Some(
                match key {
                    "item.emptyPotion.name" => "Water Bottle",
                    "item.potion.name" => "Potion",
                    "potion.prefix.grenade" => "Splash",
                    "potion.prefix.mundane" => "Mundane",
                    "potion.prefix.awkward" => "Awkward",
                    "potion.prefix.potent" => "Potent",
                    "potion.empty" => "No Effects",
                    "potion.effects.whenDrank" => "When Applied:",
                    "potion.moveSpeed" => "Speed",
                    "potion.moveSpeed.postfix" => "Potion of Swiftness",
                    "potion.regeneration" => "Regeneration",
                    "potion.regeneration.postfix" => "Potion of Regeneration",
                    "potion.heal" => "Instant Health",
                    "potion.weakness" => "Weakness",
                    "potion.damageBoost" => "Strength",
                    "potion.moveSlowdown" => "Slowness",
                    "attribute.modifier.plus.2" => "+%s%% %s",
                    "attribute.modifier.take.2" => "-%s%% %s",
                    "attribute.modifier.take.0" => "-%s %s",
                    "attribute.name.generic.movementSpeed" => "Speed",
                    "attribute.name.generic.attackDamage" => "Attack Damage",
                    _ => return None,
                }
                .to_owned(),
            )
        };
        assert_eq!(display_name(0, &[], &translate, "Potion"), "Water Bottle");
        assert_eq!(
            display_name(8194, &[], &translate, "Potion"),
            "Potion of Swiftness"
        );
        assert_eq!(
            display_name(16386, &[], &translate, "Potion"),
            "Splash Potion of Swiftness"
        );
        assert_eq!(
            display_name(8208, &[], &translate, "Potion"),
            "Awkward Potion"
        );
        assert_eq!(
            display_name(8240, &[], &translate, "Potion"),
            "Potent Potion"
        );

        let gray = |s: &str| (s.to_owned(), LineColor::Gray);
        let red = |s: &str| (s.to_owned(), LineColor::Red);
        let purple = |s: &str| (s.to_owned(), LineColor::DarkPurple);
        let blue = |s: &str| (s.to_owned(), LineColor::Blue);
        assert!(tooltip(0, &[], &translate).is_empty());
        assert_eq!(
            tooltip(8193, &[], &translate),
            vec![gray("Regeneration (0:45)")]
        );
        assert_eq!(tooltip(8197, &[], &translate), vec![gray("Instant Health")]);
        assert_eq!(
            tooltip(8194, &[], &translate),
            vec![
                gray("Speed (3:00)"),
                purple(""),
                purple("When Applied:"),
                blue("+20% Speed"),
            ]
        );
        assert_eq!(
            tooltip(8200, &[], &translate),
            vec![
                red("Weakness (1:30)"),
                purple(""),
                purple("When Applied:"),
                red("-0.5 Attack Damage"),
            ]
        );
        assert_eq!(
            tooltip(8201, &[], &translate),
            vec![
                gray("Strength (3:00)"),
                purple(""),
                purple("When Applied:"),
                blue("+130% Attack Damage"),
            ]
        );
        assert_eq!(
            tooltip(16386, &[], &translate),
            vec![
                gray("Speed (2:15)"),
                purple(""),
                purple("When Applied:"),
                blue("+20% Speed"),
            ]
        );
        // An effect-less potion is one grey line, not the water bottle's none.
        assert_eq!(tooltip(8208, &[], &translate), vec![gray("No Effects")]);
    }
}
