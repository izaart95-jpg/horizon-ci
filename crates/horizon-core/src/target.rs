//! The Minecraft Java Edition version this build targets.
//!
//! Horizon is compiled for one version at a time: the protocol, the chunk
//! format, the assets and the gameplay rules all follow [`current`]. 26.3 and
//! 1.8.9 are ports of the same game, so the difference is a compile-time
//! choice, not a runtime mode (`docs/mc-1.8.9.md`).
//!
//! **The switch is the `pub use` below.** Every version-dependent constant is
//! read through [`current`] so retargeting is one line; the blame history
//! keeps the mapping. `crate::version` stays the generated 26.3 table, so the
//! datafixer can keep converting old data into the 26.3-shaped world model
//! (`horizon_world::INTERNAL_DATA_VERSION`).
//!
//! ```
//! use horizon_core::target;
//! // The build speaks exactly one protocol; the constant re-exports it.
//! assert_eq!(target::PROTOCOL_VERSION, target::current::PROTOCOL_VERSION);
//! ```

pub use crate::version_1_8_9 as current;

/// The wire protocol this build speaks, as `SharedConstants.getProtocolVersion`.
pub const PROTOCOL_VERSION: i32 = current::PROTOCOL_VERSION;

/// Protocol 107 is 1.9, which added the attack cooldown, shields and the
/// offhand slot. Rules that did not exist before it use this to gate.
pub const fn since_1_9() -> bool {
    PROTOCOL_VERSION >= 107
}

/// Protocol 47 is 1.8: no cooldown, no shields, no offhand, numeric block ids.
pub const fn is_1_8_9() -> bool {
    PROTOCOL_VERSION == 47
}

/// The Minecraft version whose prepared resources (block/item/biome tables,
/// models, sounds) this build reads, and whose official files the launcher
/// installs.
///
/// This is the version the game *is*: the 1.8.9 build reads the generated
/// `.local/horizon-client/1.8.9` root and downloads the 1.8.9 client jar and
/// asset index from Mojang. The engine's simulation model and the prepared
/// tables that describe it are still projected from 26.3 (the worldgen and
/// datapack debt in `docs/mc-1.8.9.md`), but a 1.8.9 build never downloads or
/// reads 26.3 game files.
pub const RESOURCES_VERSION: &str = current::VANILLA_VERSION;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_agrees_with_itself() {
        assert_eq!(PROTOCOL_VERSION, current::PROTOCOL_VERSION);
        assert_eq!(
            since_1_9(),
            PROTOCOL_VERSION >= 107,
            "the 1.9 gate must follow the protocol"
        );
        assert_eq!(is_1_8_9(), PROTOCOL_VERSION == 47);
        // A build is either the 1.8 target or a post-1.9 one, never both.
        assert_ne!(is_1_8_9(), since_1_9());
    }
}
