# Minecraft 1.8.9 target

> **Status: playable preview.** The wire, save, asset and download formats are
> 1.8.9. `crates/horizon-core/src/target.rs` selects protocol 47, the client
> implements the 1.8.9 login, play, chunk, entity, inventory and UI paths, and
> the game reads the generated `.local/horizon-client/1.8.9` root (380 blocks,
> 5,400 states) that the Android installer builds from Mojang's 1.8.9 client
> jar and `1.8` asset index. A world opens, plays and round-trips through a real
> 1.8.9 server. This is not a finished 1.8.9 rewrite: the simulation still runs
> the 26.3-shaped model, and the remaining debt below is what a release needs.
>
> Deliberate debt, in order of what a player notices:
>
> - **Behaviour, gameplay and world generation are projected from 26.3.** The
>   1.8 registry decides which blocks exist, but each state's gameplay facts,
>   block behaviour, collision/occlusion shapes and worldgen values are the
>   nearest 26.3 values, and most of the APK's `data/` tree (advancements,
>   dimension data) is 26.3 content, because the 1.8.9 client jar has no
>   `data/`. The exceptions are generated from the 1.8.9 jar itself: the
>   crafting recipes (`RecipeDispatcher`), the block-drop table, the eleven
>   chest loot tables, the 26 painting motives, the 39 banner patterns and
>   the per-species death drops (`mobs/drops_1_8_9.rs`). 1.8
>   terrain generation and mob AI are not ported. The 26.3 worldgen files also name blocks 1.8.9 never had; the 1.8
>   build maps the common stone, nether and wood families to the nearest 1.8
>   block (deepslate to stone, nether wart block to netherrack, stripped wood
>   to bark) and leaves the rest out as air, so creating a world does not stop
>   on an unknown identifier.
> - **Saves are read and written as 1.8.** `crates/horizon-client/src/save.rs`
>   writes 1.8 Anvil chunks, `level.dat` and `playerdata/<uuid>.dat`, and a real
>   1.8.9 server loads the result. The Overworld build range is clamped to Y
>   0..256 for this target, matching 1.8's 16 chunk sections and keeping the
>   surface near sea level 63; the 26.3 noise below Y 0 is not generated. The
>   conversion is lossy where 1.8 has no equivalent: only world Y 0..=255 is
>   written, entities are downgraded and merged into the chunk's
>   `Level.Entities` (entities 1.8.9 lacked are dropped), and Horizon's own data
>   (`data/`, `dimensions/`) is extra files the 1.8 game ignores. A spawn or
>   player Y outside 0..255 in an older preview save is moved to a safe height
>   when read and clamped when written. A saved chunk keeps the biomes it was
>   written with, rebuilt over the generator's 0..256 range: loaded 1.8 chunks
>   are padded to the engine's taller storage, and keeping their padded range
>   made new columns beside them fail biome-region validation.
> - **LAN host and the dedicated server still speak 26.3** to guests. The
>   single-player server thread is unaffected: it talks to the window through
>   memory, not the wire.
> - **Block drops are 1.8's, but a few non-player paths still differ.** Every
>   state's drop now comes from a generated 1.8 table (1,382 `(id, metadata)`
>   rows) with 1.8's draw order, fortune, silk-touch, shears and block
>   overrides instead of the 26.3 loot tables. Each stack's
>   `Block.spawnAsEntity` position is drawn where 1.8 draws it, immediately
>   after that stack's chance and item draws (`nextFloat() * 0.5 + 0.25` per
>   axis, 0.25..0.75 above the block), so a multi-drop block and an explosion
>   read the same stream vanilla does. An explosion pays ore experience from
>   1.8's `randomDropAsItem` even when the chance fails. A broken moving piston
>   drops the block its block entity carries, at chance 1 and fortune 0, and
>   drops nothing when the entity is gone. A command's `destroy` and a broken
>   moving piston roll each stack through `Block.dropResources`, so the
>   spawn position is drawn inside the roll and spawned with `pop_popped`.
>   Debt: `/loot` is not a 1.8 command, so its `mine` source evaluates through
>   `block_drops_in` (no `spawnAsEntity` draws) and `spawn` gives the stacks
>   the post-1.8 command's own offset.
>   A jukebox's record is not in its drop table: it pops through the block's
>   pre-removal side effect with 1.8's own offset.
> - **Chest loot is 1.8's lists and 1.8's fill.** The eleven chests 1.8 fills
>   (dungeon, mineshaft, nether fortress, both temples, the three stronghold
>   chests, the village blacksmith and the bonus chest) hold 1.8's items,
>   weights and stack counts, and the random level-30 enchanted book only
>   rolls 1.8's 25 enchantments. A roll writes one stack into a random slot,
>   overwriting what was there, and a count above the stack size becomes that
>   many single items, as `WeightedRandomChestContent.fillInventory` does; the
>   other 26.3 loot tables (fishing is 1.8's, structures 1.8 never had) are
>   still the data pack's.
> - **Mob death drops are 1.8's code.** 1.8 has no `entities/*` loot tables:
>   each species drops from `dropFewItems`/`getDropItem`/`addRandomDrop`, so
>   the target no longer rolls the 26.3 entity tables. The order, the 8.5%
>   equipment chance with 1% per Looting level, the 25-durability loss, the
>   2.5% rare drop, a charged creeper's one-head budget and a skeleton's
>   record are 1.8's, and a baby drops neither items nor experience. An
>   animal's or water creature's experience uses 1.8's `world.rand`.
> - **Paintings are 1.8's 26 motives.** The variant registry is generated from
>   the 1.8 jar's `PaintingMotive` enum, and their textures are cropped from
>   the 1.8 painting atlas, so placing or saving a painting cannot pick 26.3's
>   larger art set (which a 1.8 save cannot name and the jar has no texture
>   for).
> - **Natural spawning is 1.8's.** Each biome's four monster/passive/ambient/
>   water lists, the 70/10/15/5 category caps, `tickSpawners`' 17×17 chunk pass
>   and per-chunk limit, `findSpawnLocation`, `isSpawnable`, `method_2469` and
>   every `canSpawn`/`getLimitPerChunk` override come from the 1.8 jar and run
>   in `biome_spawns_1_8_9.rs` and `mobs/spawn_1_8_9.rs`. A 1.8 world no longer
>   spawns foxes, pandas, goats, axolotls, fish or other post-1.8 mobs because
>   the lists are 1.8's, not because 26.3's were filtered.
>   Debt: which biome a column has still comes from the 26.3 generator, a
>   chunk's decoration random stream is still 26.3's (so the packs the
>   generator places afterwards cannot be seed-identical yet), and 1.8
>   village generation is unported; a dedicated server honours its
>   `spawn-animals`/`spawn-monsters` properties, while single player follows
>   the difficulty (monsters above peaceful, animals always).
- **Some 1.8 screens are still 26.3 layouts.** The title screen, options,
>   create-world, the in-game HUD, the pause menu and its "Open to LAN" screen
>   use 1.8 layouts. 1.8 has no world-options screen, so the pause menu does
>   not open one; `/difficulty` is the only way to change it. The statistics
>   screen is the 1.8 one (General, Blocks, Items and Mobs tabs, sorting and
>   scrolling); its `CRAFTING_STATS` rows come from the generated 1.8 recipe
>   set and the 1.8 smelting table.
>   Achievements are the 1.8 tree, screen and popup, and the code that
>   simulates the world grants them: single player's server thread is the one
>   running the criteria, so the popup and the tree fill in as in 1.8.
>   `exploreAllBiomes` records the 1.8 exploration biomes over the modern
>   biome names.
>
> Stages 1–3, stage 4's registry and model set, stage 7's installer and the
> client half of stage 5 are implemented; the rest is listed below. See
> [PORTING.md](../PORTING.md) for the process.

## Why this is a rewrite, not a version switch

The pinned version is the specification the engine is built against, not a
runtime setting:

- `crates/horizon-core/src/version.rs` fixes 26.3, protocol 777, DataVersion
  5023 and the 26.3 pack formats. `target.rs` re-exports the 1.8.9 constants
  for the build, while the 26.3 table stays the datafixer's destination: it
  upgrades 1.8 data into the model the engine simulates.
- `crates/horizon-prepare` compiles Horizon's Java exporter/oracle classes in
  `mods/horizon/src/test/java/dev/horizonlab/parity/` against the 26.3 client
  and server jars. Those classes call 26.3 APIs and do not compile against 1.8.9
  jars.
- `crates/horizon-protocol` models protocol 777: the configuration state,
  26.3 packet ids, data-component item stacks and paletted chunks. The 1.8.9
  module now models protocol 47 beside it, and the simulation still builds
  26.3-shaped packets that `Game::serverbound_1_8_9` translates at the wire.
- `horizon-world` implements 26.3 world generation, block behaviour and entity
  physics. A 1.8.9 single-player game needs the 1.8 rule set and generator.
- The 300+ parity harnesses compare against 26.3 jars. A 1.8.9 target needs its
  own oracles and fixtures, or those checks no longer describe the build.

Changing the version constant alone would compile a client that speaks the wrong
protocol, reads the wrong registry and writes the wrong world format. The
switch therefore moves one layer at a time, and each layer states whether it is
1.8 or still 26.3-shaped; the debt list above is that statement.

## Exact 1.8.9 metadata

From Mojang's [version JSON](https://piston-meta.mojang.com/v1/packages/d546f1707a3f2b7d034eece5ea2e311eda875787/1.8.9.json):

| Field | Value |
| --- | --- |
| Version id | `1.8.9` |
| Protocol | `47` |
| DataVersion | none (1.8 saves have no tag; DataFixerUpper reads them as `-1`) |
| Resource-pack format | `1` |
| Data-pack format | none (data packs arrived in 1.13) |
| Java | `8` (`jre-legacy`) |
| Asset index | `1.8` (`f6ad102bcaa53b1a58358f16e376d548d44933ec`) |
| Client jar | `3870888a6c3d349d3771a3e9d16c9bf5e076b908`, 8,461,484 bytes |

The constants live in `crates/horizon-core/src/version_1_8_9.rs`, beside the
generated 26.3 `version.rs`, and `crates/horizon-core/src/target.rs` re-exports
one of the two as `target::current`. **That `pub use` is the switch**; every
version-dependent call site reads `target::PROTOCOL_VERSION` or a helper
(`target::is_1_8_9()`, `target::since_1_9()`) instead of a literal.

`horizon_world::DATA_VERSION` (5023, the shape of Horizon's own side data and
of the 26.3 save format) and `horizon_world::INTERNAL_DATA_VERSION` (the
in-memory model the datafixer converts to, also 5023) are deliberately separate
from `horizon_core::target::current::DATA_VERSION` (`-1` for 1.8.9). The build
reads 1.8 saves, upgrades them into the 26.3-shaped model the engine simulates,
and `SaveFormat::V189` converts the model back to the 1.8 layout on write.

## Stages

### 1. Protocol foundation (implemented)

`crates/horizon-protocol/src/v1_8_9/` models protocol 47:

- handshake (`set protocol`, next state status/login),
- status request/response/ping/pong,
- login start, encryption request/response, login success and compression,
- the joining play packets: keep alive, join game, position and look, chat and
  disconnect; every other play id decodes as `Unknown` with its raw payload.

Framing, zlib compression and AES/CFB8 encryption are shared with the 26.3
connection. Tests use byte-exact 1.8.9 vectors and an in-memory compressed
login/join/keep-alive exchange. `net.rs` drives this module for a 1.8.9
connection (`Remote::legacy_protocol`), including the online-mode RSA exchange
and session join.

### 2. Play protocol and connection loop (world/player baseline implemented)

`play.rs` now models the joining and world/player baseline with 1.8 codecs:
time, spawn position, health, respawn, held item, chunk data and bulk, block and
multi-block changes, game state, abilities, difficulty, player-list header, and
the client's movement, look, digging, placement, settings, plugin-message and
resource-pack packets.

Entity spawn (player and living), metadata, velocity, relative move/look,
teleport, head look, status, attach, effects, experience, equipment, animation,
block action/break, world event, sound, windows and slots, sign/tile-entity
updates, plugin messages, camera, resource packs, entity NBT, player info,
title and combat event are modeled in `entity.rs`, `player_info.rs` and
`play.rs`.

All 74 clientbound ids are modeled; `game/remote_1_8_9.rs` handles every one
except `SetCompression` (handled by the network thread) and `UpdateEntityNbt`
(logged, not applied). Sound ids keep the modern engine event names:
`horizon_client_core::sound_names_1_8_9::resolve` translates an incoming 1.8
name, and `SoundRegistry::key` falls back to the 1.8 event whose samples stand
in when the engine plays a 26.3-shaped id (the `1.8` asset index supplies
`minecraft/sounds.json`, which the client jar itself does not contain). An
unmapped name plays as the server sent it, so a 1.8 server's `random.click`
keeps its block sample instead of being rewritten to the UI event. Particle
ids resolve to the engine's types, including the argument-carrying particles:
reddust colour, mobSpell colour, note pitch (carried in the offsets by the 1.8
packet), and the item/block arguments of iconcrack, blockcrack and blockdust.
A 1.8 server names a jukebox's record in world event 1005 by the numeric
`Item` id (2256–2267, or 0 to stop), not by a registry id; `game/jukebox.rs`
resolves it through `legacy_item_name` and plays the same song as the item's
`jukebox_playable` component, and non-record data stops the sound as 1.8's
`playRecord(pos, null)` does. The 26.3 events 1010 and 1011 stay for
registry-id servers.
1.8 has no chat signing, so the signed-chat packets of
26.3 have no counterpart. The connection uses the 1.8.9 packet ids for its whole
lifetime. `crates/horizon-protocol/src/v1_8_9/codec.rs` holds the wire types
that differ from 26.3: the packed position, numeric item stacks and multi-block
records.

The client sends no play packet until `JoinGame` arrives. A 1.8.9 server keeps
decoding login packets until it has sent that packet, so the `ClientSettings`
and `MC|Brand` that `NetHandlerPlayClient.handleJoinGame` sends must wait for
it: sent right after `LoginSuccess`, the server reads play id `0x15` while
still in `LOGIN` and drops the connection with `Bad packet id 21`.

### 3. World model and save format (chunk codec and save writer implemented)

`crates/horizon-world/src/chunk_1_8_9.rs` reads and writes the 1.8 chunk NBT:
`Level`, 16×16×16 sections with numeric ids, metadata and `Add`, block/sky
light, biomes, heightmap, tile entities, entities and tile ticks. Unknown
`Level` fields and optional arrays survive a round trip, so loading and saving
an unmodified chunk is lossless. `from_network`/`from_bulk` decode the separate
wire layout (packed little-endian `u16` states, columnar light arrays and each
bulk column's own 256-byte biome array, written after that column's sections)
into the same model. `to_current_nbt` runs the datafixer over a
whole column, which is required because several 1.8 block fixes depend on
neighbours or tile entities. `save.rs`'s `decode_for` uses it to read 1.8
region chunks and levels; `encode_1_8_9_with_biomes` builds a column and writes
it back with `to_nbt`.

`crates/horizon-world/src/legacy_1_8_9.rs` maps a 1.8 numeric block state to
the current block state and back (`block_state`, `legacy_block_state`).
`crates/horizon-items/src/legacy_1_8_9.rs` does the same for stacks through the
datafixer's item pipeline, so metadata becomes the flattened item and tool wear
becomes the damage component; `legacy_stack`/`legacy_name` are the reverse for
the save writer, and `named_stack` reads the raw pre-flattening `id` string a
1.8 file holds.

The save writer is wired. `SaveFormat::V189` (what `SaveFormat::of_target`
selects for this build) writes the 1.8 layout: chunks go through
`encode_1_8_9_with_biomes` into `world/region`, `DIM-1/region` and
`DIM1/region`; `level.dat` carries `version` 19133, no `DataVersion`, a
`Player` compound and camelCase `GameRules` strings; server players go to
`playerdata/<uuid>.dat`. Items keep the pre-flattening names (`minecraft:wool`
with `Damage` 14), tile entities use the legacy ids (`Chest`, `Furnace`,
`Sign`, ...), and the section writer always writes the 2,048-byte `BlockLight`
array (plus `SkyLight` in the Overworld) because 1.8's loader constructs a
nibble array from both keys unconditionally.

Block metadata is only written where 1.8 registers the state: chests start at
metadata 2 and furnaces at 2, so a chunk written with metadata 0 reads as air
in a 1.8 game. `block_state_data::is_registered` is that table, and the reverse
map in `legacy_block_state` uses only registered ids, plus the snowy
grass/mycelium states the 1.12 table dropped.

Entities go through `entity_1_8_9`: ids become the 1.8 names (`Zombie`, `Item`,
`EntityHorse`), UUIDs become `UUIDMost`/`UUIDLeast`, living entities get
`HealF` and a short `Health`, and the conversion covers `FallDistance`, a JSON
`CustomName`, the five-slot `Equipment`/`DropChances` lists, numeric
`ActiveEffects`, villager offers, falling-block `TileID`/`Data`, the enderman's
carried block and the horse/boat `Type`. 1.8 has no `entities/` folder, so
`write_entities_1_8_9` rewrites the queued region record with the downgraded
tags under `Level.Entities`; `decode_column` reads them back into the chunk and
the sidecar is still appended for older or modern saves. Entities 1.8.9 did not
have (shulkers, husks, ...) are dropped on write; an item entity whose stack has
no 1.8 name is dropped too, since 1.8 otherwise reports "has no item?!" for it
every tick; passengers, attribute modifiers, villager trades and some horse/boat
details are lossy.

A real 1.8.9 server (`server.jar` with Java 8) loads the world built by
`crates/horizon-client/examples/make_1_8_9_world.rs`, keeps the stone floor,
chest, furnace, sign, wool and log blocks, reads the chest/furnace inventories
and the sign text back, and spawns the example's dropped item and cow from
`Level.Entities` with the expected legacy item and health fields. That example
needs no prepared resources: it builds a minimal block registry from the
1.8 → current datafixer. A world saved by the game itself also opens in the
same server with no `Couldn't load chunk` errors and no item-entity errors. It
has no `players/` folder (1.8 treats that as a pre-1.7 save), keeps the player
in `playerdata/<uuid>.dat` and the statistics in `stats/<uuid>.json`.

Still to do: only world Y 0..=255 is written (1.8 is 256 high); the heightmap
is a surface scan rather than 1.8's `HeightMap` recalculation across all loaded
chunks. A 1.8 save writes no proto chunks at all: 1.8 has only full columns,
and the 26.3 proto shape carries `Heightmaps` long arrays that its NBT reader
cannot parse (type 12 is unknown, so `NbtCompound.readNbt` NPEs), so an
interrupted generation leaves the chunk to be generated again instead of a
record 1.8 cannot read. The datafixer still upgrades 1.8 data to the
26.3-shaped model, which is what the engine simulates; the writer converts it
back.

### 4. Assets and preparation (partial)

The pack reader accepts resource-pack format 1, the `.lang` parser handles the
1.8 file shape, and texture/model/font lookups fall back to the 1.8 paths.

`target::RESOURCES_VERSION` is `1.8.9` for this build: the game reads
`.local/horizon-client/1.8.9`, and the Android installer downloads the 1.8.9
client jar and the `1.8` asset index. `research-lock.json` keeps the 26.3 pins
for the research tools and lists the 1.8.9 client/server SHA-1s under
`minecraft_versions`.

`horizon-prepare` has no 1.8.9 exporters yet: its Java exporter and oracle
classes compile against 26.3. The 1.8.9 root is produced by
`crates/horizon-client/examples/prepare_1_8_9_registry.rs` (below), which
builds the registry and model states from the 1.8.9 client jar and projects the
26.3 tables onto them. Sound events need their own extraction: the 1.8.9 jar
ships no `sounds.json`, but the `1.8` asset index lists `minecraft/sounds.json`
beside the `.ogg` files and the Android installer downloads both.
Keep Mojang's jars and extracted assets out of Git (`.local/` only).

`crates/horizon-client/examples/prepare_1_8_9_registry.rs` is the stage-4
tool: it walks every block state the 1.8 runtime registers, asks
`legacy_1_8_9::block_state` for the engine name, and writes `facts.json`,
`model-states.json`, the filtered block tags and a copy of the client jar's
`assets/`. The 1.8 jar ships 340 blockstate files, named by flattened variant
(`oak_log.json`, `white_wool.json`), and the engine also needs files for the
families 1.8 split or renamed: `anvil` has three damage models, `oak_slab`
separates its double form, the terracotta colours live under
`*_stained_hardened_clay`, the lit furnace/redstone variants are separate
files, and simple renames (`grass` → `grass_block`, `hardened_clay` →
`terracotta`) are in the same set. The tool derives those definitions itself,
speaking the engine's property vocabulary (modern names and value sets), and
writes them over the copied assets. The result is 380 blocks and 5,400 states
from 469 blockstate files; every state binds to at least one model, or
deliberately to none for blocks another renderer draws (air, fluids, chests,
signs, banners). `missing-models.json` is empty. The state list starts with
`minecraft:air`, so `BlockStateId(0)` is the engine's empty state exactly as in
26.3: an unset palette entry, an empty section, a chunk outside the resident
world and a height outside the column all read back as air. The per-state
tables are projected in that same order. A property keeps the full
26.3 value domain when the 1.8 block has it, not only the values 1.8 metadata
can encode: leaf `distance` covers 0–7, redstone wire is `power` × four
connections (1,296 states), fire is `age` × five sides (512), and 1.8-only
properties are neither dropped nor joined by modern ones (`waterlogged` is
absent from stairs and leaves). The engine's decay, redstone and neighbour
rules can therefore reach every value they simulate, and the state list stays
the canonical Cartesian product the `StateTable` requires. The derived JSON is
synthesized from the 1.8 model files rather than read from a state-to-file map
in the (obfuscated) 1.8 block classes, so it is a reconstruction to check, not
a byte-for-byte copy of vanilla's binding.

Items are bound by a second generator, `prepare_1_8_9_items.rs`. A 26.3 resource
root names every item through `assets/minecraft/items/<name>.json` and draws it
from the model tree in that file; the 1.8 jar predates the folder, so without it
every icon resolved to the missing model. The tool walks the engine's item
table, keeps the names `legacy_id` says 1.8 had, and writes 566 definitions (69
of them special-rendered). An item takes, in order, its code-drawn model
(skull, banner, chest, potion, spawn egg), the jar's `models/item/<name>`, its
1.8 rename (`grass_block` from `grass`, `lily_pad` from `waterlily`, the beds
from `bed`, records from `record_*`), or the first variant model of its
blockstate. No item is left unresolved. The clock and compass textures are one
animated strip each in 1.8, so the tool crops `clock_00..63` and
`compass_00..31`, writes a model per frame and dispatches on 1.8's own
`daytime` clock and spawn-target compass instead of looping the strip in the
GUI. A spawn egg is one item (383) whose damage is the entity id; the tool
composites the two grayscale layers with the `EntityList.entityEggs` primary
and secondary colours of the 27 eggs 1.8 registers (no snow golem, iron golem,
ender dragon, wither or 1.9 shulker). Skull, banner and chest specials take
empty template models with the display transforms of the 1.8 item model, so
the code renderer supplies the geometry. `item-assets.json` names every
generated file; `tools/android_build.py` bundles exactly that list, and its
public filter keeps those generated files while still rejecting Mojang's.
1.8 display transforms name the hand contexts `thirdperson`/`firstperson`, the
unsplit 1.9 sources of the modern right-hand contexts, so the model decoder
accepts the old keys as the right hand and lets the left inherit.

The same tool projects the 26.3 per-state engine tables onto the 1.8 registry
by mapping each 1.8 state to its nearest 26.3 counterpart: gameplay facts,
block behaviours, landing and render rules, cached collision/occlusion/movement
shapes, and the worldgen bundle's block properties, cached support/collision
shapes, structure transforms, block flags, block tags, survival and placement
tables. A state maps by its full property key first. 1.8 metadata names fewer
properties than the 26.3 state (a slab has no `waterlogged`, a grass block no
`snowy`), so the tool then fills each missing property from the 26.3 block's
default state and looks again; without that step every 1.8 slab fell back to
the block's first source row (the top slab) and collided as a top slab. Only a
state that still does not match falls back to the block's first source row.
The block default follows the same rule: it is
the 1.8 state that agrees with the 26.3 default on every property 1.8 carries,
so `smooth_stone_slab` defaults to `type=bottom` rather than its first state
(the top slab). The 1.8 registry therefore decides which states
exist, and the projected row is exact for every property 1.8 encodes, but
properties 1.8 cannot express keep the 26.3 default and the behaviour and
gameplay values attached to each state are still the nearest 26.3 values. That
is the remaining content debt, not a formatting detail.

The engine's item registry is 1.8's now: `prepare_1_8_9_item_names` filters the
root's 26.3-shaped `items.json` down to the 566 names a 1.8 numeric id reaches
(plus splash potion, which the datafixer only names from a stack's damage bit),
writes the generated `items_1_8_9` table and, with `--filter-items`, the root's
`items.json` itself. `item_defaults::read` applies the same table on a 1.8
target, so commands, recipes and the creative menu cannot reach netherite,
elytra or the 1.9+ spawn eggs any more; a content mod's own names survive.

The creative menu is 1.8's own too. `tools/CreativeTabsProbe18.java` runs the
twelve `ItemGroup.itemGroups` of the real 1.8.9 jar (its `showItems` subtype
expansion and the search tab's `appendItemStacks` plus one book per
enchantment), and `prepare_1_8_9_creative` turns that dump into the
26.3-shaped `creative-inventory.json` through `legacy_1_8_9::stack`. The screen
draws 1.8's geometry: 28-pixel tab columns with a one-pixel gap, the last
column flush right (and clicked from `WIDTH - 26`, vanilla's own quirk),
28x32 cells cropped from `tabs.png` rows 0/32/64/96 instead of 26.3's
per-tab sprites, the 12x15 scroller at (232, 0) or (244, 0) disabled, a
15-character search box 89 pixels wide, the Survival Inventory tab's two
armour columns around a centred player preview, and no off-hand slot. Tabs
select on release, as `mouseReleased` does. The search box matches 1.8's
`search()`: the probe records every `getTooltip(null, false)` line
(`Formatting.strip`ed and lower-cased), so `sharpness` finds the enchanted
books and `regeneration` the potions.

The survival and creative inventories are 1.8's `InventoryScreen` with active
effects: the GUI moves right 60 pixels (`160 + (width - width0 - 200) / 2`)
and 140x32 panels sit at `x - 124`, cropped from `inventory.png` at (0, 166)
with the icon cell at (`18 * (m % 8)`, `198 + 18 * (m / 8)`), a `m:ss` (or
`**:**` for a `Short.MAX_VALUE` duration) line, and level suffixes only for
amplifiers 1..3. 1.8 iterates its effect map rather than sorting, so the rows
follow the client's effect order.

The 1.8 jar also has no `sounds.json` of its own; the registered 1.8 event
names come from the asset index's `minecraft/sounds.json`, and the engine's
modern ids reach those samples through `sound_names_1_8_9::legacy`.

The 1.8.9 client jar's assets were tried over the 26.3 tables directly (the
1.8.9 jar contains `assets/minecraft/{blockstates,models,textures,lang}`; its
sounds come from the separate `1.8` asset index). Real gaps fixed for that
path: a 1.8 blockstate names its model relative to `models/block` or
`models/item` (`"model": "stone"`), which the model library now resolves; a
bare `normal`/`inventory` variant key now selects every state; an empty
multipart list is a valid definition that binds every state to no model; and
missing 1.8 textures are found by their old names (`oak_planks` →
`planks_oak`, `bricks` → `brick`, `water_overlay` → `water_flow`). Leaf
metadata is translated both ways: 1.8's `decayable` becomes the inverted
`persistent`, `check_decay` becomes `distance` (7 while searching, 0 when
not), and a save folds distances 1–6 back to `check_decay=true` (reloading
restarts the search at 7, the one lossy mapping 1.8's two bits force). The
title-screen test terrain now asks for leaves without the post-1.13
`waterlogged` property, so the same binary loads either resource generation.
The 1.8 build hides the 26.3 toast manager: 1.8 has no `gui/sprites/toast/*`
and its achievements use `GuiAchievement` popups drawn from
`achievement_background.png`. System toasts (a failed world operation) reuse
that texture's 160x32 popup at (96, 202). Achievements are the 1.8 ones: the
same 34-entry tree, the generated block map, the pan/zoom viewport, the
taken/unlockable frames and connectors, the hover tooltip, and the sliding
popup a server's statistics response triggers. Statistics are the 1.8 screen
and file format: the four tabs, the
Block/Item header sorting and the Mobs rows follow `StatsScreen`, `ListWidget`
and `Stats` above, and the flat `stats/<uuid>.json` is read and written with
1.8's `Stat.name` keys.
The debug gallery no longer fails when the resource set has no state for its
26.3-only entries. A 26.3-generated world still renders as 1.8 where it can:
blocks the generator places under names 1.8 never had (deepslate, the new
ores) resolve to air, because the 1.8 registry is the only registry loaded.
The generated 1.8 root is what the 1.8.9 build reads, but it is not a full
1.8 content set: the registry and models are 1.8, while the behaviour, gameplay
and world-generation tables attached to those states are projected 26.3 values,
and most of the `data/` tree (loot tables, advancements, dimension and damage
data) still comes from 26.3 because the 1.8.9 client jar has none. The crafting
recipes are generated from the 1.8.9 `RecipeDispatcher` (below). Flower
pots are complete: the registry carries the empty `flower_pot` and the 21
potted blocks 1.8's tile entity can hold, each bound to the matching
`flower_pot` model. 1.8 saves and `S35PacketUpdateTileEntity` packets carry the
contents as a `FlowerPot` tile entity; the reader folds it into the potted
block state, and the writer synthesizes the tile entity from the state, so a
potted plant round-trips through a real 1.8 world and server. Pot insertion,
pick-block, the pot-plus-plant drops and the 1.8 rule that an empty hand leaves
a potted plant alone all use the shared block rules.

### Android installer

`official_install::install_with_progress` installs
`target::RESOURCES_VERSION` — 1.8.9 for this build. It fetches
`https://piston-meta.mojang.com/.../1.8.9.json`, checks the client SHA-1
against the `minecraft_versions` pin, downloads the client jar and the 734
objects of the `1.8` asset index, unpacks `assets/minecraft` into
`.local/horizon-client/1.8.9`, and overlays the APK's `engine-data.zip`. `Play`
never requests 26.3 files.

The APK bundle is built by `tools/android_build.py`, which reads the version
from `target.rs` (not from `research-lock.json`) and, for 1.8.9, adds the
generated `data/` tree and the derived `assets/minecraft/blockstates/`. On
device `android_resources::aggregate` keeps an existing non-empty aggregate, so
the projected worldgen and tag tables in the bundle survive an install whose
jar contributes no `data/`.

Private/full build only: the bundle still carries the 26.3 `data/` tree and
26.3-projected worldgen, behaviour and gameplay tables as the debt above. A
public build that ships only generated tables needs those rewritten for 1.8
(stage 6), so `tools/android_build.py --public` does not support the 1.8.9
target yet.

Status: playable preview, not a full release. The world opens, the 1.8 block
registry and models load, chunks and entities round-trip through a real 1.8.9
server, and the 1.8 sound set plays through the engine's mixer. Gameplay runs on
the 26.3-shaped simulation. Missing for a release: 1.8 world generation and mob
AI, the remaining 1.8 `data/` content, and redstone order.

### 5. Gameplay (client half implemented)

The client gates the rules 1.8 does not share: no attack cooldown or attack
indicator, sword blocking instead of shields, no offhand slot, no
sweep/swap-scale knockback, and 1.8's crit, armour and melee damage formulas.
A sword or tool installs 1.8's one `generic.attackDamage` modifier (4 plus
`ToolMaterialType.getAttackMultiplier` for a sword, 3/2/1 for an axe, pickaxe
and shovel, none for a hoe), and Sharpness adds 1.25 a level while Smite and
Bane of Arthropods add 2.5 against their `EntityGroup`, from
`EnchantmentHelper.getAttackDamage` instead of the 26.3 enchantment data. The
tooltip shows that block as 1.8 does: an empty line and one line per
`ItemStack.getAttributes` modifier, `+N Attack Damage` in blue (red for a
negative amount), with `DecimalFormat("#.###")` and no slot header (`HideFlags`
bit 2 still hides it). A stack's own `AttributeModifiers` NBT (loaded as the
`minecraft:attribute_modifiers` component) replaces the item's defaults rather
than extending them, its fixed attack-damage UUID takes the Sharpness bonus,
and its modifiers apply from every equipped slot, since 1.8 has no slot
groups; the legacy UUIDs become the datafix's named modifier ids
(`minecraft:base_attack_damage`), which is also the id the synthesized weapon
modifier uses.
Hunger and regeneration already matched 1.8 `FoodStats` byte for byte.
Air supply is 1.8's `EntityLivingBase` rule: underwater ticks drain one air
per tick, a water-breathing effect freezes the count, leaving water resets it
to 300 at once, and drowning starts only when the count passes −20 (the tick
that reaches it sets air to 0 and deals 2 damage). The 26.3 air rules stay on
the other target.

Furnaces use 1.8's `FurnaceBlockEntity.getBurnTime` and its 26-input
`SmeltingRecipeRegistry` (200 ticks per item), not the data pack's `fuel`
field and cooking recipes: planks and the other `Material.WOOD` blocks burn
300 ticks, wooden slabs 150, coal blocks 16000, wooden tools 200, sticks 100,
coal 1600, lava buckets 20000, saplings 100 and blaze rods 2400. A hopper may
put anything into any furnace slot, as 1.8's `canInsertInvStack` does; the
fuel rule only gates the menu slot. Experience is paid when a player takes
the output (`FurnaceOutputSlot.onCrafted`), from the product's table value.

Crafting uses the 1.8 recipe set, not the 26.3 data pack's: `tools/RecipeProbe18.java`
dumps the 1.8.9 `RecipeDispatcher` (308 shaped, 58 shapeless, 7 of other types)
and `examples/prepare_1_8_9_recipes.rs` writes 393 modern-shaped JSON files into
`.local/horizon-client/1.8.9/data/minecraft/recipe`, all named `1_8_9_*`; the
26.3 files and the `recipe/brewing` folder are removed. A damage of 32767 (any
variant) becomes every flattened variant, deduplicated by item name, so 1.8's
any-wood and any-wool shapes accept each flattened item. The seven other types
map to the engine's specials: `ArmorDyeRecipeType` to four `crafting_dye`
recipes, `FireworkRecipeType` to `firework_rocket`/`firework_star`/
`firework_star_fade` (the creeper shape takes the five 1.8 skull items),
`PatternRecipeType` to 16 `bannerduplicate`, `BookCloningRecipeType` to
`bookcloning` (generations 0..1), `MapCloningRecipeType` to `crafting_transmute`,
`RepairingRecipeType` to `repairitem`, and `CopyingRecipeType` to one
`crafting_special_bannerpattern` carrying the 38 craftable patterns in the 1.8
`BannerPattern` enum's order. `MapUpscaleRecipeType` is a shaped recipe whose
result is the input map with `map_is_scaling`. Two 1.8 rules differ from the
current specials and are branched on `is_1_8_9()`: repaired items drop every
enchantment (not just the curses), and the four 1.8 dyes that flattened to
non-`*_dye` items (`ink_sac`, `cocoa_beans`, `lapis_lazuli`, `bone_meal`) are
dyes. A banner pattern takes one banner with fewer than six layers; an
ingredient pattern accepts one pattern item and at most one dye stack (no dye
means black), a layout pattern requires the whole 3x3 grid to equal the layout
with one dye colour, and the first match in enum order wins. Debt: the 1.8
`data/` tree still supplies the loot tables and advancements, and the generated
recipe root has no cooking recipes at all (the 1.8 furnace uses its own table),
so only a data pack can add one.

Item names come from the 1.8 client's own English file. The current resource
root names items with current keys (`item.minecraft.diamond`), while 1.8's
`en_US.lang` only has the old ones (`item.diamond.name`), so the display name
used to fall back to the lowercased id. `tools/ItemNameProbe18.java` dumps every
1.8 `(id, damage)` name and `examples/prepare_1_8_9_names.rs` writes
`NAMES_1_8_9` (566 entries) from the current item table, keyed by each item's
`minecraft:item_name`; `Language::load` inserts it only when the root is a 1.8
one (`tile.stone.stone.name` is present), so a 26.3 root keeps its own names.
A locale file names the same items under the 1.8 keys, so the generator also
writes `LEGACY_KEYS_1_8_9` (538 current keys onto their `tile.<name>.name` /
`item.<name>.name` key, a banner onto `ItemBanner`'s
`item.banner.<colour>.name`) and `Language::load` copies each locale's value
onto the current key before the English table fills what it cannot answer. A
spawn egg is one 1.8 item key for all 27 eggs, so the loader composes the
locale's `item.monsterPlacer.name` with `entity.<Name>.name` from
`EGG_ENTITY_KEYS_1_8_9` (1.8's `EntityList` class names: `LavaSlime`,
`EntityHorse`, `PigZombie`).
Spawn eggs are 1.8's item 383 with the entity's network id in `Damage`
(`Creeper` 50, `Pig` 90, `Villager` 120); `legacy_id`/`legacy_stack` map the
current egg names by that id, the datafixer's `ItemSpawnEggFix` turns the damage
into `EntityTag.id`, and 1.9's shulker (69) is not a 1.8 egg. 1.8's
`ItemMonsterPlacer` ignores `EntityTag`: using an egg spawns the mob through
`onInitialSpawn` with `world.rand.nextFloat() * 360` yaw, half a block higher
when the top of a slab was clicked, and copies only the egg's display name to
the mob's `CustomName`. The 1.8 target does that and drops a current egg's
`entity_data` component when encoding a slot, and its egg use skips the 26.3
component application.

Names are plain text in 1.8: `ItemStack.getDisplayName` returns `display.Name`
verbatim (`setStackDisplayName` stores the raw string), and
`Entity.getDisplayName` wraps `CustomName` in `ChatComponentText`; neither
parses JSON. The `display.Name`/`CustomName` JSON-component form arrived with
1.13, so writing 1.8 data flattens a current component to its words (keeping a
plain string's `§` codes), and reading a datafixed name parses the JSON
component the 1.13 fixer wrote: `component::saved_name_text` for nametags,
holograms, container titles, command block names and tooltips, and
`saved_name_plain` for the field a 1.8 save writes back.

Potion damage is 1.8's bitmask, not a potion name: bits 0–3 select the
effects, bit 4 is the awkward marker, bit 5 glowstone (level II), bit 6
redstone (extended), bit 13 marks every brewed effect potion and bit 14 makes
it splash. `StatusEffectStrings.method_3472` reads a reagent's `+n`/`-n`/`&n`/
`!n` string, the brewing stand has four slots (three bottles and the
ingredient), no fuel, `BrewTime` only, 400 ticks and no brew sound, and every
reagent is consumed one at a time because 1.8 `Item.isFood()` is
`recipeRemainder != null` (no reagent has one). The screen uses the 1.8
176×166 layout: bottles at (56,46), (79,53), (102,46), the ingredient at
(79,17), the progress and bubble regions of `gui/container/brewing_stand.png`
and no fuel bar or empty-slot icons. A splash potion applies its effects
without the modern fire-dousing, axolotl or level-event-2007 behaviour, gives
the entity it hit full strength and sends level event 2002 with the damage
value. Names and tooltips are `PotionItem.getDisplayName`/`appendTooltip`:
the effect's `.postfix` name (or the potion prefix), red/gray effect lines,
`potion.empty`, then the dark-purple `When Applied:` block with 1.8's
attribute modifiers (speed +20% and slowness −15% per level multiplied-total,
strength +130% multiplied-total, weakness −0.5 added, health boost +4 added;
haste, jump boost and absorption grant none). Potion items encode as id 373
with the bitmask in the slot's `Damage` and no `Potion` NBT string; decoding
adds the empty tag compound `ItemPotionFix` needs, and names the two values the
fixer's `damage & 127` would read as water (8192, the sugar/ghast-tear/spider-eye
mundane, and 16384, its splash form) so they decode as `minecraft:mundane` and
re-encode as the same damage. Taking a bottle from a brewing stand grants the
1.8 `potion` achievement when the potion's damage is above 0, as
`SlotBrewingStandPotion` does: a normal non-water potion or any splash potion
(bit 14), on the single player server thread, a LAN host or a dedicated server.
1.8's `setDamage` only rewrites `Damage`, so a brewed bottle keeps its NBT
through the splash transmute: a custom name and `CustomPotionEffects` survive,
and the 1.8 writer emits the effects the way
`PotionEffect.writeCustomPotionEffectToNBT` does (`Id`, `Amplifier`,
`Duration`, `Ambient`, `ShowParticles`), so a save round trip keeps them.

Fishing rolls 1.8's own pools instead of the 26.3 loot tables: junk (11
entries, weight 83), treasure (6 entries, weight 6) and fish (cod 60, salmon
25, clownfish 2, pufferfish 13), with `FishingHook.getResult`'s thresholds
(junk 0.1, then treasure 0.05). Luck of the Sea narrows junk and widens
treasure, Lure narrows both, and 1.8 has no open-water rule. A junk or
treasure roll counts `junk_fished`/`treasure_fished`, and a fish counts
`fish_caught`, as `getResult` increments them. Treasure bows and rods keep
`WeightedRandomFishingLoot`'s damage range (25% damaged) and enchant at level
30; a treasure book becomes an enchanted book. `FishingHook` is an `Entity`, so
it owns its `random`: the launch spread, the movement and bite timers, the item
pick, its damage and enchant rolls and the XP roll draw from it, while the
level's stream only advances for `getResult`'s category roll, and casting the
rod no longer consumes it. The hook's own draws use 1.8's ranges: a bite is
`nextInt(random, 10, 30)`, the wait `nextInt(random, 100, 900)` and the swim
angle `nextGaussian() * 4.0`. The tick around those draws is 1.8's
`EntityFishHook.tick`: the hook moves every tick, the block ray runs before
the move and the nearest living mob or player along it is hooked (items
return false from `attackEntityFrom`), a hit block freezes it for one tick
because 1.8 never writes `xTile`/`yTile`/`zTile`, the drag is `0.92` (`0.5`
against a wall or the ground, `0.9` of that in water), the five-slice water
fraction adds `0.04 * (2f - 1)` to the motion every tick (`-0.04` in air), and
reeling pulls with `sqrt(distance) * 0.08` on Y for 3 rod damage. Debt: in
single player without LAN there is no host target, so the thrower's own body
is not hookable, and a hook's `new Random()` is not seed-reproducible in
vanilla either.

Block drops are 1.8's code, not the 26.3 loot tables. `tools/BlockDropProbe18.java`
dumps every 1.8 state (7,806 states) and
`examples/prepare_1_8_9_block_drops.rs` writes `block-drops.json`, one row per
`(id, metadata)` pair (1,382 rows): the item `getDropItem` returns, `getMeta`'s
damage, `getBonusDrops`/`getDropCount`, the silk stack and whether
`requiresSilkTouch` applies. `Items::block_drops_raw` uses the table under
`is_1_8_9()` and reproduces `Block.randomDropAsItem` in vanilla order:
`getBonusDrops`/`getDropCount` first, then per item the chance draw
(`1.0 / power` for every explosion, 1 otherwise), then `getDropItem`, which is
where gravel's flint roll and tall grass's 1/8 wheat-seed roll live.
`harvest` adds the player-only shears and silk branches, including the snow
layer's `layers + 1` snowballs, and the block overrides for leaves, crops,
potatoes, stems, nether wart, cocoa, beds, banners, jukeboxes and the lower
half of double plants are explicit. Breaking a double plant's upper half also
takes the lower half with it, as `DoublePlantBlock.onBreakByPlayer` does, and
`doTileDrops` is checked before the pop offsets. Ore experience is part of
the drop call: `explosion_experience` pays coal (0..2), diamond and emerald
(3..7), lapis and quartz (2..5) and redstone (1..5) after the drop draws,
including when every explosion chance failed, at the block centre and only
under `doTileDrops`. 1.8's `EntityXPOrb` also has no merge pass and builds
without a level-random draw, so the 26.3 merge id and spawn merge are skipped
under 1.8. Each stack's `Block.spawnAsEntity` position is drawn inside its own
drop call (`nextFloat() * 0.5 + 0.25` per axis), so a multi-drop block and an
explosion read the same stream as vanilla, and `doTileDrops` is checked before
those draws. `Block.dropResources` — a command's `destroy` or a broken moving
piston — evaluates through `block_drops_with` with `doTileDrops` and spawns each
`Popped` through `pop_popped`; only `/loot`, a command 1.8 did not have, still
reads `block_drops_in` and spawns at the command's own offset. A broken moving
piston drops the block its block entity carries, as
`BlockPistonMoving.dropBlockAsItemWithChance` does: the carried state's
`dropBlockAsItem(..., 0)` at chance 1 and fortune 0, whatever chance the
explosion passed, and nothing when the entity is gone. A jukebox's record is
not part of its block drop: the
`PreRemove::Jukebox` side effect pops it the way 1.8's
`BlockJukebox.dropRecord` does, with `(1 - 0.7) * 0.5` added to x and z and
`(1 - 0.7) * 0.2 + 0.6` to y after each coordinate's `nextFloat() * 0.7`, and
stops the song at the same time.

Chests are 1.8's lists, not the 26.3 loot tables. 1.8 has no loot-table
files: `DungeonFeature`, the mineshaft, nether fortress, desert and jungle
temples, the three stronghold chests, the village blacksmith and the bonus
chest hold a static `List<WeightedRandomChestContent>` and pass it to
`WeightedRandomChestContent.fillInventory` with their own count. Several also
append one random level-30 `enchanted_book`; the stronghold library appends
`method_6347(random, 1, 5, 2)`'s one to five books. `tools/ChestLootProbe18.java`
dumps all 117 rows and `examples/prepare_1_8_9_chest_loot.rs` writes the
eleven tables under `data/minecraft/loot_table/chests/`;
`horizon_items::chest_loot_1_8_9` maps each structure's field to the table id
and keeps 1.8's count (`8` for a dungeon, `3 + nextInt(4)` for a mineshaft,
`3 + nextInt(6)` for the blacksmith, `10` for the bonus chest, ...). The book
is a `book` in the table with `enchant_with_levels` 30 restricted to 1.8's 25
enchantments, so no Mending, Frost Walker or other post-1.8 enchantment can
appear. The blacksmith's list is written as `village/village_weaponsmith`,
the id the datafixer gives 1.8's `village_blacksmith` chest; the modern
village's other house tables stay as they are until 1.8 village generation is
ported. The public APK filter carries exactly these eleven generated files
and the 393 generated recipes from `data/`; every other loot table is still
the 26.3 tree. `LootData::fill_1_8_9` places each roll the way
`fillInventory` does: one stack in a random slot, overwriting, and a count
above the item's stack size becomes that many single items rather than a
split into max stacks.

1.8's paintings are its own 26 motives, not 26.3's 51. The 1.8 client has
no `painting_variant` registry: `PaintingEntity.PaintingMotive` is an enum of
name, pixel size and texture offset, and a saved painting's `Motive` is the
enum constant's name (`Kebab`, `SkullAndRoses`). `tools/PaintingProbe18.java`
dumps the enum from the real jar (26 rows) and
`examples/prepare_1_8_9_paintings.rs` checks the dump against
`horizon_items::paintings_1_8_9::VARIANTS`, then writes the 26 current
`data/minecraft/painting_variant/*.json` files and the `#placeable` tag. The
variants carry only `asset_id` and the block size: 1.8 has no painting
title/author tooltips. A save's motive still round-trips as the enum name
(`Kebab` → `minecraft:kebab` → `Kebab`), and the random placement picks only
these motives, so a 1.8 server never receives a `Motive` it cannot name.

The 1.8 jar has no `textures/painting/<motive>.png` at all: all 26 motives
and the wooden back patch are rectangles in one 256x256
`paintings_kristoffer_zetterstrand.png`, which `PaintingEntityRenderer` crops
at run time from each motive's `textureX`/`textureY` (the back is the 16x16
square at (192, 0)). The current renderer instead asks for
`painting/<asset_id>` and `painting/back`, so the same generator crops the 26
rectangles and the back patch into `assets/minecraft/textures/painting/`.
Without that step paintings do not render at all in a 1.8 resource root (the
back sprite was missing, and every front sprite with it); the atlas itself
stays Mojang's and is still downloaded with the jar, never bundled.

Banners use the same enum scheme. `BannerBlockEntity.BannerPattern` is 39
values of display name, two-letter save code and, for six of them, an
ingredient stack; 1.8 has no `banner_pattern` registry.
`tools/BannerPatternProbe18.java` dumps the enum and
`examples/prepare_1_8_9_banners.rs` writes the 39 current
`banner_pattern/*.json` files, whose `translation_key` is the prefix of 1.8's
`item.banner.<id>.<colour>` keys, plus `#no_item_required` and the six
ingredient tags, dropping 26.3's four post-1.8 patterns. The enum's display
name is already the current id for all 39, including the diagonal code/name
swap (`rd` saves as `diagonal_up_right`).

1.8 pattern textures are opaque white-on-black masks: white where the dye
paints the pattern, black where the banner below shows through. The current
renderer wants the 1.9+ form (white pixels with the mask in alpha, tinted by
the dye), so the same generator converts all 38 pattern masks. It also copies
1.8's `entity/banner_base.png` (pole and cloth) to the current models'
`entity/banner/banner_base`; without both steps a placed banner drew no cloth
and every layer painted a black flag.

Natural spawning is `MobSpawnerHelper.tickSpawners`, not the 26.3
`NaturalSpawner`. `crates/horizon-world/src/biome_spawns_1_8_9.rs` holds the
four lists of every 1.8 `BiomeGenBase` (736 entries over 61 biome names, dumped
from the 1.8 jar), the 70/10/15/5 category caps and the swamp hut's and ocean
monument's monster overrides. The pass gathers each non-spectator player's
17×17 chunks (counting the border and out-of-border positions in the cap's
divisor, as 1.8 does), applies the category gate and
`getPersistentEntityCount`, then for every chunk draws `findSpawnLocation`
(`roundUp(heightmap + 1, 16)`, then `rand(height)`), three attempts of four
tries, one weighted entry per attempt, the 24-block player and world-spawn
exclusions, the `method_10753` list test and `isSpawnable`, then the entity's
own `canSpawn` and `method_2469`, and stops the chunk at `getLimitPerChunk`
(wolf 8, horse 6, ghast 1, otherwise 4). The per-species rules are in
`crates/horizon-client/src/mobs/spawn_1_8_9.rs`; light reads use
`World.method_8557` and `method_8558` (stairs/slabs/farmland/opacity-0 get
neighbour light), and the heightmap and `recievesSunlight` use the exported
1.8 opacity. Ambient darkness is `World.method_3597(1.0)` with 1.8's
`MathHelper` sine table and the real rain/thunder gradients; `isThundering`
reads the gradient above 0.9. Single player passes
`spawnMonsters = difficulty != PEACEFUL` and `spawnAnimals = true`; a
dedicated server passes its `spawn-monsters`/`spawn-animals` properties,
whatever the difficulty is (MinecraftServer.setDifficulty). The breedable gate
is `world time % 400 == 0`; `doMobSpawning` and a debug
generator skip the pass. The fresh-chunk packs are 1.8's too:
`SurfaceChunkGenerator.decorateChunk` seeds the provider random with the chunk
coordinates and 1.8's `nextLong() / 2 * 2 + 1` (truncating, unlike 26.3's
`nextLong() | 1`, which differs on negative odd draws), reads the passive list
from the biome at the chunk's far corner `(cx*16+16, 0, cz*16+16)`, then
`MobSpawnerHelper.spawnMobs` loops on `nextFloat() < 0.1`, picks with
`Weighting.rand` on the world random, draws `min + nextInt(1+max-min)` and the
`(16,16)` origin, and for every mob makes up to four
`getTopPosition`/`isSpawnable` attempts with the yaw float and the
`nextInt(5)-nextInt(5)` drift (including the clamp redraw) in vanilla's order.
`getTopPosition` reads the generated per-block `Material.blocksMovement` /
`Material.FOILAGE` table (`block_materials_1_8_9.rs`, dumped from the 1.8 jar
by `tools/BlockMaterialProbe18.java`), not 26.3 opacity — glass, snow layers,
carpets, skulls and flower pots block movement, water and plants do not, and
leaves block but are skipped as foliage. Packs exist only in the overworld;
the nether, End, flat and debug generators place none. The 26.3-only
structures are dropped before the registry is built, and the sets they leave
empty with them, so a 1.8 world generates the mineshaft, village, stronghold,
desert and jungle temples, swamp hut, igloo, ocean monument and nether
fortress only. Debt: the 26.3 generator
still decides which biome a column has (the 1.8 list is selected by that biome
name), its decoration random stream is not 1.8's yet, so the draws before the
pack (and the pack itself) cannot be seed-identical, 1.8 village
generation is unported. `findSpawnLocation` bounds its opacity scan with the
kept surface heightmap instead of walking the whole column; `isFullCube` and
`World.isOpaque` carry
1.8's own exceptions over the exported occlusion flags: the redstone block is
not a full cube (it emits redstone power), while top-half stairs and slabs,
hoppers and the tallest snow layer are opaque.

Each mob a spawner creates finalizes through 1.8's `EntityLiving.onInitialSpawn`
(`crates/horizon-client/src/mobs/finalize_1_8_9.rs`): the superclass adds the
`nextGaussian() * 0.05` follow-range bonus, which the cave spider alone skips,
then the species runs its override — the horse's one-in-ten donkey and
one-in-five foal with 15..31 health, 0.1125..0.3375 speed and a 0.4..1.0 jump
strength, the sheep's 5-in-100 black, grey and light grey plus 3-in-100 brown
with the once-in-500 pink, the rabbit's six coats, the slime's 1/2/4 size (one
step bigger half the time on harder difficulties), the villager's five
professions, the zombie's loot picking, baby (with the baby's chicken
jockey: the first chicken within 5×3×5, else a new one), villager zombie, door
breaking, armour, pumpkin and `handleAttributes` bonuses (knockback
resistance, follow range, the leader's reinforcements and health), and the
skeleton's armour and bow or, four times in five in the nether, a wither
skeleton with a stone sword and four attack damage, the ocelot's
one-in-seven pair of kittens, babies at the parent's position and facing,
and the spider's one-in-a-hundred skeleton jockey (a skeleton at its
position and facing, finalized and mounted on it) plus, on hard difficulty
and with a tenth chance times the special difficulty, one of four long
effects (speed, strength, regeneration or invisibility). Sheep colour,
villager profession, the ocelot's kittens, a baby zombie's chicken jockey,
the spider's jockey roll and its effect draw on `World.random`, as in 1.8.
The spawn gear is 1.8's too: `setEquipmentBasedOnDifficulty` gives the armour
set, then `setEnchantmentBasedOnDifficulty` enchants the main hand and the
feet, legs, chest and head from the mob's own random. The selection is
`crates/horizon-items/src/enchant_1_8_9.rs` with tables generated from the real
jar by `tools/EnchantProbe18.java`: it gates on `EnumEnchantmentType` (so an
axe gets efficiency, never sharpness), walks the candidates in Java `HashMap`
order at the map's post-insertion capacity, and stops at
`canApplyTogether`. The same finalize runs for natural and generation spawns,
spawn eggs, `/summon` without NBT and spawner blocks; a mob loaded from NBT
keeps its saved state, as in 1.8.

Skulls are 1.8's, not 26.3's six head blocks. The 1.8 registry has one block id
(144) whose metadata is `SkullType | NO_DROP << 3`; the engine maps it to the
twelve modern head names (six floor with `rotation`, six wall with `facing`) for
32 + 8 = 40 states each, 240 in all. The floor/wall placement, `FACING` and the
`Rot & 15` rotation follow `SkullBlock.getStateFromData`, and the tile entity
keeps `SkullType`, `Rot` and `Owner` (a 1.8 `GameProfile`, {name, uuid,
properties}) in `SkullBlockEntity`; the item is 397 with `Damage = SkullType`
and a `SkullOwner` compound. A skull never appears in `block-drops.json`:
`SkullBlock.randomDropAsItem` is a no-op, and `Chunk.setBlockState` calls the old
block's `onBreaking` on every server-side change (explosions, piston moves,
player breaks), so the server's block-change hook pays the item after the
entity is removed, copies the profile for a player head, and draws the three
float pop offsets itself. `NO_DROP` suppresses the drop, and a creative break
sets it before the change, so no 1.8 skull is dropped in creative. The remote
path rewrites the typed block state from the tile entity's `SkullType`/`Rot`,
preserves `powered`, and syncs the block entity before the generic datafix
profile path; a client-side change only removes the block entity.

Still to port: 1.8 mob AI, the 1.8 world generator, and redstone/piston order.
Server authority hides most of these for remote play; single player and client
prediction do not.

The statistics screen is 1.8's: 48 General rows, 142 Blocks rows, 187 Item
rows and 27 Mobs rows, its own tab/header/scrollbar layout, list scrolling and
sorting, item and header tooltips, and the flat `stats/<uuid>.json` file the
1.8 server reads and sends (`PacketPlayOutStatistics`). The Block/Item row values
canonicalise through `Stats.method_8293`'s aliases (grass and farmland count as
dirt, a jack o'lantern as a pumpkin, ...). `Stats.CRAFTING_STATS` is built from
the generated 1.8 crafting recipes and 1.8's own smelting table. Debt: 1.8 names
are resolved through modern item names (`pack.names.display_item`), so
non-English names fall back to English.
Achievements are the 1.8 tree in the same stats file: `AchievementsAndCriterions`
creates 34 entries on its 24-pixel grid, `StatsScreen`'s achievement button opens
`AchievementsScreen`, and a 1.8 server's statistics response arms the
`GuiAchievement` popup. The screen's map is generated with the same
`String.hashCode` seed, and a locked entry fades by its distance to the nearest
taken parent. Every trigger the 1.8 criteria live in is ported to the code that
simulates the world (the server thread, a LAN host or a dedicated server):
crafting and furnace takes through `onCraftedBy`, pickups
(`EntityItem.playerTouch`, `diamondsToYou` for the thrower), kills
(`onKillEntity`'s IMob, `EntitySkeleton.onDeath`'s 50-block arrow,
`EntityGhast.attackEntityFrom`'s deflected fireball, `dropFewItems`' wither box),
breeding, `EntityPig.fall`, beacons (`constructBeacon` and the fourth layer),
portals, minecart rails, brewing, `overkill`, `openInventory` and
`exploreAllBiomes`. Counters reach a
window through the statistics answer, and a `horizon:achievements` payload carries
them over the internal protocol, which has no achievement registry.
`exploreAllBiomes` samples the biome at `(floor x, 0, floor z)` once a second,
as `EntityPlayerMP.onUpdate` does, keeps the requirement set in memory as
`StatFileWriter` does, and waits for its parent `theEnd` like
`StatFileWriter.canUnlock`. `BiomeGenBase.explorationBiomes` names 36 ids, but ten
are hill, plateau or shore variants the modern biome list folds into their base
name (desert hills into the desert, ice mountains into the snowy plains, the
mushroom shore into the mushroom fields, ...), so the achievement requires the
26 name families a current world can show instead. The End exit portal calls
`teleportToDimension(1)` while the player is still in dimension 1, and
`ServerPlayerEntity.teleportToDimension` grants `THE_END_2` for that `(1, 1)`
call and rolls the credits, so a repeat exit credits `theEnd2` by design; the
collapsed End-to-Overworld event is that call. The inventory key opens the
survival screen for a spectator too, as `MinecraftClient` does, and still
sends `OPEN_INVENTORY_ACHIEVEMENT`; `ServerPlayNetworkHandler.onClientStatus`
case 3 increments the counter whatever the game mode, and
`onClickWindow` ignores the spectator's slot clicks.

The enchanting table draws from 1.8's own 25-entry `enchantmentsList`, not the
26.3 `#minecraft:in_enchanting_table` tag (which adds Mending, Frost Walker and
the other post-1.8 enchantments). The three offers use 1.8's
`EnchantmentHelper.a(random, slot, bookshelves, stack)` cost formula, and a
click re-seeds `xpSeed + slot` and applies every entry
`EnchantmentHelper.buildEnchantmentList` returns (a book drops one random entry
from a list of two or more); the shown clue is one random entry of that same
list. 1.8's `en_US.lang` keys the enchantments by
legacy names (`enchantment.damage.all`), not the current description keys.
`Language::load` copies each locale's own value from its legacy key onto the
current key before the English fallback (`ENCHANTMENT_NAMES`), so a translated
1.8 language file names the enchantments in its language and only a missing
one falls back to English.

The anvil is `AnvilScreenHandler.updateResult`: the prior-work tax is
`input.repairCost + addition.repairCost`, renaming costs 1, a rename-only
result caps at 39 and any result at 40 or more is refused outside creative.
Every result raises the prior-work penalty, a rename included (`2 * base + 1`),
and the second slot counts as a book only when it is an enchanted book with at
least one enchantment (`getEnchantmentNbt(stack).size() > 0`), so an empty
enchanted book neither combines nor renames through the anvil.
The per-level enchantment fee is 1.8's switch on
`Enchantment.getEnchantmentType()`, which the jar fills with the enchantment's
weight: 10 costs 1, 5 costs 2, 2 costs 4 and 1 costs 8, halved with a floor of
one when the addition is an enchanted book. Item acceptance is
`Enchantment.isAcceptableItem`, not the enchanting table's type mask: an axe
takes sharpness, smite and bane, shears take efficiency, silk touch and
unbreaking, and a hoe only unbreaking. The conflict check asks the candidate's
`differs` about each enchantment already on the item, and that relation is
asymmetric in 1.8: a silk touch book combines onto looting, a looting book does
not combine onto silk touch. Repair materials are `Item.canRepair`: only
tools, swords and armour have one (wooden tools take any planks, stone
cobblestone, iron and chainmail iron, gold gold, diamond diamond, leather
leather), so a bow with string, a fishing rod with string, shears with iron and
a hoe with its material are not 1.8 repairs even though the 26.3 repairable
components say they are; two damaged items of the same kind still combine.

### 6. Single-player world generation

Port the 1.8 terrain generator, biome selection, caves, features and
structures. For a server-first cut this can come after stages 1–5; single-player
is not playable without it.

### 7. Android installer and launcher (private build implemented)

`official_install` loads the 1.8.9 version JSON, checks the client jar against
the `minecraft_versions` pin in `research-lock.json`, and downloads the `1.8`
asset index; `tools/android_build.py` packages the 1.8.9 engine bundle; the
launcher labels its target from `target.rs`. The 37 1.8.9 libraries are not
needed by the native client, which runs without a Java runtime. Still open: a
public build needs 1.8 worldgen and datapack content instead of the
26.3-derived tables. See ["Android installer"](#android-installer).

### 8. Parity and release

Add 1.8.9 oracles and harnesses, retire the 26.3-only checks that no longer
describe the build, and update `docs/android-direction.md`, `docs/rust-parity.md`
and the release notes. Do not ship a build that claims 1.8.9 while any stage
still consumes 26.3 data.

Known 1.8 debt, in the order it should be paid:

- World generation still runs the 26.3 generator with 1.8 block tables
  projected onto it (stage 6). Terrain, caves, features and structures are not
  yet 1.8 algorithms, so seed-for-seed parity with a 1.8.9 server is not there.
- The item registry and creative tabs are 1.8's (566 items, the twelve
  `ItemGroup` tabs generated from the real jar). The
  per-tick spawn rules, generation mob packs and the remaining `data/` trees
  (loot tables outside the generated 1.8 chest loot and block drops,
  advancements, dimension and damage data) are also still partly 26.3.
- A 1.8 state takes the 26.3 row of its nearest state: properties 1.8 cannot
  express keep the 26.3 default, and behaviour/gameplay values are projected
  rather than re-derived from 1.8 code.
- Chunk decoration still comes from the 26.3 generator, so the random stream
  before `MobSpawnerHelper.spawnMobs` (and therefore which packs a seed gets)
  is not 1.8's yet.
- Every 1.8 mob spawn path finalizes through 1.8's `onInitialSpawn` (see the
  gameplay section): natural and generation spawns, spawn eggs, `/summon`
  without NBT and spawner blocks; a mob loaded from NBT keeps its saved state.

## Rules during the rewrite

- Keep the 26.3 build and its tests green until the replacement is ready. The
  1.8.9 module is additive for exactly this reason.
- Do not copy Mojang jars, assets or decompiled sources into the repository.
- Prefer small, tested stages over one large switch; each stage should be
  reviewable and revertible.
- Use byte-exact protocol vectors and vanilla 1.8.9 as the oracle; a round trip
  through the same codec is not evidence of compatibility.

## Verification

Last full run in this tree (Linux, `--no-default-features` for the client):

| Suite | Result |
| --- | --- |
| `cargo test -p horizon-core --lib` | 114 passed, 3 ignored |
| `cargo test -p horizon-items --lib` | 81 passed |
| `cargo test -p horizon-protocol --lib` | 213 passed |
| `cargo test -p horizon-world --lib` | 314 passed, 2 ignored |
| `cargo test -p horizon-client --lib --no-default-features` | 1226 passed, 0 failed, 38 ignored |
| `cargo test -p horizon-client --bins --no-default-features` | 514 + 18 passed, 3 ignored |
| `cargo test -p horizon-client --tests --no-default-features` | 32 passed across the integration binaries |
| `cargo test -p horizon-client-render --lib -- --test-threads=1` | 948 passed, 22 ignored |
| `cargo check -p horizon-client --bin horizon-client --no-default-features` | builds |
| `live_1_8_9` against a real 1.8.9 server | 47 clientbound packet kinds decoded, 0 errors |
| `make_1_8_9_world` + real 1.8.9 `server.jar` | world loads, 0 chunk/item errors, exit 0 |
| Horizon-saved world + real 1.8.9 `server.jar` | 629 chunks in 4 region files load and save, no chunk errors |
| Fresh world on the 1.8.9 target + real 1.8.9 `server.jar` | `SpawnY 64`, 0 problem lines, exit 0 |
| Older preview save (`SpawnY -62`, player Y -10) reopened | player at Y 64, saved inside 0..255, real server loads it with 0 problem lines |
| `kept_biomes_take_the_generators_height_and_keep_saved_y` | kept biomes span 0..256 and keep their saved Y |
| `prepare_1_8_9_registry` + state check | 380 blocks, 5,400 states, 0 states the definition does not bind, state 0 is air |
| `prepare_1_8_9_recipes` from the 1.8.9 `RecipeDispatcher` dump | 365 shaped/shapeless + 28 special = 393 files, no skipped stacks |
| `cargo test -p horizon-client --lib the_1_8_9_recipe_set_loads_and_crafts` | passed: 393 files and 393 loaded recipes, all `1_8_9_*`, six 1.8 shapes craft |
| `cargo test -p horizon-client --lib the_1_8_9_banner_pattern_recipe_adds_a_layer` | passed: creeper head adds a black creeper layer, a red dye adds a red square-bottom-left layer |
| `prepare_1_8_9_chest_loot` from the `ChestLootProbe18` dump | 11 chest tables, 117 rows, 52 distinct items (all in `items.json`), 5 random-book entries with 1.8's 25 enchantments |
| `cargo test -p horizon-items --lib chest_loot_1_8_9` | 5 passed (the eleven categories and counts, the dump flattens through `legacy_1_8_9`, weights/counts/book shape, the library's 1..5 books, a real `LootData::fill`) |
| `cargo test -p horizon-world --lib fill_1_8_9_places_full_stacks_and_singles_like_1_8` | passed: a full stack lands in one slot, four rolls of two stay stacks of two, and 65 items become 65 singles at 65 slot draws |
| `cargo test -p horizon-client --lib a_1_8_container_is_filled_with_1_8s_slot_rules` | passed: `roll_container` on the real dungeon table equals `LootData::fill_1_8_9` for the same position seed |
| `prepare_1_8_9_paintings` from the `PaintingProbe18` dump | 26 motives, 26 variant files, 26 cropped textures plus `back.png`, and the 26-entry `#placeable` tag, no 26.3 motive or per-motive 26.3 texture left |
| `cargo test -p horizon-items --lib paintings_1_8_9` | 4 passed (enum order/sizes/atlas corners, the dump's pixels/Aztec2/bounds, crop rows and refusals, JSON/tag shape) |
| `cargo test -p horizon-client-render --lib a_1_8_world_never_spawns_a_post_1_8_mob` | passed: a 26.3 list keeps zombie/squid and drops fox/goat/axolotl |
| `prepare_1_8_9_banners` from the `BannerPatternProbe18` dump | 39 patterns, 7 tags and 39 textures: the four post-1.8 patterns gone, every converted mask opaque white where the dye paints, and `entity/banner/banner_base` copied from 1.8's `entity/banner_base.png` |
| `cargo test -p horizon-items --lib banner_patterns_1_8_9` | 4 passed (the 39 enum values and save codes, the dump's six ingredients, the alpha-mask conversion, JSON and tag shape) |
| `engine-data-generated.zip` after chest loot, recipes, paintings and banners | 1,339 entries: the eleven 1.8 chest tables, 393 `1_8_9_*` recipes, 26 painting variants with their tag plus 27 cropped textures, 39 banner patterns with 7 tags and 39 converted textures, no other loot table, recipe or advancement |
| `prepare_1_8_9_names` from the 1.8.9 name dump | 566 current item keys named, 0 items without a 1.8 variant |
| `prepare_1_8_9_items` over the 1.8.9 client jar | 566 definitions (69 special), 763 generated assets, 0 unresolved, 663 model references all present |
| `cargo test -p horizon-assets --lib model::tests` | 4 passed (1.8 `thirdperson`/`firstperson` displays reach the hand contexts) |
| `cargo test -p horizon-client-core --lib lang::tests` | 8 passed (a 1.8 root names items with current keys and leaves a 26.3 root alone) |
| `cargo test -p horizon-items --lib legacy_1_8_9::tests` | 18 passed (`Creeper` egg is 383/50, `Pig` 383/90, the 27 coloured 1.8 eggs encode, the shulker, ender dragon and wither eggs do not, records are 2256–2267) |
| `cargo test -p horizon-world --lib a_1_8_jukebox_pops_its_record_like_block_jukebox_drop_record` | passed: `java.util.Random(42)` gives x 0.6592945754528046, y 0.6982656478881836, z 0.6282564103603363 after three ordered float draws |
| `cargo test -p horizon-client --bins level_event_1005_names_a_record_by_its_numeric_id` | passed: 2256 → `music_disc_13`, 2267 → `music_disc_wait`, 0/1/2255 stop |
| `a_banner_carries_its_colour_and_patterns_in_block_entity_tag` | passed: `Base`/`Color` are dye metadata and the white banner item keeps the colour |
| `engine-data.zip` after the 1.8.9 items | 11,792,209 bytes, 11,037 entries, 566 item definitions and the generated item assets |
| `engine-data.zip` after the state-projection fix | 12,106,631 bytes, 11,070 entries, 47,161,672 bytes uncompressed |
| v37 private aarch64 APK | versionCode 37, versionName `0.7.9-1.8.9`, 103,080,857 bytes, sha256 `6a3c7a53c42b49ec809e97d6d951aeb63a7b87b7d6c7d71f380137f419b488d6`; carries the rebuilt 90,878,944-byte `.so` and the engine bundle above |
| `a_pearl_feels_the_water_its_box_touches` | passed: state 0 is air, so the test scene's empty sections no longer stop the pearl before it reaches the pool |
| `cargo test -p horizon-assets --lib` | 14 passed (empty multipart, 1.8 variant keys) |
| `cargo test -p horizon-world --lib legacy_1_8_9` | 15 passed (leaf distance 1..6 saves as `check_decay=true`, 26.3-only aliases, `getByBlockState` packing, all 22 flower-pot states) |
| `cargo test -p horizon-world --lib chunk_1_8_9` | 13 passed (bulk columns carry their own biomes, wire layout) |
| `a_1_8_flower_pot_round_trips_its_contents` | potted states save as block 140 plus a `FlowerPot` tile entity and load back as potted states |
| `cargo test -p horizon-client-render --lib chunk_1_8_9` | 4 passed, 2.4 s (a real 1.8.9 save resolves every distinct state) |
| `cargo test -p horizon-client-core --lib sound_names_1_8_9` | 5 passed (1.8 event names, UI press, shared samples) |
| `bare_model_names_resolve_in_the_1_8_folders` | passed |
| `tools/android_build.py` bundle | 11,889 entries, 45.7 MB uncompressed, 1.8.9 tables + `data/` |
| Real 1.8.9 server behind a protocol-47 frame logger | `LoginSuccess` → `JoinGame` → client `ClientSettings`/`MC\|Brand`, 348 chunks decoded, 0 decode errors, no `Bad packet id 21` |
| `cargo test -p horizon-client --lib title::` | 64 passed (1.8 title, options and create-world layouts) |
| `cargo test -p horizon-client-core --lib key_display` | 2 passed (1.8 LWJGL key names) |
| `cargo test -p horizon-client --lib --no-default-features drops_1_8_9::tests::` | 14 passed (cow 42 leather 2/beef 1, sheared sheep, burning chicken, saddled pig, guardian sponge, charged-creeper head budget, skeleton record, blaze/ocelot/baby, horse inventory, equipment chance and 195 damage, monster-only equipment XP, animal experience from the world random) |
| `cargo test -p horizon-client --lib --no-default-features mobs::enderman::tests::a_killed_enderman_drops_its_carried_block` | passed: a 1.8 enderman leaves its carried block and drops ender pearls only |
| `cargo test -p horizon-client --lib --no-default-features spawn_1_8_9` | 10 passed (roundUp, per-species chunk limits, the 1.8 sine-table ambient darkness, the world-border block test, squid/guardian as the only water spawns, the heightmap spawn location, weighted picks, an empty list) |
| `BiomeSpawnProbe18` dump | 736 rows over 61 biome names, 18 distinct lists, the four 1.8 category caps and the swamp-hut/ocean-monument monster overrides, written to `biome_spawns_1_8_9.rs` |
| `cargo test -p horizon-client --lib resident_bubble_columns_and_nautilus_preserve_their_air_rules` | passed on both targets: 1.8 freezes air while water breathing, resets it to 300 on leaving water and drowns at −20 |
| `prepare_1_8_9_registry` + `cargo test -p horizon-client --lib lan::prediction::tests` | bottom slab state 4741 projects the 0.0..0.5 shape (not the top slab's); 19 prediction tests pass |
| `cargo test -p horizon-items --lib enchant_1_8_9` | 5 passed (the jar's accept/apply masks, the weight-keyed anvil fee, the generated tables) |
| `cargo test -p horizon-client --lib stations::tests` | 11 passed (1.8 fee, combine direction and gate, repair materials, the empty-book rule, the rename penalty) |
| 1.8.9 `MinecraftClient.handleKeybinds` bytecode | the inventory key sends `OPEN_INVENTORY` and opens the survival screen in every game mode; the remote path sends it too (internal action 3, wire action 2) |
| `cargo test -p horizon-client-render --lib one_eight_splash_keeps_the_stacks_own_nbt` | passed: the splash transmute keeps the custom name and the custom speed effect |
| `cargo test -p horizon-items --lib a_custom_potion_keeps_its_effects_in_1_8_nbt` | passed: `Id` 1, `Amplifier` 1, `Duration` 200, `Ambient` 0, `ShowParticles` 1, and the reader gives the effect back |
| `cargo test -p horizon-client-core --lib lang::tests` | 10 passed (the Russian, German and Chinese bundles, the 1.8 `.lang` parser, a locale's legacy enchantment keys and item names, a composed spawn egg, the English fallback) |

Run `horizon-client-render` serially: its GPU tests can crash when several run
at once. The ignored tests need prepared resources.

The previously failing 26.3-shaped tests now pass on the 1.8 root, each
branched on `target::is_1_8_9()` rather than deleted: soul-sand bubble
columns, netherite, weakness/health-boost modifiers, movement prediction,
the furnace quick-move, the movement rules and the prepared data root each
assert the 1.8 rule on this target and keep their 26.3 checks on the other.

The network layer is also checked against a real server:
`cargo run -p horizon-protocol --example live_1_8_9 -- 127.0.0.1:25599` joins a
1.8.9 server (offline mode, `network-compression-threshold=-1`), answers
keep-alives and the spawn teleport, sends settings, animation, entity action,
creative-slot, held-item and digging packets, then drives op commands that make
the server emit chunks, entities, metadata, particles, sounds, scoreboard,
titles, explosions and tab completion, and finally places a sign, answers the
sign editor and submits its text. The probe fails on any decode error or
missing required packet. `TileEntityData` is not required: 1.8.9 sends sign
updates as `UpdateSign`, and `TileEntityData` only covers non-sign block
entities when their block is updated after the entity exists. The server jar
and its world live under `.local/` and are never committed.

The parity harnesses still expect the 26.3 prepared data in `.local/`, so they
are not part of the 1.8.9 verification yet.

## Verify stage 1

```sh
cargo test -p horizon-protocol --lib v1_8_9
cargo test -p horizon-core --lib
```

The protocol tests cover the 47 handshake bytes, the dashed login UUID, the
compression threshold, the join-game layout and an end-to-end login over a
compressed in-memory connection.
