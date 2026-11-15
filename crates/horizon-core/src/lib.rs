//! Horizon's initial simulation primitives, targeting the Minecraft Java Edition version in [`version`].
//!
//! This is not a playable game. Differential checks cover only the operations
//! listed in docs/rust-parity.md; the rest of vanilla remains unimplemented.
#![forbid(unsafe_code)]

pub mod biome;
pub mod carving;
pub mod density;
pub mod fast_hash;
pub mod geometry;
pub mod jmath;
pub mod no_ai;
pub mod noise;
pub mod paletted;
pub mod player_texture;
pub mod position;
pub mod random;
pub mod storage;
pub mod text;
pub mod ticks;

pub mod release;
pub mod target;
pub mod version;
pub mod version_1_8_9;

pub use target::current::VANILLA_VERSION;
