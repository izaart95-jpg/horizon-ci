//! 1.8.9 sound names (`S29PacketSoundEffect`) and the modern event ids the
//! engine speaks, plus the 1.8 sample event a modern id falls back to.
//!
//! 1.8 sends flat, dot-separated names such as `random.explode` or
//! `mob.zombie.say`; 1.9 renamed the set when it moved sound events into
//! `sounds.json`. The client translates an incoming 1.8 name to the modern id
//! the engine speaks ([`resolve`]); the sound registry maps the modern ids the
//! engine plays back to the 1.8 event with the same samples ([`legacy`]).
//!
//! [`legacy`] also covers the sound groups 26.3 gives individual blocks
//! (`block.metal.break`, `block.crop.break`) because 1.8 had eleven shared
//! `SoundType`s, and the button, door, chest and piston events 1.8 played with
//! one of its generic samples.

/// A real 1.8 sound event and the modern event id it became. The first entry
/// for a modern id also names the 1.8 event its samples come from.
const PAIRS: &[(&str, &str)] = &[
    ("random.splash", "entity.generic.splash"),
    ("ambient.weather.rain", "weather.rain"),
    ("ambient.weather.thunder", "entity.lightning_bolt.thunder"),
    ("dig.cloth", "block.wool.break"),
    ("dig.glass", "block.glass.break"),
    ("dig.grass", "block.grass.break"),
    ("dig.gravel", "block.gravel.break"),
    ("dig.sand", "block.sand.break"),
    ("dig.snow", "block.snow.break"),
    ("dig.stone", "block.stone.break"),
    ("dig.wood", "block.wood.break"),
    ("step.cloth", "block.wool.step"),
    ("step.grass", "block.grass.step"),
    ("step.gravel", "block.gravel.step"),
    ("step.ladder", "block.ladder.step"),
    ("step.sand", "block.sand.step"),
    ("step.snow", "block.snow.step"),
    ("step.stone", "block.stone.step"),
    ("step.wood", "block.wood.step"),
    ("fire.fire", "block.fire.ambient"),
    ("fire.ignite", "item.flintandsteel.use"),
    ("fireworks.blast", "entity.firework_rocket.blast"),
    ("fireworks.blast_far", "entity.firework_rocket.blast_far"),
    ("fireworks.largeBlast", "entity.firework_rocket.large_blast"),
    (
        "fireworks.largeBlast_far",
        "entity.firework_rocket.large_blast_far",
    ),
    ("fireworks.launch", "entity.firework_rocket.launch"),
    ("fireworks.twinkle", "entity.firework_rocket.twinkle"),
    (
        "fireworks.twinkle_far",
        "entity.firework_rocket.twinkle_far",
    ),
    ("liquid.lava", "block.lava.ambient"),
    ("liquid.lavapop", "block.lava.pop"),
    ("liquid.water", "block.water.ambient"),
    ("game.hostile.hurt", "entity.generic.hurt"),
    ("game.hostile.swim", "entity.generic.swim"),
    ("game.hostile.swim.splash", "entity.generic.splash"),
    ("game.neutral.hurt", "entity.generic.hurt"),
    ("game.neutral.swim", "entity.generic.swim"),
    ("game.neutral.swim.splash", "entity.generic.splash"),
    ("game.player.hurt", "entity.player.hurt"),
    ("game.potion.smash", "entity.splash_potion.break"),
    ("game.tnt.primed", "entity.tnt.primed"),
    ("mob.bat.death", "entity.bat.death"),
    ("mob.bat.hurt", "entity.bat.hurt"),
    ("mob.bat.idle", "entity.bat.ambient"),
    ("mob.bat.loop", "entity.bat.loop"),
    ("mob.bat.takeoff", "entity.bat.takeoff"),
    ("mob.blaze.breathe", "entity.blaze.burn"),
    ("mob.blaze.death", "entity.blaze.death"),
    ("mob.blaze.hit", "entity.blaze.hurt"),
    ("mob.cat.hiss", "entity.cat.hiss"),
    ("mob.cat.hitt", "entity.cat.hurt"),
    ("mob.cat.meow", "entity.cat.ambient"),
    ("mob.cat.purr", "entity.cat.purr"),
    ("mob.cat.purreow", "entity.cat.purreow"),
    ("mob.chicken.hurt", "entity.chicken.hurt"),
    ("mob.chicken.plop", "entity.chicken.egg"),
    ("mob.chicken.say", "entity.chicken.ambient"),
    ("mob.chicken.step", "entity.chicken.step"),
    ("mob.cow.hurt", "entity.cow.hurt"),
    ("mob.cow.say", "entity.cow.ambient"),
    ("mob.cow.step", "entity.cow.step"),
    ("mob.creeper.death", "entity.creeper.death"),
    ("mob.creeper.say", "entity.creeper.primed"),
    ("mob.enderdragon.end", "entity.ender_dragon.death"),
    ("mob.enderdragon.growl", "entity.ender_dragon.growl"),
    ("mob.enderdragon.hit", "entity.ender_dragon.hurt"),
    ("mob.enderdragon.wings", "entity.ender_dragon.flap"),
    ("mob.endermen.death", "entity.enderman.death"),
    ("mob.endermen.hit", "entity.enderman.hurt"),
    ("mob.endermen.idle", "entity.enderman.ambient"),
    ("mob.endermen.portal", "entity.enderman.teleport"),
    ("mob.endermen.scream", "entity.enderman.scream"),
    ("mob.endermen.stare", "entity.enderman.stare"),
    ("mob.ghast.affectionate_scream", "entity.ghast.scream"),
    ("mob.ghast.charge", "entity.ghast.warn"),
    ("mob.ghast.death", "entity.ghast.death"),
    ("mob.ghast.fireball", "entity.ghast.shoot"),
    ("mob.ghast.moan", "entity.ghast.ambient"),
    ("mob.ghast.scream", "entity.ghast.scream"),
    ("mob.guardian.curse", "entity.elder_guardian.curse"),
    ("mob.guardian.death", "entity.guardian.death"),
    ("mob.guardian.hit", "entity.guardian.hurt"),
    ("mob.guardian.idle", "entity.guardian.ambient"),
    ("mob.horse.angry", "entity.horse.angry"),
    ("mob.horse.armor", "entity.horse.armor"),
    ("mob.horse.breathe", "entity.horse.breathe"),
    ("mob.horse.death", "entity.horse.death"),
    ("mob.horse.donkey.angry", "entity.donkey.angry"),
    ("mob.horse.donkey.death", "entity.donkey.death"),
    ("mob.horse.donkey.hit", "entity.donkey.hurt"),
    ("mob.horse.donkey.idle", "entity.donkey.ambient"),
    ("mob.horse.gallop", "entity.horse.gallop"),
    ("mob.horse.hit", "entity.horse.hurt"),
    ("mob.horse.idle", "entity.horse.ambient"),
    ("mob.horse.jump", "entity.horse.jump"),
    ("mob.horse.land", "entity.horse.land"),
    ("mob.horse.leather", "entity.horse.saddle"),
    ("mob.horse.skeleton.death", "entity.skeleton_horse.death"),
    ("mob.horse.skeleton.hit", "entity.skeleton_horse.hurt"),
    ("mob.horse.skeleton.idle", "entity.skeleton_horse.ambient"),
    ("mob.horse.soft", "entity.horse.step"),
    ("mob.horse.wood", "entity.horse.step_wood"),
    ("mob.horse.zombie.death", "entity.zombie_horse.death"),
    ("mob.horse.zombie.hit", "entity.zombie_horse.hurt"),
    ("mob.horse.zombie.idle", "entity.zombie_horse.ambient"),
    ("mob.irongolem.death", "entity.iron_golem.death"),
    ("mob.irongolem.hit", "entity.iron_golem.hurt"),
    ("mob.irongolem.throw", "entity.iron_golem.attack"),
    ("mob.irongolem.walk", "entity.iron_golem.step"),
    ("mob.magmacube.big", "entity.magma_cube.squish"),
    ("mob.magmacube.jump", "entity.magma_cube.jump"),
    ("mob.magmacube.small", "entity.magma_cube.squish_small"),
    ("mob.pig.death", "entity.pig.death"),
    ("mob.pig.say", "entity.pig.ambient"),
    ("mob.pig.step", "entity.pig.step"),
    ("mob.rabbit.death", "entity.rabbit.death"),
    ("mob.rabbit.hop", "entity.rabbit.jump"),
    ("mob.rabbit.hurt", "entity.rabbit.hurt"),
    ("mob.rabbit.idle", "entity.rabbit.ambient"),
    ("mob.sheep.say", "entity.sheep.ambient"),
    ("mob.sheep.shear", "entity.sheep.shear"),
    ("mob.sheep.step", "entity.sheep.step"),
    ("mob.silverfish.hit", "entity.silverfish.hurt"),
    ("mob.silverfish.kill", "entity.silverfish.death"),
    ("mob.silverfish.say", "entity.silverfish.ambient"),
    ("mob.silverfish.step", "entity.silverfish.step"),
    ("mob.skeleton.death", "entity.skeleton.death"),
    ("mob.skeleton.hurt", "entity.skeleton.hurt"),
    ("mob.skeleton.say", "entity.skeleton.ambient"),
    ("mob.skeleton.step", "entity.skeleton.step"),
    ("mob.slime.attack", "entity.slime.attack"),
    ("mob.slime.big", "entity.slime.squish"),
    ("mob.slime.small", "entity.slime.squish_small"),
    ("mob.spider.death", "entity.spider.death"),
    ("mob.spider.say", "entity.spider.ambient"),
    ("mob.spider.step", "entity.spider.step"),
    ("mob.villager.death", "entity.villager.death"),
    ("mob.villager.haggle", "entity.villager.trade"),
    ("mob.villager.hit", "entity.villager.hurt"),
    ("mob.villager.idle", "entity.villager.ambient"),
    ("mob.villager.no", "entity.villager.no"),
    ("mob.villager.yes", "entity.villager.yes"),
    ("mob.wither.death", "entity.wither.death"),
    ("mob.wither.hurt", "entity.wither.hurt"),
    ("mob.wither.idle", "entity.wither.ambient"),
    ("mob.wither.shoot", "entity.wither.shoot"),
    ("mob.wither.spawn", "entity.wither.spawn"),
    ("mob.wolf.bark", "entity.wolf.ambient"),
    ("mob.wolf.death", "entity.wolf.death"),
    ("mob.wolf.growl", "entity.wolf.growl"),
    ("mob.wolf.hurt", "entity.wolf.hurt"),
    ("mob.wolf.panting", "entity.wolf.pant"),
    ("mob.wolf.shake", "entity.wolf.shake"),
    ("mob.wolf.whine", "entity.wolf.whine"),
    ("mob.zombie.death", "entity.zombie.death"),
    ("mob.zombie.hurt", "entity.zombie.hurt"),
    ("mob.zombie.infect", "entity.zombie.infect"),
    ("mob.zombie.metal", "entity.zombie.attack_iron_door"),
    ("mob.zombie.remedy", "entity.zombie_villager.cure"),
    ("mob.zombie.say", "entity.zombie.ambient"),
    ("mob.zombie.step", "entity.zombie.step"),
    ("mob.zombie.unfect", "entity.zombie_villager.converted"),
    ("mob.zombie.wood", "entity.zombie.attack_wooden_door"),
    ("mob.zombie.woodbreak", "entity.zombie.break_wooden_door"),
    ("mob.zombiepig.zpig", "entity.zombified_piglin.ambient"),
    ("mob.zombiepig.zpigangry", "entity.zombified_piglin.angry"),
    ("mob.zombiepig.zpigdeath", "entity.zombified_piglin.death"),
    ("mob.zombiepig.zpighurt", "entity.zombified_piglin.hurt"),
    ("note.bass", "block.note_block.bass"),
    ("note.bassattack", "block.note_block.bass"),
    ("note.bd", "block.note_block.basedrum"),
    ("note.harp", "block.note_block.harp"),
    ("note.hat", "block.note_block.hat"),
    ("note.pling", "block.note_block.pling"),
    ("note.snare", "block.note_block.snare"),
    ("portal.portal", "block.portal.ambient"),
    ("portal.travel", "block.portal.travel"),
    ("portal.trigger", "block.portal.trigger"),
    ("random.anvil_break", "block.anvil.destroy"),
    ("random.anvil_land", "block.anvil.land"),
    ("random.anvil_use", "block.anvil.use"),
    ("random.bow", "entity.arrow.shoot"),
    ("random.bowhit", "entity.arrow.hit"),
    ("random.break", "entity.item.break"),
    ("random.burp", "entity.player.burp"),
    ("random.chestclosed", "block.chest.close"),
    ("random.chestopen", "block.chest.open"),
    ("random.door_close", "block.wooden_door.close"),
    ("random.door_open", "block.wooden_door.open"),
    ("random.drink", "entity.generic.drink"),
    ("random.eat", "entity.generic.eat"),
    ("random.explode", "entity.generic.explode"),
    ("random.fizz", "block.fire.extinguish"),
    ("random.levelup", "entity.player.levelup"),
    ("random.orb", "entity.experience_orb.pickup"),
    ("random.pop", "entity.item.pickup"),
    ("random.successful_hit", "entity.arrow.hit_player"),
    ("random.wood_click", "block.wooden_button.click_on"),
    ("gui.button.press", "ui.button.click"),
    ("tile.piston.in", "block.piston.contract"),
    ("tile.piston.out", "block.piston.extend"),
    ("ambient.cave.cave", "ambient.cave"),
    ("creeper.primed", "entity.creeper.primed"),
    ("game.hostile.die", "entity.hostile.death"),
    ("game.hostile.hurt.fall.big", "entity.hostile.big_fall"),
    ("game.hostile.hurt.fall.small", "entity.hostile.small_fall"),
    ("game.neutral.die", "entity.generic.death"),
    ("game.neutral.hurt.fall.big", "entity.generic.big_fall"),
    ("game.neutral.hurt.fall.small", "entity.generic.small_fall"),
    ("game.player.die", "entity.player.death"),
    ("game.player.hurt.fall.big", "entity.player.big_fall"),
    ("game.player.hurt.fall.small", "entity.player.small_fall"),
    ("game.player.swim", "entity.player.swim"),
    ("game.player.swim.splash", "entity.player.splash"),
    ("item.fireCharge.use", "item.firecharge.use"),
    ("minecart.base", "entity.minecart.riding"),
    ("minecart.inside", "entity.minecart.inside"),
    ("mob.guardian.attack", "entity.guardian.attack"),
    ("mob.guardian.elder.death", "entity.elder_guardian.death"),
    ("mob.guardian.elder.hit", "entity.elder_guardian.hurt"),
    ("mob.guardian.elder.idle", "entity.elder_guardian.ambient"),
    ("mob.guardian.flop", "entity.guardian.flop"),
    ("mob.guardian.land.death", "entity.guardian.death_land"),
    ("mob.guardian.land.hit", "entity.guardian.hurt_land"),
    ("mob.guardian.land.idle", "entity.guardian.ambient_land"),
    ("mob.wolf.step", "entity.wolf.step"),
    ("music.game", "music.game"),
    ("music.game.creative", "music.creative"),
    ("music.game.end", "music.end"),
    ("music.game.end.credits", "music.credits"),
    ("music.game.end.dragon", "music.dragon"),
    ("music.game.nether", "music.nether.nether_wastes"),
    ("music.menu", "music.menu"),
    ("records.11", "music_disc.11"),
    ("records.13", "music_disc.13"),
    ("records.blocks", "music_disc.blocks"),
    ("records.cat", "music_disc.cat"),
    ("records.chirp", "music_disc.chirp"),
    ("records.far", "music_disc.far"),
    ("records.mall", "music_disc.mall"),
    ("records.mellohi", "music_disc.mellohi"),
    ("records.stal", "music_disc.stal"),
    ("records.strad", "music_disc.strad"),
    ("records.wait", "music_disc.wait"),
    ("records.ward", "music_disc.ward"),
];

/// The modern event for a 1.8 sound name, or `None` when 26.3 has no
/// counterpart (removed mobs and records). A name with no mapping is still
/// playable against 1.8 resources: the packet handler plays it unchanged.
pub fn resolve(name: &str) -> Option<&'static str> {
    PAIRS
        .iter()
        .find(|(old, _)| *old == name)
        .map(|(_, new)| *new)
}

/// The 1.8 event whose samples an engine event should use: the event itself
/// when 1.8 kept the name, else the event it became. 26.3 picks music by
/// biome; 1.8 has one track set per dimension, so every biome-specific track
/// falls back to the dimension's set.
pub fn legacy(event: &str) -> Option<&'static str> {
    let path = event.strip_prefix("minecraft:").unwrap_or(event);
    if let Some((old, _)) = PAIRS.iter().find(|(_, new)| *new == path) {
        return Some(old);
    }
    if let Some(old) = renamed(path) {
        return Some(old);
    }
    if let Some((group, action)) = path
        .strip_prefix("block.")
        .and_then(|rest| rest.rsplit_once('.'))
        && let Some(sound) = block_sound(group, action)
    {
        return Some(sound);
    }
    if path.starts_with("music.nether.") {
        return Some("music.game.nether");
    }
    if path.starts_with("music.overworld.") || path == "music.under_water" {
        return Some("music.game");
    }
    None
}

/// Modern events 1.8 played under an unrelated name, and the shared samples
/// 1.8 used for buttons, doors, chests, dispensers and pistons.
fn renamed(path: &str) -> Option<&'static str> {
    Some(match path {
        "entity.generic.burn" => "fire.fire",
        "entity.generic.extinguish_fire" => "random.fizz",
        "entity.player.attack.crit" | "entity.player.attack.strong" => "random.successful_hit",
        "entity.player.attack.knockback" => "random.bowhit",
        "entity.dragon_fireball.explode" => "random.explode",
        "entity.ender_dragon.ambient" => "mob.enderdragon.growl",
        "entity.ender_dragon.shoot" => "mob.ghast.fireball",
        "entity.elder_guardian.curse" => "mob.guardian.curse",
        "entity.player.teleport" => "mob.endermen.portal",
        "entity.horse.eat" => "random.eat",
        "entity.fishing_bobber.throw" => "random.bow",
        "entity.fishing_bobber.splash" => "random.splash",
        "entity.fishing_bobber.retrieve" => "random.pop",
        "item.shears.snip" => "mob.sheep.shear",
        "item.bone_meal.use" => "random.pop",
        "block.piston.extend" => "tile.piston.out",
        "block.piston.contract" => "tile.piston.in",
        "block.chest.open" | "block.ender_chest.open" => "random.chestopen",
        "block.chest.close" | "block.ender_chest.close" => "random.chestclosed",
        "block.dispenser.dispense" | "block.dispenser.fail" => "random.click",
        "block.dispenser.launch" => "random.bow",
        "block.enchantment_table.use" => "random.levelup",
        "block.wooden_button.click_on" | "block.wooden_button.click_off" => "random.wood_click",
        "block.stone_button.click_on" | "block.stone_button.click_off" => "random.click",
        "block.lever.click" => "random.click",
        "block.wooden_pressure_plate.click_on"
        | "block.wooden_pressure_plate.click_off"
        | "block.metal_pressure_plate.click_on"
        | "block.metal_pressure_plate.click_off"
        | "block.tripwire.click_on"
        | "block.tripwire.click_off"
        | "block.tripwire.detach" => "random.click",
        "block.iron_door.open"
        | "block.wooden_door.open"
        | "block.iron_trapdoor.open"
        | "block.wooden_trapdoor.open"
        | "block.fence_gate.open" => "random.door_open",
        "block.iron_door.close"
        | "block.wooden_door.close"
        | "block.iron_trapdoor.close"
        | "block.wooden_trapdoor.close"
        | "block.fence_gate.close" => "random.door_close",
        "block.anvil.use" => "random.anvil_use",
        "block.anvil.destroy" => "random.anvil_break",
        _ => return None,
    })
}

/// The 1.8 `SoundType` samples for the per-block sound groups 26.3 names.
/// `anvil`, `glass`, `ladder` and `slime_block` override `SoundType` itself;
/// 1.8 had no anvil step sample (`block.anvil.step` is silent there).
fn block_sound(group: &str, action: &str) -> Option<&'static str> {
    if group == "slime_block" {
        return Some(if action == "step" {
            "mob.slime.small"
        } else {
            "mob.slime.big"
        });
    }
    if group == "anvil" {
        return match action {
            "place" => Some("random.anvil_land"),
            "step" => None,
            "break" | "hit" | "fall" => Some("dig.stone"),
            _ => None,
        };
    }
    if group == "glass" {
        return match action {
            // 1.8's glass SoundType places with the stone step sample.
            "place" | "step" => Some("step.stone"),
            "break" | "hit" | "fall" => Some("dig.glass"),
            _ => None,
        };
    }
    if group == "ladder" {
        // 1.8's ladder SoundType keeps the base place sound (`dig.wood`), and
        // mining hits and falls use the break sound too.
        return Some(match action {
            "step" => "step.ladder",
            "break" | "place" | "hit" | "fall" => "dig.wood",
            _ => return None,
        });
    }
    let (dig, step) =
        match group {
            // 1.8 had no metal SoundType: iron and metal blocks used STONE.
            "stone" | "metal" | "iron" | "nether_bricks" | "netherrack" | "nether_ore"
            | "spawner" => ("dig.stone", "step.stone"),
            "wood" => ("dig.wood", "step.wood"),
            "gravel" => ("dig.gravel", "step.gravel"),
            "grass" | "crop" | "vine" | "big_dripleaf" | "sponge" | "wet_sponge"
            | "nether_wart" => ("dig.grass", "step.grass"),
            "sand" | "soul_sand" => ("dig.sand", "step.sand"),
            "snow" => ("dig.snow", "step.snow"),
            "wool" | "cobweb" => ("dig.cloth", "step.cloth"),
            _ => return None,
        };
    Some(match action {
        "step" => step,
        "break" | "place" | "hit" | "fall" => dig,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{legacy, resolve};

    #[test]
    fn common_names_resolve_to_the_modern_events() {
        assert_eq!(resolve("random.explode"), Some("entity.generic.explode"));
        assert_eq!(resolve("mob.zombie.say"), Some("entity.zombie.ambient"));
        assert_eq!(resolve("step.stone"), Some("block.stone.step"));
        assert_eq!(resolve("dig.wood"), Some("block.wood.break"));
        assert_eq!(resolve("random.pop"), Some("entity.item.pickup"));
        assert_eq!(
            resolve("mob.zombiepig.zpig"),
            Some("entity.zombified_piglin.ambient")
        );
        assert_eq!(resolve("ambient.cave.cave"), Some("ambient.cave"));
        assert_eq!(resolve("records.cat"), Some("music_disc.cat"));
    }

    #[test]
    fn the_engine_finds_the_1_8_samples_for_modern_events() {
        assert_eq!(legacy("minecraft:block.grass.break"), Some("dig.grass"));
        // 1.8's GuiButton uses `gui.button.press` (the master category);
        // `random.click` is the block sound the server sends for levers and
        // buttons in the world, which then plays as sent.
        assert_eq!(
            legacy("minecraft:ui.button.click"),
            Some("gui.button.press")
        );
        assert_eq!(resolve("random.click"), None);
        assert_eq!(legacy("minecraft:ambient.cave"), Some("ambient.cave.cave"));
        assert_eq!(legacy("minecraft:music_disc.cat"), Some("records.cat"));
        assert_eq!(legacy("music.game"), Some("music.game"));
        assert_eq!(
            legacy("minecraft:music.nether.crimson_forest"),
            Some("music.game.nether")
        );
        assert_eq!(
            legacy("minecraft:music.overworld.desert"),
            Some("music.game")
        );
    }

    #[test]
    fn shared_sound_groups_and_samples_stand_in_for_per_block_events() {
        assert_eq!(legacy("minecraft:block.metal.break"), Some("dig.stone"));
        assert_eq!(
            legacy("minecraft:block.netherrack.break"),
            Some("dig.stone")
        );
        assert_eq!(legacy("minecraft:block.crop.break"), Some("dig.grass"));
        assert_eq!(legacy("minecraft:block.cobweb.break"), Some("dig.cloth"));
        assert_eq!(legacy("minecraft:block.glass.place"), Some("step.stone"));
        assert_eq!(
            legacy("minecraft:block.anvil.place"),
            Some("random.anvil_land")
        );
        assert_eq!(legacy("minecraft:block.anvil.step"), None);
        assert_eq!(legacy("minecraft:block.ladder.break"), Some("dig.wood"));
        assert_eq!(legacy("minecraft:block.ladder.place"), Some("dig.wood"));
        assert_eq!(legacy("minecraft:block.ladder.step"), Some("step.ladder"));
        assert_eq!(
            legacy("minecraft:block.slime_block.break"),
            Some("mob.slime.big")
        );
        assert_eq!(
            legacy("minecraft:block.slime_block.step"),
            Some("mob.slime.small")
        );
        assert_eq!(
            legacy("minecraft:block.chest.open"),
            Some("random.chestopen")
        );
        assert_eq!(
            legacy("minecraft:block.ender_chest.close"),
            Some("random.chestclosed")
        );
        assert_eq!(
            legacy("minecraft:block.piston.extend"),
            Some("tile.piston.out")
        );
        assert_eq!(
            legacy("minecraft:block.piston.contract"),
            Some("tile.piston.in")
        );
    }

    #[test]
    fn every_mapping_is_a_modern_entity_block_ui_item_weather_ambient_or_music_event() {
        for name in [
            "ambient.cave.cave",
            "random.explode",
            "mob.creeper.say",
            "note.bd",
            "tile.piston.in",
            "game.player.hurt",
            "liquid.lava",
            "fireworks.launch",
            "records.cat",
        ] {
            let event = resolve(name).unwrap();
            assert!(
                [
                    "entity.",
                    "block.",
                    "ui.",
                    "item.",
                    "weather.",
                    "ambient.",
                    "music.",
                    "music_disc."
                ]
                .iter()
                .any(|prefix| event.starts_with(prefix)),
                "{name} -> {event}"
            );
        }
    }

    #[test]
    fn unknown_names_are_not_invented() {
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("mob.enderdragon.flap"), None);
        assert_eq!(legacy("minecraft:block.potent_sulfur.break"), None);
        assert_eq!(legacy("minecraft:entity.witch.ambient"), None);
    }
}
