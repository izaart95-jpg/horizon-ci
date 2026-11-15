//! 1.8.9's melee damage: the attack damage a weapon adds and the bonus
//! damage Sharpness, Smite and Bane of Arthropods add.
//!
//! 1.8 has no attack-speed attribute and no base-attack-damage modifier: a
//! sword or tool installs one `generic.attackDamage` modifier in the main
//! hand, and `EntityPlayer.attack` adds the enchantment damage on top of the
//! player's own 1.0 base.

use crate::items::ItemStack;

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

/// The main-hand modifier `apply_equipment` installs, in the shape the
/// 26.3 components use: (attribute, id, amount, operation, slot group). 1.8's
/// `ATTACK_DAMAGE_MODIFIER_UUID` is the id, so a re-equip replaces it.
pub fn attack_modifier(item: &str) -> Vec<(String, String, f64, String, String)> {
    attack_damage(item)
        .map(|amount| {
            vec![(
                "minecraft:attack_damage".to_owned(),
                "CB3F55D3-645C-4F38-A497-9C13A33DB5CF".to_owned(),
                amount,
                "add_value".to_owned(),
                "mainhand".to_owned(),
            )]
        })
        .unwrap_or_default()
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
}
