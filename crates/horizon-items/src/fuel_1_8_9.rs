//! `FurnaceBlockEntity.getBurnTime` of 1.8: fuel is a rule over the item and
//! its block's `Material.WOOD`, not the current data pack's `fuel` field.
//!
//! The names are the flattened ones a 1.8 stack decodes to, so every metadata
//! variant of a wooden block is its own item here.

/// `FurnaceBlockEntity.getBurnTime(stack)`.
pub fn burn_time(item: &str) -> i32 {
    // `block == Blocks.WOODEN_SLAB` is checked before the material.
    if matches!(
        item,
        "minecraft:oak_slab"
            | "minecraft:spruce_slab"
            | "minecraft:birch_slab"
            | "minecraft:jungle_slab"
            | "minecraft:acacia_slab"
            | "minecraft:dark_oak_slab"
    ) {
        return 150;
    }
    // Every 1.8 block whose `Material` is `Material.WOOD` (`Block.getMaterial()
    // == Material.WOOD` in the 1.8 registry), by its flattened item name.
    if matches!(
        item,
        // Planks, logs and bark-sided logs.
        "minecraft:oak_planks"
            | "minecraft:spruce_planks"
            | "minecraft:birch_planks"
            | "minecraft:jungle_planks"
            | "minecraft:acacia_planks"
            | "minecraft:dark_oak_planks"
            | "minecraft:oak_log"
            | "minecraft:spruce_log"
            | "minecraft:birch_log"
            | "minecraft:jungle_log"
            | "minecraft:acacia_log"
            | "minecraft:dark_oak_log"
            | "minecraft:oak_wood"
            | "minecraft:spruce_wood"
            | "minecraft:birch_wood"
            | "minecraft:jungle_wood"
            | "minecraft:acacia_wood"
            | "minecraft:dark_oak_wood"
            // Stairs, slabs (the single slab is the 150 above), fences, gates,
            // doors, trapdoors, pressure plates and buttons.
            | "minecraft:oak_stairs"
            | "minecraft:spruce_stairs"
            | "minecraft:birch_stairs"
            | "minecraft:jungle_stairs"
            | "minecraft:acacia_stairs"
            | "minecraft:dark_oak_stairs"
            | "minecraft:oak_fence"
            | "minecraft:spruce_fence"
            | "minecraft:birch_fence"
            | "minecraft:jungle_fence"
            | "minecraft:acacia_fence"
            | "minecraft:dark_oak_fence"
            | "minecraft:oak_fence_gate"
            | "minecraft:spruce_fence_gate"
            | "minecraft:birch_fence_gate"
            | "minecraft:jungle_fence_gate"
            | "minecraft:acacia_fence_gate"
            | "minecraft:dark_oak_fence_gate"
            | "minecraft:oak_trapdoor"
            | "minecraft:oak_pressure_plate"
            // The blocks that are wood without being made of planks.
            | "minecraft:crafting_table"
            | "minecraft:chest"
            | "minecraft:trapped_chest"
            | "minecraft:bookshelf"
            | "minecraft:jukebox"
            | "minecraft:note_block"
            | "minecraft:daylight_detector"
            | "minecraft:brown_mushroom_block"
            | "minecraft:red_mushroom_block"
            | "minecraft:mushroom_stem"
            // Banners are BlockItems over the wooden standing banner.
            | "minecraft:white_banner"
            | "minecraft:orange_banner"
            | "minecraft:magenta_banner"
            | "minecraft:light_blue_banner"
            | "minecraft:yellow_banner"
            | "minecraft:lime_banner"
            | "minecraft:pink_banner"
            | "minecraft:gray_banner"
            | "minecraft:light_gray_banner"
            | "minecraft:cyan_banner"
            | "minecraft:purple_banner"
            | "minecraft:blue_banner"
            | "minecraft:brown_banner"
            | "minecraft:green_banner"
            | "minecraft:red_banner"
            | "minecraft:black_banner"
    ) {
        return 300;
    }
    if item == "minecraft:coal_block" {
        return 16000;
    }
    if matches!(
        item,
        "minecraft:wooden_sword"
            | "minecraft:wooden_pickaxe"
            | "minecraft:wooden_axe"
            | "minecraft:wooden_shovel"
            | "minecraft:wooden_hoe"
    ) {
        return 200;
    }
    match item {
        "minecraft:stick" => 100,
        "minecraft:coal" | "minecraft:charcoal" => 1600,
        "minecraft:lava_bucket" => 20000,
        "minecraft:oak_sapling"
        | "minecraft:spruce_sapling"
        | "minecraft:birch_sapling"
        | "minecraft:jungle_sapling"
        | "minecraft:acacia_sapling"
        | "minecraft:dark_oak_sapling" => 100,
        "minecraft:blaze_rod" => 2400,
        _ => 0,
    }
}

/// `FurnaceBlockEntity.isFuel`.
pub fn is_fuel(item: &str) -> bool {
    burn_time(item) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `FurnaceBlockEntity.getBurnTime` of the 1.8 jar.
    #[test]
    fn one_eight_fuel_follows_the_burn_time_rules() {
        for (item, ticks) in [
            ("minecraft:oak_slab", 150),
            ("minecraft:oak_planks", 300),
            ("minecraft:dark_oak_log", 300),
            ("minecraft:oak_wood", 300),
            ("minecraft:oak_stairs", 300),
            ("minecraft:oak_fence_gate", 300),
            ("minecraft:oak_trapdoor", 300),
            ("minecraft:oak_pressure_plate", 300),
            ("minecraft:crafting_table", 300),
            ("minecraft:chest", 300),
            ("minecraft:trapped_chest", 300),
            ("minecraft:bookshelf", 300),
            ("minecraft:jukebox", 300),
            ("minecraft:note_block", 300),
            ("minecraft:daylight_detector", 300),
            ("minecraft:brown_mushroom_block", 300),
            ("minecraft:red_mushroom_block", 300),
            ("minecraft:mushroom_stem", 300),
            ("minecraft:white_banner", 300),
            ("minecraft:black_banner", 300),
            ("minecraft:coal_block", 16000),
            ("minecraft:wooden_pickaxe", 200),
            ("minecraft:wooden_sword", 200),
            ("minecraft:wooden_hoe", 200),
            ("minecraft:stick", 100),
            ("minecraft:coal", 1600),
            ("minecraft:charcoal", 1600),
            ("minecraft:lava_bucket", 20000),
            ("minecraft:oak_sapling", 100),
            ("minecraft:dark_oak_sapling", 100),
            ("minecraft:blaze_rod", 2400),
            // Not fuels in 1.8.
            ("minecraft:stone", 0),
            ("minecraft:bucket", 0),
            ("minecraft:oak_sign", 0),
            ("minecraft:ladder", 0),
            ("minecraft:oak_boat", 0),
            ("minecraft:iron_pickaxe", 0),
            ("minecraft:ender_chest", 0),
            ("minecraft:wooden_button", 0),
            ("minecraft:oak_button", 0),
            ("minecraft:oak_door", 0),
            ("minecraft:spruce_door", 0),
        ] {
            assert_eq!(burn_time(item), ticks, "{item}");
        }
    }

    /// `FurnaceFuelSlot.canInsert` uses `isFuel`, and the bucket that dries a
    /// wet sponge is the only non-fuel the slot takes.
    #[test]
    fn one_eight_fuel_slot_takes_fuel_and_buckets() {
        assert!(is_fuel("minecraft:coal"));
        assert!(is_fuel("minecraft:oak_slab"));
        assert!(!is_fuel("minecraft:bucket"));
        assert!(!is_fuel("minecraft:water_bucket"));
        assert!(!is_fuel("minecraft:oak_sign"));
    }
}
