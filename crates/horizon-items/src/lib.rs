//! Blocks and items as the client knows them, below gameplay: block facts
//! (`blocks`), item stacks and definitions (`items`), their data components
//! (`item_data`), enchantments, cooldowns, bundles and item stacks in NBT.
//! horizon-client re-exports each module under its old path
//! (`horizon_client::items`, ...).
#![forbid(unsafe_code)]
pub mod banner_layer;
pub mod banner_patterns_1_8_9;
pub mod block_drops_1_8_9;
pub mod blocks;
pub mod bundle;
pub mod chest_loot_1_8_9;
pub mod content_namespaces;
pub mod cooldowns;
pub mod enchant;
pub mod enchant_1_8_9;
pub mod firework;
pub mod flowers;
pub mod fuel_1_8_9;
pub mod item_data;
pub mod item_nbt;
pub mod items;
pub mod legacy_1_8_9;
pub mod paintings_1_8_9;
pub mod random_sequences;
pub mod use_animation;
pub use horizon_client_core::Result;
