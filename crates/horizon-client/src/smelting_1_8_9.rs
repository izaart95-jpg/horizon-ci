//! 1.8's `SmeltingRecipeRegistry`: one 200-tick recipe per input, with the
//! experience its product pays when a player takes it.
//!
//! The table is the constructor of the 1.8 jar, in its order, with the
//! flattened item names a 1.8 stack decodes to. `addBlock`/`addItem` match
//! every damage of the input, so the metadata variants that are separate
//! items now each get their own row.
use crate::{
    crafting::{Cooking, Ingredient},
    items::ItemStack,
};
use std::sync::OnceLock;

/// `(input, product, experience)` of `SmeltingRecipeRegistry`'s constructor.
const RECIPES_1_8_9: &[(&str, &str, f32)] = &[
    ("minecraft:iron_ore", "minecraft:iron_ingot", 0.7),
    ("minecraft:gold_ore", "minecraft:gold_ingot", 1.0),
    ("minecraft:diamond_ore", "minecraft:diamond", 1.0),
    // `Blocks.SAND` covers red sand too.
    ("minecraft:sand", "minecraft:glass", 0.1),
    ("minecraft:red_sand", "minecraft:glass", 0.1),
    ("minecraft:porkchop", "minecraft:cooked_porkchop", 0.35),
    ("minecraft:beef", "minecraft:cooked_beef", 0.35),
    ("minecraft:chicken", "minecraft:cooked_chicken", 0.35),
    ("minecraft:rabbit", "minecraft:cooked_rabbit", 0.35),
    ("minecraft:mutton", "minecraft:cooked_mutton", 0.35),
    ("minecraft:cobblestone", "minecraft:stone", 0.1),
    (
        "minecraft:stone_bricks",
        "minecraft:cracked_stone_bricks",
        0.1,
    ),
    ("minecraft:clay_ball", "minecraft:brick", 0.3),
    ("minecraft:clay", "minecraft:terracotta", 0.35),
    ("minecraft:cactus", "minecraft:green_dye", 0.2),
    // `Blocks.LOG`/`LOG2` cover every species and the bark-sided variants.
    ("minecraft:oak_log", "minecraft:charcoal", 0.15),
    ("minecraft:spruce_log", "minecraft:charcoal", 0.15),
    ("minecraft:birch_log", "minecraft:charcoal", 0.15),
    ("minecraft:jungle_log", "minecraft:charcoal", 0.15),
    ("minecraft:acacia_log", "minecraft:charcoal", 0.15),
    ("minecraft:dark_oak_log", "minecraft:charcoal", 0.15),
    ("minecraft:oak_wood", "minecraft:charcoal", 0.15),
    ("minecraft:spruce_wood", "minecraft:charcoal", 0.15),
    ("minecraft:birch_wood", "minecraft:charcoal", 0.15),
    ("minecraft:jungle_wood", "minecraft:charcoal", 0.15),
    ("minecraft:acacia_wood", "minecraft:charcoal", 0.15),
    ("minecraft:dark_oak_wood", "minecraft:charcoal", 0.15),
    ("minecraft:emerald_ore", "minecraft:emerald", 1.0),
    ("minecraft:potato", "minecraft:baked_potato", 0.35),
    ("minecraft:netherrack", "minecraft:nether_brick", 0.1),
    ("minecraft:wet_sponge", "minecraft:sponge", 0.15),
    // `FishItem.FishType` 0 and 1 are the cookable ones.
    ("minecraft:cod", "minecraft:cooked_cod", 0.35),
    ("minecraft:salmon", "minecraft:cooked_salmon", 0.35),
    ("minecraft:coal_ore", "minecraft:coal", 0.1),
    ("minecraft:redstone_ore", "minecraft:redstone", 0.7),
    ("minecraft:lapis_ore", "minecraft:lapis_lazuli", 0.2),
    ("minecraft:nether_quartz_ore", "minecraft:quartz", 0.2),
];

/// The table as furnace recipes; the id names the product, so every input of
/// the same product shares one id and one experience.
fn recipes() -> &'static [Cooking] {
    static RECIPES: OnceLock<Vec<Cooking>> = OnceLock::new();
    RECIPES.get_or_init(|| {
        RECIPES_1_8_9
            .iter()
            .map(|&(input, product, experience)| Cooking {
                id: format!(
                    "1_8_9_smelting/{}",
                    product.strip_prefix("minecraft:").unwrap_or(product)
                ),
                category: "misc".to_owned(),
                group: String::new(),
                kind: "minecraft:smelting".to_owned(),
                ingredient: Ingredient::of(vec![input.to_owned()]),
                result: ItemStack::new(product, 1),
                experience,
                time: 200,
            })
            .collect()
    })
}

/// `SmeltingRecipeRegistry.getResult`, for a furnace's input.
pub fn cook(input: &ItemStack) -> Option<&'static Cooking> {
    recipes().iter().find(|c| c.ingredient.test(input))
}

/// The products of the 1.8 table, for `Stats.CRAFTING_STATS`.
pub fn products() -> impl Iterator<Item = &'static str> {
    RECIPES_1_8_9.iter().map(|&(_, product, _)| product)
}

/// `SmeltingRecipeRegistry.getXp` for the product a recipe id names. The id
/// may carry the `minecraft:` namespace a furnace writes (`recipe_used_key`).
pub fn experience(id: &str) -> f32 {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    recipes()
        .iter()
        .find(|c| c.id == id)
        .map_or(0.0, |c| c.experience)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> ItemStack {
        ItemStack::new(name, 1)
    }

    /// Every row of the 1.8 constructor, and a few non-recipes.
    #[test]
    fn one_eight_smelting_is_the_registry_table() {
        for (item, product, xp) in [
            ("minecraft:iron_ore", "minecraft:iron_ingot", 0.7),
            ("minecraft:gold_ore", "minecraft:gold_ingot", 1.0),
            ("minecraft:diamond_ore", "minecraft:diamond", 1.0),
            ("minecraft:sand", "minecraft:glass", 0.1),
            ("minecraft:red_sand", "minecraft:glass", 0.1),
            ("minecraft:porkchop", "minecraft:cooked_porkchop", 0.35),
            ("minecraft:beef", "minecraft:cooked_beef", 0.35),
            ("minecraft:chicken", "minecraft:cooked_chicken", 0.35),
            ("minecraft:rabbit", "minecraft:cooked_rabbit", 0.35),
            ("minecraft:mutton", "minecraft:cooked_mutton", 0.35),
            ("minecraft:cobblestone", "minecraft:stone", 0.1),
            (
                "minecraft:stone_bricks",
                "minecraft:cracked_stone_bricks",
                0.1,
            ),
            ("minecraft:clay_ball", "minecraft:brick", 0.3),
            ("minecraft:clay", "minecraft:terracotta", 0.35),
            ("minecraft:cactus", "minecraft:green_dye", 0.2),
            ("minecraft:oak_log", "minecraft:charcoal", 0.15),
            ("minecraft:dark_oak_wood", "minecraft:charcoal", 0.15),
            ("minecraft:emerald_ore", "minecraft:emerald", 1.0),
            ("minecraft:potato", "minecraft:baked_potato", 0.35),
            ("minecraft:netherrack", "minecraft:nether_brick", 0.1),
            ("minecraft:wet_sponge", "minecraft:sponge", 0.15),
            ("minecraft:cod", "minecraft:cooked_cod", 0.35),
            ("minecraft:salmon", "minecraft:cooked_salmon", 0.35),
            ("minecraft:coal_ore", "minecraft:coal", 0.1),
            ("minecraft:redstone_ore", "minecraft:redstone", 0.7),
            ("minecraft:lapis_ore", "minecraft:lapis_lazuli", 0.2),
            ("minecraft:nether_quartz_ore", "minecraft:quartz", 0.2),
        ] {
            let recipe = cook(&input(item)).unwrap_or_else(|| panic!("{item} smelts"));
            assert_eq!(&*recipe.result.item, product, "{item}");
            assert_eq!(recipe.result.count, 1, "{item}");
            assert_eq!(recipe.time, 200, "{item}");
            assert_eq!(recipe.experience, xp, "{item}");
            assert_eq!(experience(&recipe.id), xp, "{item}");
            // The furnace stores the id namespaced.
            assert_eq!(
                experience(&crate::containers::recipe_used_key(&recipe.id)),
                xp,
                "{item}"
            );
        }
        // 1.8 does not smelt these (stone bricks only crack from the default
        // variant, and the other stone brick variants have no recipe).
        for item in [
            "minecraft:stone",
            "minecraft:mossy_stone_bricks",
            "minecraft:chiseled_stone_bricks",
            "minecraft:oak_planks",
            "minecraft:clownfish",
            "minecraft:pufferfish",
            "minecraft:charcoal",
        ] {
            assert!(cook(&input(item)).is_none(), "{item}");
        }
    }

    /// A product's experience is paid per item taken, not per input.
    #[test]
    fn one_eight_smelting_experience_is_per_product() {
        let logs = ["minecraft:oak_log", "minecraft:acacia_wood"];
        let ids: Vec<String> = logs
            .iter()
            .map(|item| cook(&input(item)).unwrap().id.clone())
            .collect();
        assert_eq!(ids[0], ids[1]);
        assert_eq!(experience(&ids[0]), 0.15);
        assert_eq!(experience("1_8_9_smelting/not_a_product"), 0.0);
    }
}
