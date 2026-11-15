//! What Minecraft Java Edition 1.8.9 states about itself, from Mojang's
//! version JSON (`https://piston-meta.mojang.com/v1/packages/d546f1707a3f2b7d034eece5ea2e311eda875787/1.8.9.json`).
//!
//! This is the 1.8.9 target beside the pinned 26.3 [`crate::version`].
//! `crate::target` selects it for the Android/game build; the 26.3 table stays
//! the datafixer's destination, upgrading 1.8 data into the model the engine
//! simulates.

/// `WorldVersion.id()`; level.dat, the server list, crash reports.
pub const VERSION_ID: &str = "1.8.9";
/// `WorldVersion.name()`: the display name in the window title and logs.
pub const VANILLA_VERSION: &str = "1.8.9";
/// `SharedConstants.getProtocolVersion()`.
pub const PROTOCOL_VERSION: i32 = 47;
/// 1.8 saves carry no `DataVersion` tag; DataFixerUpper reads them as -1.
pub const DATA_VERSION: i32 = -1;
/// `pack.mcmeta`'s `pack_format` (the 1.8 resource-pack format).
pub const RESOURCE_PACK_FORMAT: (i64, i64) = (1, 0);
/// Data packs did not exist in 1.8; 0 means "none".
pub const DATA_PACK_FORMAT: (i64, i64) = (0, 0);
/// The Java major version 1.8.9 runs on.
pub const JAVA_VERSION: u32 = 8;
/// The Mojang Java runtime component named by the version JSON.
pub const JAVA_COMPONENT: &str = "jre-legacy";
/// `WorldVersion.stable()`.
pub const STABLE: bool = true;
/// `WorldVersion.buildTime()` as `Date.toString()` prints it in UTC.
pub const BUILD_TIME: &str = "Thu Dec 03 09:24:39 UTC 2015";
/// The asset index version JSON's `assetIndex.id` names.
pub const ASSET_INDEX: &str = "1.8";
/// The asset index's SHA-1, as published in the version JSON.
pub const ASSET_INDEX_SHA1: &str = "f6ad102bcaa53b1a58358f16e376d548d44933ec";
/// The client jar's SHA-1, as published in the version JSON.
pub const CLIENT_JAR_SHA1: &str = "3870888a6c3d349d3771a3e9d16c9bf5e076b908";
/// The client jar's size in bytes, as published in the version JSON.
pub const CLIENT_JAR_SIZE: u64 = 8_461_484;
