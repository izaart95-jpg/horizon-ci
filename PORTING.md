# Porting Horizon to another Minecraft version

This is the working guide for replacing Horizon's pinned Minecraft target with a
different Java Edition version. It is written from the 1.8.9 port, but the order
and the checks apply to any version (1.12.2, 1.16.5, 1.20.1, ...).

The rule that shapes everything: **the Minecraft version is a compile-time
specification, not a runtime setting.** Horizon is a clean-room reimplementation,
so a new version means new packet layouts, world formats, assets, rules and
oracles — not a protocol translation layer. Port in stages and keep the old
target green until the new one is ready.

> Status of the 1.8.9 port: the switch is flipped (`target::current` is
> `version_1_8_9`), the full protocol-47 play set is modeled and the client
> implements the 1.8.9 login, play, chunk, entity, inventory and UI paths. The
> engine still simulates the 26.3-shaped world model, reads 26.3 prepared
> resources and writes 26.3 saves; LAN/dedicated hosting still speaks 26.3. See
> `docs/mc-1.8.9.md` for the remaining debt.
>
> Verification in this tree: the library and bin suites pass —
> `horizon-core` 114, `horizon-items` 52, `horizon-protocol` 213,
> `horizon-world` 314, `horizon-client` lib 1178, bins 531 and integration
> tests 32, `horizon-client-render` 938 (run with `--test-threads=1`; parallel
> GPU tests can crash) — and `cargo check -p horizon-client --bin horizon-client
> --no-default-features` builds the game. No 1.8.9 server jar or `.local/`
> prepared assets were available, so a live join was not exercised here; the
> protocol, chunk, item and block formats are covered by byte vectors and unit
> tests.

## 0. Define "done" before touching code

Write down which of these the port must cover, because they have very different
costs:

- **Protocol only** — Horizon can ping, log in and exchange packets with a
  server of that version, but does not simulate it.
- **Client to a real server** — chunks, entities, inventory and UI render from
  that version's packets; single-player may still use the old world.
- **Single-player / integrated server** — the version's world generation,
  block behaviour, mobs, saves and assets are simulated locally.
- **Full replacement** — every mode, the launcher and the parity suite describe
  only the new version.

A full replacement is a rewrite of the target-specific layers, not a patch. Do
not advertise a version while any shipped path still consumes the old one.

## 1. Audit every place the version is pinned

Grep the tree for the old version, protocol number and data version, then list
the owners:

| Surface | What to inspect |
| --- | --- |
| Version constants | `crates/horizon-core/src/version.rs` (id, protocol, DataVersion, pack formats) |
| Protocol | `crates/horizon-protocol/` — ids, packet structs, framing, compression, encryption |
| Preparation | `crates/horizon-prepare/` — Java exporters compiled against the game jars |
| World data | `crates/horizon-world/` — NBT, regions, chunk format, `datafix/`, registry |
| Assets | asset index, blockstate/model/texture/sound formats, `horizon-assets` |
| Gameplay | `horizon-world/src/level/`, `horizon-client/src/mobs/`, game binary |
| Launcher / installer | `launcher/`, `official_install`, `research-lock.json`, bundled data |
| Tests / parity | `tools/test_*_parity.py`, Java oracles, fixtures, `tools/parity.py` |

Record the result in a short document. `docs/mc-1.8.9.md` is the example.

## 2. Record the exact metadata from Mojang

Fetch the official version JSON from `piston-meta.mojang.com` and copy these
values into a new constants module beside the old one (do **not** edit the old
constants yet):

- version id and protocol number,
- DataVersion (or "none"),
- resource-pack and data-pack format numbers,
- asset index id and SHA-1,
- client/server jar SHA-1 and sizes,
- required Java runtime.

For 1.8.9 the result is `crates/horizon-core/src/version_1_8_9.rs`. Keep hashes
and sizes, not just names: the installer and verification need them.

## 3. Build the protocol module additively

Create `crates/horizon-protocol/src/v<version>/` and register it from
`lib.rs`. Leave the old `packets/` module untouched. Use one file per state:

```
v1_8_9/
  mod.rs       constants, state machine, integration test
  ids.rs       packet ids, grouped by state and direction
  codec.rs     wire types that differ from the old target
  handshake.rs
  status.rs
  login.rs
  play.rs
```

### Packet enums

Reuse the existing `packet!` and `packet_enum!` macros. Every unmodeled id must
fall through to `Unknown(id, payload)` so a partial port can still read a live
connection without guessing. `decode` must reject trailing bytes.

### Codecs that usually change

Move these into `codec.rs` with byte-exact tests:

- **Packed block positions.** Bit order is version-specific. 1.8 is
  `x (26) | y (12) | z (26)`; 26.3 is `x (26) | z (26) | y (12)`. Never reuse
  the old `Pos` codec.
- **Item stacks.** Before 1.13 they are numeric id + count + damage + optional
  NBT; later versions are registry ids plus data components. An empty stack is
  usually id `-1` and no further fields.
- **UUIDs.** Some versions put a 36-character dashed string on the wire, others
  two big-endian longs.
- **Positions and angles.** Fixed-point `int / 32` entity coordinates and
  packed degree bytes appear in older versions. 1.8 entity metadata stores a
  position as three plain ints, not the packed block-position form.
- **Entity metadata.** 1.8 uses one header byte (`type << 5 | index`), the
  value, and a `0x7F` terminator. Type ids, the terminator and even the way
  positions are encoded changed in later versions. Model every value type and
  reject a missing terminator.
- **Optional NBT.** A leading `0` (TAG_End) means "no tag"; it is not a length.
- **Arrays.** Counts may be `VarInt` or fixed `Int`; check the version, not the
  neighboring packet.
- **One id, several shapes.** `Player Info` packs five actions, `Title` five,
  `Combat Event` three, `World Border` six, `Scoreboard Team` five, and
  `Open Window` gains a trailing entity id only for `EntityHorse`. Read the
  discriminator first and keep every field `Option`/enum-shaped; a writer that
  always emits the extra field desyncs the stream.
- **Presence can be implicit.** `Spawn Entity` writes its velocity only when
  `object_data != 0`, and `World Particles` writes trailing VarInts only for
  ids 36/37/38. Model the dependency (derive presence from the discriminator),
  never from an unrelated `Option` the caller set.
- **Window ids are not one type.** In 1.8 `Open Window` and `Window Items` use
  an unsigned byte, while `Set Slot` and `Confirm Transaction` use a signed
  byte. `Window Items` counts slots with an `i16`, not a `VarInt`.
- **Attribute ids are strings.** Before 1.16 each modifier carries a UUID
  (`u128`) and a `Double` amount; later versions changed the id and the payload.

### States that do not exist yet

Older versions have no configuration state and no login acknowledgement: login
success moves straight to play. Newer versions may add states. Model the actual
state machine; do not force it through the old one.

### Login details

- `Login Success` before 1.16 carries the UUID as a string.
- `Set Compression` is sent before `Login Success`; the codec switches
  immediately after that packet, not at the end of login.
- Online mode uses the same RSA/AES scheme, but the session endpoint and hash
  inputs differ by version. Keep the crypto shared and the driver version-aware.

## 4. Test bytes before wiring the game

A round trip through your own encoder proves nothing. Every modeled packet gets:

1. **A byte-exact vector** taken from a protocol reference or a real capture
   (handshake with the protocol number, login UUID form, a chunk header, a
   packed position, an empty slot).
2. **A decode test** for malformed input: trailing bytes, truncated fields,
   oversized strings/counts, negative lengths.
3. **An in-memory connection test** covering the real sequence: handshake →
   login → compression switch → join game → keep alive. `Pipe::pair()` in
   `v1_8_9/mod.rs` is the template.

Run only the new module while iterating:

```sh
cargo test -p horizon-protocol --lib v1_8_9
```

Then the whole crate and the core constants:

```sh
cargo test -p horizon-protocol --lib
cargo test -p horizon-core --lib
```

## 5. Write the full play set for the new version

Model packets by what the client must do, in this order:

1. join, respawn, difficulty, abilities, held item, health,
2. time, spawn position, chunk data/bulk, block change, multi-block change,
3. player movement/look, digging, placement, animation, entity action,
4. client settings, plugin messages, resource-pack status,
5. entity spawn/metadata/equipment/velocity/teleport,
6. windows, slots, inventory, crafting, scoreboard, teams, chat.

Keep the play state on the new ids for the connection's whole lifetime. Do not
translate packets through the old version's envelopes: field layouts and
semantics (for example 1.8's `id << 4 | meta` block states) are part of the
version, not an encoding detail.

Entity metadata and chunk section formats are the two largest version-specific
blobs. Port them as explicit tables/codecs with fixtures; do not infer them from
the old code.

## 6. World data, registries and saves

The existing datafixer upgrades old worlds **to** the current target. A new
target needs the opposite direction at the boundary:

- read the new version's chunk format directly (1.8: `Level` compound, 16³
  sections with `Blocks`/`Data`/`Add`, light nibbles, biomes, heightmap),
- keep the internal world model, but add a **new-version writer** so saves stay
  in the new format,
- add the new version's numeric block/item registry and map it to internal
  states; if the old target used namespaced ids, this mapping is bidirectional
  and must be tested both ways,
- preserve fields the module does not model in an `extra` compound, and preserve
  optional arrays as `Option`, so a load/save cycle does not silently drop
  `InhabitedTime`, `LightPopulated`, Nether `SkyLight` absence, or mod data,
- do not recompute the heightmap until its exact semantics are checked against
  the version; a wrong rule corrupts lighting and spawning,
- treat a missing `DataVersion` as the version's own value (1.8 reads as `-1`),
- never silently upgrade a new-version save to the old target's format.

Use synthetic chunks with known blocks, light and biomes, write them, read them
back out-of-process where possible, and compare NBT. Add a golden save produced
by the real game if the licence allows storing only the generated data (never
Mojang jars or assets).

## 7. Assets and preparation

`horizon-prepare` compiles Java exporters against the game jars; those classes
call version-specific APIs. For a new version either:

- port the exporters to that version's API, or
- replace the data they extract with a version-independent data source.

Download the matching asset index, verify its SHA-1 and size, and decode that
version's model/blockstate/texture/sound formats. Keep jars, extracted assets
and decompiled sources under `.local/`; `tools/check_public_tree.py` must stay
green. The launcher bundle and `research-lock.json` follow only after the
runtime can consume the data.

## 8. Gameplay and world generation

List the rules that differ before writing code. For 1.8.9 that includes:

- no attack cooldown; sword blocking instead of shields; no offhand,
- 1.8 hunger, regeneration, enchanting and potions,
- old mob AI, spawning and breeding rules,
- 1.8 redstone, pistons and block update order,
- the 1.8 terrain generator, biome selection, caves, features and structures.

Server-authoritative play can reuse the server's rules for many of these, but
client prediction and single-player cannot. Port the generator only after the
world format and registries are stable; it is the largest single stage.

## 9. Installer, launcher and packaging

Point `official_install` at the new version JSON and asset index, add the exact
libraries and hashes, and package the data bundle. The native client still needs
no Java runtime; a `jre-legacy` entry matters only to preparation tooling.
Keep the old installer path working until the new runtime is complete, then
remove it in one reviewable change.

## 10. Parity, docs and the switch

- Add oracles and harnesses for the new version. Existing harnesses describe
  the old target and must not be deleted until their replacements cover the
  same behaviour.
- Keep the byte-exact protocol tests; they are the cheapest regression net.
- Update the version constants **last**, after the game, assets, saves and
  installer consume only the new version.
- Run the full self-check before proposing the switch:

```sh
python3 tools/pr_ready.py
python3 tools/parity.py --changed origin/main
```

- Update `docs/mc-1.8.9.md` (or its equivalent), `docs/android-direction.md`,
  `docs/rust-parity.md` and the release notes. State which modes are supported
  and which are not.

## Worked example: 1.8.9

Files added so far:

| Path | Purpose |
| --- | --- |
| `crates/horizon-core/src/version_1_8_9.rs` | protocol 47, asset index, jar hash, no DataVersion |
| `crates/horizon-protocol/src/v1_8_9/ids.rs` | 1.8.9 packet ids |
| `.../codec.rs` | 1.8 packed position, numeric slot, multi-block records |
| `.../handshake.rs`, `status.rs`, `login.rs` | handshake/status/login |
| `.../play.rs` | join/world/player baseline, unknown ids preserved |
| `.../entity.rs` | 1.8 entity metadata, velocity, metadata value types |
| `.../player_info.rs` | 1.8 player list: add/update/remove actions |
| `.../play_extra.rs` | discriminator packets: scoreboard, teams, border, map, particles, attributes |
| `crates/horizon-world/src/chunk_1_8_9.rs` | 1.8 section/chunk NBT read and write |
| `docs/mc-1.8.9.md` | the staged plan and metadata |

Deliberate choices:

- `crates/horizon-core/src/version.rs` still holds the generated 26.3 table
  because the datafixer and the prepared resources use it;
  `target::RESOURCES_VERSION` names the folder the build reads, and only
  `target::current` names the game version. Keeping them apart is what lets the
  protocol flip before the asset pipeline does.
- The network boundary is translated (`Game::serverbound_1_8_9`) instead of
  rewriting every gameplay send site at once. Translate at one seam, never in
  both directions: inbound packets are decoded as protocol 47 and applied
  directly, not re-encoded as 26.3.
- Tests use byte-exact vectors and a compressed in-memory login/join sequence.
- Serverbound chat has only the message field; clientbound chat has the
  position byte. The asymmetry is version-specific and easy to get wrong.
- `Map Chunk Bulk` sends its per-chunk metadata before the concatenated section
  data; there is no data-length field in 1.8.
- 1.8's packed position puts y between x and z, unlike 26.3.
- The clientbound position packet carries a `flags` byte; the serverbound one
  carries `on_ground`. They are different structs.
- A 1.8 section is indexed `y << 8 | z << 4 | x`, stores the low block id byte
  in `Blocks` and the high nibble in optional `Add`, and packs light two blocks
  per byte (even index low nibble, odd index high nibble).
- The 1.8 **network** chunk and the 1.8 **saved** chunk are different formats.
  The wire payload is 4096 little-endian `u16` states per section
  (`id << 4 | metadata`, so no separate `Blocks`/`Data`/`Add`), laid out
  column-first: every section's states, then every section's block light, then
  every section's sky light, then biomes. `Map Chunk Bulk` concatenates each
  column's section blocks and puts all columns' biomes at the end.
- `Chunk1_8` keeps unknown `Level` fields and optional arrays instead of
  normalizing them, so loading and saving an unmodified chunk is lossless.
- Converting a 1.8 chunk to the current format is a **chunk-level** operation,
  not a state lookup: snowy grass, door halves, pumpkins, flower pots and snow
  layers are fixed from the blocks or tile entities around them. Run
  `update_chunk` over the whole column (see `Chunk1_8::to_current_nbt`) instead
  of mapping `id << 4 | meta` one block at a time.
- The current internal palette entry is `{id: "minecraft:stone", properties:
  {...}}` with lowercase keys, and section `Y` stays a byte. The datafixer's
  intermediate 1.13 palette uses `{Name, Properties}`; don't assert on that
  shape when testing the upgraded result.
- After the fixes a section palette may be padded with air entries to a power
  of two, and unused 1.8 metadata resolves to the block's first registered
  state. Check that the expected states are present, not the palette length.
- Numeric block ids stop at 197 in 1.8. The 1.12 table continues into blocks
  added by 1.9-1.12, so gate every lookup by the target version's highest id
  (id 198 is `end_rod` in 1.12 and must be air in 1.8).
- A single 1.8 block state can be upgraded without a chunk: take
  `BlockStateData.getTag(id << 4 | meta)`, then run the datafixer from 1.13
  (DataVersion 1631) to current over the block-state tag. That applies the
  post-flattening renames (`grass` to `short_grass`, `grass_path` to
  `dirt_path`) but not the context-dependent chunk fixes.
- Numeric item stacks should go through `Ty::ItemStack` from `-1` after
  rebuilding the 1.8 NBT shape (`id` Int, `Count` Byte, `Damage` Short,
  optional `tag`); the datafixer then does metadata flattening, wear and
  components in one pass.
- Entity ids are not uniformly `VarInt` in 1.8: `EntityVelocity` and
  `EntityMetadata` use one, while `EntityStatus` and `AttachEntity` use a plain
  `Int`. Copy the id type from the packet, not from a neighbouring packet.

## Checklist

- [ ] metadata recorded with hashes, sizes and Java requirement
- [ ] version constants added beside the old ones, old ones untouched
- [ ] protocol module registered additively, unknown ids preserved
- [ ] byte-exact vectors for every modeled packet
- [ ] in-memory handshake/login/join test
- [ ] full play set modeled and tested
- [ ] chunk reader and writer for the new format, round-trip tested
- [ ] numeric registry mapping tested in both directions
- [ ] assets and prepare pipeline for the new version
- [ ] gameplay rules and world generation ported
- [ ] installer, launcher and data bundle switched
- [ ] new parity oracles replace the old ones
- [ ] version constants flipped last
- [ ] docs and release notes match what actually works

## Pitfalls seen while doing this

- A struct name shared by clientbound and serverbound packets silently binds to
  the wrong layout; name them `Client...`/`Server...` when the fields differ.
- Float bit patterns are exact in tests; use `f32`/`f64` values that round-trip
  cleanly or compare decoded values.
- Counts read from the network must be bounds-checked before allocating.
- Do not derive `Default` for a struct containing a non-`Default` NBT tag; write
  an explicit empty constructor.
- Do not reuse the old target's packed-position or item-stack codecs "because
  they look the same". They are not.
- Chunk section arrays have exact lengths; reject a `Blocks` array that is not
  4096 bytes instead of padding it. Duplicate section `Y` values are corruption,
  not something to merge.
- The 1.8 heightmap is easy to get subtly wrong. Preserve it on round trip and
  add a separately tested recalculation only after checking the version's rule.
- Keep Mojang jars, assets, decompiled sources and generated worlds out of Git.
