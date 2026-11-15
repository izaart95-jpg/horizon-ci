//! 1.8.9's melee damage: the attack damage a weapon adds and the bonus
//! damage Sharpness, Smite and Bane of Arthropods add.
//!
//! 1.8 has no attack-speed attribute and no base-attack-damage modifier: a
//! sword or tool installs one `generic.attackDamage` modifier in the main
//! hand, and `EntityPlayer.attack` adds the enchantment damage on top of the
//! player's own 1.0 base.

use crate::items::ItemStack;
use horizon_world::nbt::Tag;

/// 1.8's `Item.ATTACK_DAMAGE_MODIFIER_UUID`; the datafix names it
/// `minecraft:base_attack_damage`, which is also the id the engine uses.
pub const ATTACK_DAMAGE_UUID: &str = "cb3f55d3-645c-4f38-a497-9c13a33db5cf";
/// `Item.ATTACK_SPEED_MODIFIER_UUID` (1.9's; the datafix keeps it too).
const ATTACK_SPEED_UUID: &str = "fa233e1c-4180-4865-b01b-bcce9785aca3";

/// `ToolMaterialType.getAttackMultiplier`: wood and gold add nothing, then
/// stone, iron and diamond add one, two and three.
fn material_bonus(material: &str) -> Option<f64> {
    Some(match material {
        "wooden" | "golden" => 0.0,
        "stone" => 1.0,
        "iron" => 2.0,
        "diamond" => 3.0,
        _ => return None,
    })
}

/// The `generic.attackDamage` modifier 1.8 gives a sword or tool in the main
/// hand: `SwordItem` adds 4 and `ToolItem` 3 for an axe, 2 for a pickaxe and
/// 1 for a shovel, each plus the material's damage. A hoe has none (1.8's
/// `HoeItem` is not a `ToolItem`), and neither has anything else.
///
/// `minecraft:diamond_sword` is 7, exactly the `+7 Attack Damage` line the
/// 1.8.9 client shows.
pub fn attack_damage(item: &str) -> Option<f64> {
    let path = item.strip_prefix("minecraft:").unwrap_or(item);
    let (material, base) = if let Some(m) = path.strip_suffix("_sword") {
        (m, 4.0)
    } else if let Some(m) = path.strip_suffix("_pickaxe") {
        (m, 2.0)
    } else if let Some(m) = path.strip_suffix("_shovel") {
        (m, 1.0)
    } else if let Some(m) = path.strip_suffix("_axe") {
        (m, 3.0)
    } else {
        return None;
    };
    Some(base + material_bonus(material)?)
}

/// `EnchantmentHelper.getAttackDamage`: Sharpness adds 1.25 per level to
/// every hit, Smite and Bane of Arthropods 2.5 per level, but only against
/// their own `EntityGroup`. The client asks with no target for the tooltip
/// (DEFAULT matches neither).
pub fn enchantment_bonus(stack: &ItemStack, undead: bool, arthropod: bool) -> f64 {
    let level = |id: &str| f64::from(stack.enchantment_level(id));
    let mut bonus = level("minecraft:sharpness") * 1.25;
    if undead {
        bonus += level("minecraft:smite") * 2.5;
    }
    if arthropod {
        bonus += level("minecraft:bane_of_arthropods") * 2.5;
    }
    bonus
}

/// The modifier `apply_equipment` installs for a weapon without its own
/// `AttributeModifiers`, in the shape the 26.3 components use: (attribute, id,
/// amount, operation, slot group). 1.8's `ATTACK_DAMAGE_MODIFIER_UUID` is the
/// datafix's `minecraft:base_attack_damage`, so a re-equip replaces it.
///
/// 1.8 has no slot groups: `LivingEntity` applies every equipped stack's
/// attributes, so the group is `any` (holding two swords makes the last one's
/// fixed-uuid modifier win, exactly as `replaceAll` does in 1.8).
pub fn attack_modifier(item: &str) -> Vec<(String, String, f64, String, String)> {
    attack_damage(item)
        .map(|amount| {
            vec![(
                "minecraft:attack_damage".to_owned(),
                "minecraft:base_attack_damage".to_owned(),
                amount,
                "add_value".to_owned(),
                "any".to_owned(),
            )]
        })
        .unwrap_or_default()
}

/// `ItemStack.getAttributes`: a stack's own `AttributeModifiers` NBT (loaded
/// as the `minecraft:attribute_modifiers` component) *replaces* the item's
/// defaults, it does not extend them. `None` means the stack has none and the
/// item's defaults apply; `Some` with an empty list means the stack carries
/// the component but no usable modifier, so nothing applies.
///
/// Each entry is (attribute, id, amount, operation, slot group). 1.8 stores no
/// slot groups, and its `LivingEntity` applies every equipped stack's
/// attributes, so every parsed entry is `any`.
pub fn stack_modifiers(stack: &ItemStack) -> Option<Vec<(String, String, f64, String, String)>> {
    let tag = stack
        .data()
        .and_then(|data| data.other("minecraft:attribute_modifiers"))?;
    // `{modifiers: [...]}` before the tooltip-display fix unwraps the list.
    let list = match tag {
        Tag::List(list) => list.as_slice(),
        Tag::Compound(map) => match map.get(&horizon_world::nbt::JavaString::from("modifiers")) {
            Some(Tag::List(list)) => list.as_slice(),
            _ => &[],
        },
        _ => &[],
    };
    Some(list.iter().filter_map(modifier_1_8_9).collect())
}

/// One `AttributeModifiers` entry. 1.8 skips a modifier whose UUID has a zero
/// half (`ItemStack.getAttributes`), and `EntityAttributes.fromNbt` drops an
/// operation outside 0..2.
fn modifier_1_8_9(tag: &Tag) -> Option<(String, String, f64, String, String)> {
    let Tag::Compound(map) = tag else {
        return None;
    };
    let field = |name: &str| map.get(&horizon_world::nbt::JavaString::from(name));
    let attribute = attribute_1_8_9(&as_str(field("type")?)?);
    let amount = as_double(field("amount")?)?;
    let operation = match field("operation")? {
        Tag::String(s) => match s.to_string_lossy().as_str() {
            "add_value" => 0,
            "add_multiplied_base" => 1,
            "add_multiplied_total" => 2,
            _ => return None,
        },
        tag => as_int(tag)?,
    };
    if !(0..=2).contains(&operation) {
        return None;
    }
    // 1.8 has no slot groups: every equipped stack's modifiers apply. The
    // component's `slot` is therefore dropped, not filtered on.
    let id = match field("id").and_then(as_str) {
        Some(id) => id,
        // Before the attribute-modifier-id fix the entry still has its UUID.
        None => uuid_text(field("uuid")?)?,
    };
    Some((
        attribute,
        id,
        amount,
        match operation {
            1 => "add_multiplied_base",
            2 => "add_multiplied_total",
            _ => "add_value",
        }
        .to_owned(),
        "any".to_owned(),
    ))
}

/// The engine's attribute id for a stored one: 1.8 names it `generic.attackDamage`,
/// the datafix `minecraft:generic.attack_damage`, the engine `attack_damage`.
fn attribute_1_8_9(name: &str) -> String {
    let short = name.strip_prefix("minecraft:").unwrap_or(name);
    let short = short
        .strip_prefix("generic.")
        .or_else(|| short.strip_prefix("horse."))
        .or_else(|| short.strip_prefix("zombie."))
        .unwrap_or(short);
    format!("minecraft:{short}")
}

/// `attribute.name.<n>`'s 1.8 key: the jar's language file only has the seven
/// attributes 1.8 knew, so anything else shows its raw key.
pub fn attribute_display_key(attribute: &str) -> String {
    let short = attribute.strip_prefix("minecraft:").unwrap_or(attribute);
    match short {
        "max_health" => "attribute.name.generic.maxHealth".to_owned(),
        "follow_range" => "attribute.name.generic.followRange".to_owned(),
        "knockback_resistance" => "attribute.name.generic.knockbackResistance".to_owned(),
        "movement_speed" => "attribute.name.generic.movementSpeed".to_owned(),
        "attack_damage" => "attribute.name.generic.attackDamage".to_owned(),
        "jump_strength" => "attribute.name.horse.jumpStrength".to_owned(),
        "spawn_reinforcements" => "attribute.name.zombie.spawnReinforcements".to_owned(),
        "attack_speed" => "attribute.name.generic.attackSpeed".to_owned(),
        "armor" => "attribute.name.generic.armor".to_owned(),
        "armor_toughness" => "attribute.name.generic.armorToughness".to_owned(),
        "luck" => "attribute.name.generic.luck".to_owned(),
        "flying_speed" => "attribute.name.generic.flyingSpeed".to_owned(),
        _ => format!("attribute.name.{short}"),
    }
}

/// The modifier id the attribute-modifier-id fix gives a 1.8 UUID: two of them
/// have names, and any other becomes `minecraft:<lowercase uuid>`, the form
/// `Modifier::new` accepts.
fn uuid_text(tag: &Tag) -> Option<String> {
    let ints = match tag {
        Tag::IntArray(ints) => ints.clone(),
        _ => return None,
    };
    if ints.len() != 4 {
        return None;
    }
    let most = (u64::from(ints[0] as u32) << 32) | u64::from(ints[1] as u32);
    let least = (u64::from(ints[2] as u32) << 32) | u64::from(ints[3] as u32);
    // 1.8 drops the modifier when either half is zero.
    if most == 0 || least == 0 {
        return None;
    }
    let uuid = format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        most >> 32,
        (most >> 16) & 0xffff,
        most & 0xffff,
        least >> 48,
        least & 0xffff_ffff_ffff
    );
    Some(match uuid.as_str() {
        ATTACK_DAMAGE_UUID => "minecraft:base_attack_damage".to_owned(),
        ATTACK_SPEED_UUID => "minecraft:base_attack_speed".to_owned(),
        _ => format!("minecraft:{uuid}"),
    })
}

fn as_str(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(s) => Some(s.to_string_lossy()),
        _ => None,
    }
}

fn as_int(tag: &Tag) -> Option<i32> {
    match tag {
        Tag::Byte(v) => Some(i32::from(*v)),
        Tag::Short(v) => Some(i32::from(*v)),
        Tag::Int(v) => Some(*v),
        Tag::Long(v) => Some(*v as i32),
        _ => None,
    }
}

fn as_double(tag: &Tag) -> Option<f64> {
    match tag {
        Tag::Double(bits) => Some(f64::from_bits(*bits)),
        Tag::Float(bits) => Some(f64::from(f32::from_bits(*bits))),
        tag => as_int(tag).map(f64::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_add_their_material_damage() {
        // The twenty attack-damage lines of the real 1.8.9 client's tooltips.
        assert_eq!(attack_damage("minecraft:wooden_sword"), Some(4.0));
        assert_eq!(attack_damage("minecraft:golden_sword"), Some(4.0));
        assert_eq!(attack_damage("minecraft:stone_sword"), Some(5.0));
        assert_eq!(attack_damage("minecraft:iron_sword"), Some(6.0));
        assert_eq!(attack_damage("minecraft:diamond_sword"), Some(7.0));
        assert_eq!(attack_damage("minecraft:wooden_axe"), Some(3.0));
        assert_eq!(attack_damage("minecraft:diamond_axe"), Some(6.0));
        assert_eq!(attack_damage("minecraft:stone_pickaxe"), Some(3.0));
        assert_eq!(attack_damage("minecraft:diamond_pickaxe"), Some(5.0));
        assert_eq!(attack_damage("minecraft:golden_shovel"), Some(1.0));
        assert_eq!(attack_damage("minecraft:diamond_shovel"), Some(4.0));
        // 1.8.9's hoes carry no modifier at all.
        assert_eq!(attack_damage("minecraft:diamond_hoe"), None);
        assert_eq!(attack_damage("minecraft:bow"), None);
        // A mod's own item keeps its component modifiers.
        assert_eq!(attack_damage("example:ruby_sword"), None);
    }

    #[test]
    fn enchantments_add_their_group_damage() {
        let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
        assert_eq!(enchantment_bonus(&sword, false, false), 0.0);
        sword.enchant("minecraft:sharpness", 5);
        assert_eq!(enchantment_bonus(&sword, false, false), 6.25);
        sword.enchant("minecraft:smite", 4);
        // Smite only counts against the undead.
        assert_eq!(enchantment_bonus(&sword, false, false), 6.25);
        assert_eq!(enchantment_bonus(&sword, true, false), 16.25);
        sword.enchant("minecraft:bane_of_arthropods", 2);
        assert_eq!(enchantment_bonus(&sword, false, true), 11.25);
    }

    fn with_modifiers(modifiers: Vec<Tag>) -> ItemStack {
        let mut stack = ItemStack::new("minecraft:diamond_sword", 1);
        let mut data = crate::item_data::StackData::default();
        data.other.push((
            "minecraft:attribute_modifiers".to_owned(),
            Tag::List(modifiers),
        ));
        stack.extra = Some(Box::new(data));
        stack
    }

    fn modifier(
        attribute: &str,
        amount: f64,
        operation: &str,
        id: Option<&str>,
        uuid: Option<[i32; 4]>,
    ) -> Tag {
        let mut entry = vec![
            (
                "type",
                Tag::String(horizon_world::nbt::JavaString::from(attribute)),
            ),
            ("amount", Tag::Double(amount.to_bits())),
            (
                "operation",
                Tag::String(horizon_world::nbt::JavaString::from(operation)),
            ),
        ];
        if let Some(id) = id {
            entry.push(("id", Tag::String(horizon_world::nbt::JavaString::from(id))));
        }
        if let Some(uuid) = uuid {
            entry.push(("uuid", Tag::IntArray(uuid.to_vec())));
        }
        Tag::compound(entry)
    }

    #[test]
    fn a_stacks_own_modifiers_replace_the_items_defaults() {
        let stack = with_modifiers(vec![modifier(
            "minecraft:generic.max_health",
            4.0,
            "add_value",
            Some("minecraft:test"),
            None,
        )]);
        assert_eq!(
            stack_modifiers(&stack),
            Some(vec![(
                "minecraft:max_health".to_owned(),
                "minecraft:test".to_owned(),
                4.0,
                "add_value".to_owned(),
                "any".to_owned(),
            )]),
            "the sword's own +7 is replaced, not extended"
        );
        // The component with no entry is a real answer: nothing applies.
        assert_eq!(
            stack_modifiers(&with_modifiers(Vec::new())),
            Some(Vec::new())
        );
        // Without the component the item's defaults are the answer.
        assert_eq!(
            stack_modifiers(&ItemStack::new("minecraft:diamond_sword", 1)),
            None
        );
        assert_eq!(
            attack_modifier("minecraft:diamond_sword")[0].1,
            "minecraft:base_attack_damage"
        );
    }

    #[test]
    fn legacy_uuids_become_named_modifier_ids() {
        // 1.8's Attack Damage UUID: `cb3f55d3-645c-4f38-a497-9c13a33db5cf`.
        let attack = [
            0xcb3f55d3u32 as i32,
            0x645c4f38,
            0xa4979c13u32 as i32,
            0xa33db5cfu32 as i32,
        ];
        let stack = with_modifiers(vec![modifier(
            "generic.attackDamage",
            2.0,
            "add_value",
            None,
            Some(attack),
        )]);
        assert_eq!(
            stack_modifiers(&stack).unwrap()[0].1,
            "minecraft:base_attack_damage",
            "the datafix's name for Item.ATTACK_DAMAGE_MODIFIER_UUID"
        );
        // A UUID with a zero half is dropped by ItemStack.getAttributes.
        let zeroed = with_modifiers(vec![modifier(
            "generic.attackDamage",
            2.0,
            "add_value",
            None,
            Some([0, 0, 1, 2]),
        )]);
        assert_eq!(stack_modifiers(&zeroed), Some(Vec::new()));
        // An operation outside 0..2 is dropped by EntityAttributes.fromNbt.
        let bad_op = with_modifiers(vec![modifier(
            "generic.attackDamage",
            2.0,
            "bogus",
            Some("minecraft:test"),
            None,
        )]);
        assert_eq!(stack_modifiers(&bad_op), Some(Vec::new()));
    }

    #[test]
    fn attribute_names_normalise_and_keep_their_1_8_keys() {
        for (stored, engine) in [
            ("generic.attackDamage", "minecraft:attack_damage"),
            ("minecraft:generic.attack_damage", "minecraft:attack_damage"),
            ("minecraft:attack_damage", "minecraft:attack_damage"),
            ("horse.jumpStrength", "minecraft:jump_strength"),
            (
                "zombie.spawnReinforcements",
                "minecraft:spawn_reinforcements",
            ),
        ] {
            assert_eq!(attribute_1_8_9(stored), engine);
        }
        assert_eq!(
            attribute_display_key("minecraft:attack_damage"),
            "attribute.name.generic.attackDamage"
        );
        // 1.8's language file has no key for the attributes it did not have.
        assert_eq!(
            attribute_display_key("minecraft:armor"),
            "attribute.name.generic.armor"
        );
    }
}
