# Syncing `origin/android` into the 1.8.9 branch

This repository is the Android fork of Horizon. The `android` branch here is a
**Minecraft 1.8.9 build**: the network protocol, item and block ids, chunk
format, combat rules and many HUD screens are 1.8.9, not 26.3. `origin/android`
is the fork's own upstream (Android platform work, renderer optimisations,
accounts, input fixes). This guide is the repeatable way to bring new
`origin/android` commits into the 1.8.9 branch without losing 1.8.9 semantics.

The port is a **replay plus adaptation**, not a plain merge. Most commits apply
cleanly; the ones that touch version constants, networking, items, blocks,
chunks, saves, combat or sounds need the rules in
[Conflict rules](#conflict-rules).

Related documents:

- [mc-1.8.9.md](mc-1.8.9.md) — what the 1.8.9 build covers and what is left.
- [PORTING.md](../PORTING.md) — how to port another Minecraft version.
- [android-upstream.json](android-upstream.json) — the recorded sync point.
- [parity-workflow.md](parity-workflow.md) — harnesses and CI scopes.

## 0. Before you touch anything

1. **Commit the 1.8.9 work.** The replay cannot run on a dirty tree.
   ```sh
   git status --short          # must be empty after the commit
   git add -A
   git commit -m "Support Minecraft 1.8.9 (protocol 47) in the client"
   ```
2. **Make a rollback branch.**
   ```sh
   git branch backup-189 HEAD
   ```
3. **Record the current tip** in `docs/android-upstream.json`
   (`synced_origin_commit`). If the field is missing, add it before syncing.
4. **Stop other cargo processes.** This environment can have as little as
   1–2 GiB free; a parallel `cargo test` plus the replay build can OOM the
   whole session. Use `-j 1` for tests and never run two cargo commands at
   once.

## 1. Fetch and measure the gap

```sh
git fetch --no-tags origin android
git rev-list --count HEAD..origin/android
git log --oneline HEAD..origin/android
git diff --stat HEAD origin/android
```

- `HEAD..origin/android` is the list to review when the local 1.8.9 work is a
  single commit on top of the last sync point.
- If the histories diverged, compare `git merge-base HEAD origin/android`
  with the recorded sync point and review from the merge base instead.

`tools/android_sync.py` prints the same list grouped by risk:

```sh
python3 tools/android_sync.py                 # uses the recorded sync point
python3 tools/android_sync.py --fetch         # fetch first
python3 tools/android_sync.py --since <sha>   # explicit base
```

## 2. Triage every commit

Read the messages, then open the diff for anything marked HIGH. Do not replay
blindly: a commit that is correct for 26.3 can be wrong for 1.8.9 even when it
merges cleanly.

| Area | Typical files | Action |
|---|---|---|
| Android platform, launcher, accounts, wardrobe, IME/SAF, CI | `crates/horizon-client/src/android/`, `launcher/`, `packaging/android/`, `.github/` | Replay as-is; they do not speak Minecraft packets. Check that new UI does not assume 26.3 capabilities (offhand, data packs, configuration state). |
| Renderer, terrain, shaders, LOD | `crates/horizon-client-render/src/gfx/`, `*.wgsl`, `mesher`, `lod` | Replay. Keep `target::RESOURCES_VERSION` for resource paths and the 1.8 texture fallbacks (`sprites.rs`, `chunk_1_8_9.rs`). |
| Gameplay, entities, mobs, blocks, items | `crates/horizon-client/src/game/`, `mobs/`, `horizon-items`, `horizon-world` | Replay and audit every version assumption. Numeric 1.8 ids and the 1.8 metadata handlers win over modern registry data. |
| Networking, login, protocol | `crates/horizon-client/src/net.rs`, `lan/`, `horizon-protocol` | HIGH. 1.8 has no configuration state, numbered packet ids, classic named-root NBT, `i8` angles and numeric slots. Never import 26.3 packet shapes into the 47 path. |
| Version constants, datafixer, saves | `crates/horizon-core/src/version*`, `target.rs`, `horizon-world/src/datafix/`, `save.rs` | HIGH. `target.rs` is the single switch; `version_1_8_9.rs` must stay the current version. Do not take an upstream `version.rs` wholesale. |
| Combat, offhand, sounds, particles | `game/combat.rs`, `game/sound*`, `game/remote*`, `particles.rs` | HIGH. 1.8 has no attack cooldown, no sweep, no offhand, different sound names and particle ids. Keep the `since_1_9()` / `is_1_8_9()` gates. |
| Docs, tests, benchmarks | `docs/`, `tests/`, `tools/`, `.md` | Replay. Update counts and commands if the build changed. |

A useful first pass over the merged result:

```sh
git diff --stat backup-189..HEAD          # what the replay actually changed
rg -n "since_1_9|is_1_8_9" crates         # every 1.8 branch that must survive
rg -n "PROTOCOL_VERSION" crates           # must point at target::PROTOCOL_VERSION
```

## 3. Replay onto `origin/android`

The clean history shape is a linear branch: `origin/android` first, then the
1.8.9 commit(s). In this repository `git merge` and `git rebase` can fail with
`fatal: stash failed` (a git 2.53 interaction with the worktree); the forced
checkout plus cherry-pick is reliable and has the same result:

```sh
git checkout -f --detach origin/android
git cherry-pick <1.8.9-tip>        # e.g. the commit recorded as backup-189
# resolve conflicts (see below)
git branch -f android HEAD
git checkout android
```

Why `-f`: git can report phantom local changes for files that are identical
in the index and the worktree, and then refuse the checkout. Verify the tree is
really clean first:

```sh
git diff-files --name-status        # empty
git diff-index --name-status HEAD   # empty
```

Use `-f` only after both are empty. `backup-189` is the safety net.

If the replay is several 1.8.9 commits, cherry-pick them in order. Keep the
1.8.9 commits on top; never rewrite upstream commits.

### If a conflict is not worth resolving

A commit that only makes sense for 26.3 (a new configuration-state packet, a
data-pack registry, an offhand feature, a 26.3-only save field) can be skipped:

```sh
git cherry-pick --skip
```

Record the skipped commit and the reason in `docs/android-upstream.json`. Do
not silently drop behavior: either gate it for 1.8.9 or write down why it does
not apply.

## Conflict rules

These are invariants of the 1.8.9 build. When a conflict touches one of them,
the 1.8.9 side wins and the upstream code is adapted around it.

1. **One version switch.** `horizon_core::target` re-exports
   `version_1_8_9` as `current`. `PROTOCOL_VERSION` is 47 and
   `target::DATA_VERSION` is `-1`. `horizon_core::version` remains the 26.3
   table used by the datafixer.
2. **Resources are still 26.3.** `target::RESOURCES_VERSION` is
   `version::VANILLA_VERSION` ("26.3") until the 1.8 asset pipeline exists.
   Resource paths use `RESOURCES_VERSION`; the displayed game version uses
   `VANILLA_VERSION` ("1.8.9").
3. **No configuration state.** 1.8 login goes handshake → login → play.
   `SetCompression` is login-only; there is no login acknowledgement and no
   registry/data-pack payload.
4. **Numeric ids at the wire.** Blocks are `id << 4 | meta` via
   `horizon-world::legacy_1_8_9`; items are numeric via
   `horizon-items::legacy_1_8_9`. Modern registry ids never go into 1.8
   packets. 1.8 NBT is the classic named root; `horizon-protocol`'s
   `NbtClassic` is the codec.
5. **1.8 units.** Angles are `i8 * 360 / 256`, positions `i32 / 32`,
   velocity `i16 / 8000`, sound pitch `u8 / 63`, sound coordinates `i32 / 8`.
6. **No offhand.** `Kind::Inventory` has 45 slots in 1.8 (`slot_count()`), the
   recipe book is hidden, and horse inventory maps to the mount menu. Upstream
   offhand code must be behind `since_1_9()` or removed from the 1.8 path.
7. **1.8 combat.** No attack cooldown, sweep, sword-swap scale or modern
   knockback; crit and armour use the 1.8 formulas; sword blocking is 1.8;
   the attack indicator stays off; attribute tooltips are gated.
8. **1.8 sounds and particles.** `game/sound_1_8_9.rs` maps 1.8 names to
   modern events; particle ids follow `EnumParticleTypes` order and the
   argument particles (reddust, mobSpell, note, item/block cracks, barrier)
   consume their data as 1.8 defines it.
9. **Saves stay 26.3-shaped for now.** 1.8 chunks are read and converted;
   writes still use the 26.3 writer (`save::VERSION_NAME` pins the 26.3
   version). A 1.8 save writer is a separate port; do not half-change it in a
   sync.
10. **LAN and the dedicated server still speak the internal 26.3 protocol.**
    Single player runs through the in-memory server thread, so a sync must not
    change that boundary. The shared `PROTOCOL_VERSION` constant now says 47;
    treat LAN/dedicated guests as documented debt until their outbound
    translation exists.

## 4. Verify

Run the checks in this order, one cargo command at a time. Stop at the first
failure; fix the replay, do not "fix" the tests.

```sh
cargo fmt --all -- --check
cargo check -j 1 -p horizon-client --bin horizon-client --no-default-features
cargo test  -j 1 -p horizon-protocol -p horizon-core -p horizon-items -p horizon-world
cargo test  -j 1 -p horizon-client --lib --no-default-features
cargo test  -j 1 -p horizon-client --bins --no-default-features
cargo test  -j 1 -p horizon-client --tests --no-default-features
cargo test  -j 1 -p horizon-client-render --lib -- --test-threads=1
```

`horizon-client-render` tests must run serially (parallel GPU tests can crash).
The four small crates first catch protocol/registry mistakes before the long
client build.

For networking changes, run the live 1.8.9 probe (it needs a real 1.8.9
server; the jar and files stay under `.local/`, never committed):

```sh
# online-mode=false, network-compression-threshold=-1, HorizonProbe opped
java -jar .local/minecraft/1.8.9/server.jar nogui
cargo run -p horizon-protocol --example live_1_8_9 -- 127.0.0.1:25599
```

Expected: `all packets decoded, every required packet arrived` with 45+
packet kinds, including `MapChunkBulk`, `MapChunk`, `MultiBlockChange`,
`Explosion`, `Particles`, `SoundEffect`, `Title`, `DisplayObjective`,
`TabCompleteResponse`, `OpenSignEditor` and `UpdateSign`. The probe places a
sign and submits its text through the real editor flow. `TileEntityData` is
not required: 1.8.9 sends sign updates as `UpdateSign`, and `TileEntityData`
only covers non-sign block entities when their block is updated after the
entity exists.

Android platform changes additionally need the Android build/checks used by
the fork (ARM64 Clippy and the targeted input/content/account/GPU tests).
Do not claim emulator FPS as phone FPS.

## 5. Record the sync

Update `docs/android-upstream.json`:

- `synced_origin_commit` — the `origin/android` tip that was replayed.
- `synced_origin_date` — the date of the sync.
- `synced_origin_commits` / `synced_origin_pending` — how many commits the
  replay covered and how many were left unreviewed (0 after a complete pass).
- `synced_local_commit` — the replay commit on this branch, for the next
  agent's starting point.
- `fork_sync_review` — one paragraph: what was integrated, conflicts and
  decisions, what was skipped and why, and which checks ran.
- `merged_upstream_commit` / `latest_fetched_commit` — keep the upstream
  (ludovic) nightly facts accurate; do not overwrite them with fork commit
  ids. The fork sync lives in the `synced_origin_*` fields.
- `pending_review_commits` — the upstream-nightly count; the fork's own count
  is `synced_origin_pending`.

If the schema changes, update this guide with it.

## 6. Rollback

```sh
git checkout android
git reset --hard backup-189
```

Then re-do the replay from a clean tree. Never force-push a shared branch; if
the sync was already pushed, revert the merge/replay commit instead.

## Checklist

- [ ] Tree committed, `backup-189` created, no cargo running.
- [ ] `git fetch origin android`, gap counted, commits read.
- [ ] High-risk commits opened and adapted, not just merged.
- [ ] Replay applied on `origin/android`; conflicts resolved by the rules.
- [ ] `fmt`, check, crate tests, client lib/bins/tests, render (serial) green.
- [ ] Live 1.8.9 probe green for networking changes.
- [ ] `docs/android-upstream.json` updated with commit, date, review.
- [ ] Skipped commits and remaining debt written down, not hidden.
