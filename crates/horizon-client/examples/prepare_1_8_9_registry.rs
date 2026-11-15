//! Build a 1.8.9 block registry and model-state table from the 1.8.9 client jar.
//!
//! This is the first half of stage 4 of `docs/mc-1.8.9.md`: it writes
//! `facts.json` (the block registry the engine loads), `model-states.json`
//! (the schema and state order the model binder checks) and copies the 1.8.9
//! jar's `assets/` over them. Both tables are keyed by the names the engine
//! uses (`minecraft:oak_log`, `minecraft:white_wool`), not by the pre-1.13
//! numeric names, so the rest of the engine can read them unchanged.
//!
//! The state list comes from the 1.8 numeric table: for every id and metadata
//! the 1.8 runtime registers, `legacy_1_8_9::block_state` gives the current
//! name and properties. A block gets a model state only when the 1.8 jar has
//! a blockstate file with that exact name. The pre-flattening families whose
//! files are split by variant (`anvil` has three damage models, `oak_slab`
//! separates its double form, the terracotta colours live under the old
//! `*_stained_hardened_clay` names) are reported in `missing-models.json`.
//! Binding them needs a hand-written state-to-file map: the 1.8 client keeps
//! that map in its (obfuscated) block classes, not in the asset files.
//!
//! It also projects the 26.3 worldgen bundle's per-state block tables (block
//! properties, cached shapes, structure transforms, block flags) and its
//! block-name tables onto the 1.8 registry, because those tables check their
//! state identities against it. The rest of the worldgen data is copied from
//! the 26.3 export unchanged: single-player world generation is still
//! 26.3-shaped, which is recorded as debt in `docs/mc-1.8.9.md`.
//!
//! It does not write the behaviour, gameplay, item or other world-generation
//! tables. Mojang's jar stays under `.local/`; nothing this writes is committed.
//!
//! ```sh
//! cargo run -p horizon-client --example prepare_1_8_9_registry -- \
//!     --client-jar .local/minecraft/1.8.9/client.jar \
//!     --tags .local/horizon-client/26.3/block-tags.json \
//!     --output .local/horizon-client/1.8.9
//! ```

use horizon_world::{
    chunk::BlockState,
    datafix::flat_tables::BLOCK_IDS,
    legacy_1_8_9::{MAX_BLOCK_ID, block_state, is_registered},
    nbt::{self, Compound, JavaString, Limits, NamedTag, Tag},
};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// The 1.8 `flower_pot` blockstate's `contents=` values and the modern block
/// each one flattens to. 1.8 has one block whose contents live in its tile
/// entity; the engine has one block per content.
const FLOWER_POT_CONTENTS: [(&str, &str); 22] = [
    ("empty", "flower_pot"),
    ("rose", "potted_poppy"),
    ("blue_orchid", "potted_blue_orchid"),
    ("allium", "potted_allium"),
    ("houstonia", "potted_azure_bluet"),
    ("red_tulip", "potted_red_tulip"),
    ("orange_tulip", "potted_orange_tulip"),
    ("white_tulip", "potted_white_tulip"),
    ("pink_tulip", "potted_pink_tulip"),
    ("oxeye_daisy", "potted_oxeye_daisy"),
    ("dandelion", "potted_dandelion"),
    ("oak_sapling", "potted_oak_sapling"),
    ("spruce_sapling", "potted_spruce_sapling"),
    ("birch_sapling", "potted_birch_sapling"),
    ("jungle_sapling", "potted_jungle_sapling"),
    ("acacia_sapling", "potted_acacia_sapling"),
    ("dark_oak_sapling", "potted_dark_oak_sapling"),
    ("mushroom_brown", "potted_brown_mushroom"),
    ("mushroom_red", "potted_red_mushroom"),
    ("dead_bush", "potted_dead_bush"),
    ("fern", "potted_fern"),
    ("cactus", "potted_cactus"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client_jar = PathBuf::from(".local/minecraft/1.8.9/client.jar");
    let mut tags = PathBuf::from(".local/horizon-client/26.3/block-tags.json");
    let mut output = PathBuf::from(".local/horizon-client/1.8.9");
    let mut source = PathBuf::from(".local/horizon-client/26.3");
    let mut copy_assets = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--client-jar" => client_jar = args.next().ok_or("--client-jar needs a path")?.into(),
            "--tags" => tags = args.next().ok_or("--tags needs a path")?.into(),
            "--output" => output = args.next().ok_or("--output needs a path")?.into(),
            "--source" => source = args.next().ok_or("--source needs a path")?.into(),
            "--no-assets" => copy_assets = false,
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    if source == output {
        return Err("--source and --output must differ: the tables are rewritten in place".into());
    }
    let mut archive = zip::ZipArchive::new(fs::File::open(&client_jar)?)?;
    let mut blockstates = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let raw = entry.name()?.to_owned();
        let Some(name) = raw.strip_prefix("assets/minecraft/blockstates/") else {
            continue;
        };
        let Some(name) = name.strip_suffix(".json") else {
            continue;
        };
        let name = name.to_owned();
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        blockstates.insert(name, serde_json::from_str::<Value>(&text)?);
    }

    // Every state the 1.8 runtime registers, grouped by the current name the
    // engine knows. `block_state` has already applied the datafixer's renames.
    let mut by_name: BTreeMap<String, Vec<(Map<String, Value>, bool)>> = BTreeMap::new();
    for id in 0..=MAX_BLOCK_ID {
        if !BLOCK_IDS
            .iter()
            .any(|&(_, block_id)| block_id == i32::from(id))
        {
            continue;
        }
        for meta in 0..16u8 {
            if !is_registered(id, meta) {
                continue;
            }
            let state = block_state(id, meta);
            // The datafixer uses `%%FILTER_ME%%` for states that have no
            // block; it is not a 1.8 block.
            if !state.name.starts_with("minecraft:") {
                continue;
            }
            let fluid = matches!(state.name.as_str(), "minecraft:water" | "minecraft:lava");
            let properties = state
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            let states = by_name.entry(state.name).or_default();
            // The numeric table names several ids for the same state (the
            // flowing and still fluids, for example); keep one.
            if !states.iter().any(|(seen, _)| *seen == properties) {
                states.push((properties, fluid));
            }
        }
    }

    // 1.8's flower pot contents live in the block entity, not metadata. The
    // context-free datafixer can only pick one content for the bare id; use
    // the empty pot and every content the tile entity can hold instead, so
    // each gets its own model and gameplay row.
    by_name.remove("minecraft:potted_cactus");
    for (_, name) in FLOWER_POT_CONTENTS {
        by_name.insert(format!("minecraft:{name}"), vec![(Map::new(), false)]);
    }

    // 1.8 has one skull block (id 144) whose type lives in the tile entity;
    // 1.13 flattened it into a block per type. The datafixer rows for 144 are
    // `%%FILTER_ME%%`, so the loop above skipped them. Seed one state per name
    // and let `expand_states` fill the modern property domains; the tile
    // entity converter picks the typed state per placed skull.
    for (name, wall) in horizon_world::legacy_1_8_9::SKULLS {
        let mut properties = Map::new();
        if wall {
            properties.insert("facing".into(), json!("north"));
        } else {
            properties.insert("rotation".into(), json!("0"));
        }
        properties.insert("powered".into(), json!("false"));
        by_name.insert(name.to_owned(), vec![(properties, false)]);
    }

    // 1.8 metadata does not record every modern property combination, but the
    // engine's state table assumes vanilla's full product. Expand before the
    // model definitions, so the derived files bind the expanded states too.
    let source_tables = load_source(&source)?;
    for (name, states) in by_name.iter_mut() {
        *states = expand_states(std::mem::take(states), &source_tables, name);
    }

    // The 1.8 jar splits or renamed several families' blockstate files. These
    // definitions speak the engine's state vocabulary (modern property names
    // and value sets) and are written over the copied assets.
    let derived = derive_definitions(&blockstates, &by_name);
    let mut facts = Vec::new();
    let mut defaults = Map::new();
    let mut model_states = Map::new();
    let mut registry: BTreeSet<String> = BTreeSet::new();
    let mut missing_models = Vec::new();
    // The same states in `facts.json` order: the rows of every per-state table
    // the engine loads are written in this order.
    //
    // The engine's empty state is `BlockStateId(0)`: an unset palette entry, an
    // empty section, a chunk outside the resident world and a height outside
    // the column all read back as state 0. 26.3's registry starts with air for
    // that reason; 1.8's name-grouped table would start with `acacia_bark`, so
    // emit air first and keep the rest alphabetical.
    let mut registry_states: Vec<BlockState> = Vec::new();
    let mut ordered = Vec::with_capacity(by_name.len());
    if let Some(states) = by_name.remove("minecraft:air") {
        ordered.push(("minecraft:air".to_owned(), states));
    }
    ordered.extend(by_name);
    for (name, states) in ordered {
        let path = name.strip_prefix("minecraft:").unwrap_or(&name);
        let definition = derived.get(path).or_else(|| blockstates.get(path));
        let schema = definition.map(schema_of).unwrap_or_default();
        if definition.is_none() {
            missing_models.push(name.clone());
        }
        // The default state is the 26.3 one where this block exists there;
        // the first state otherwise. The state order alone does not say which
        // value of each property vanilla treats as the default. 1.8 states
        // omit properties 1.8 did not have (`waterlogged`, `snowy`), so match
        // the 26.3 default on the properties this block does carry: without
        // that, a smooth stone slab's default became its first state (the top
        // slab) instead of 26.3's (and 1.8 metadata 0's) bottom slab.
        let target = source_name(&name).unwrap_or_else(|| name.clone());
        let wanted = source_tables
            .defaults
            .get(&target)
            .or_else(|| source_tables.defaults.get(&name))
            .map(state_properties);
        let default_index = facts.len()
            + wanted
                .as_ref()
                .and_then(|wanted| {
                    states
                        .iter()
                        .position(|(properties, _)| properties == wanted)
                        .or_else(|| {
                            states.iter().position(|(properties, _)| {
                                properties
                                    .iter()
                                    .all(|(key, value)| wanted.get(key) == Some(value))
                            })
                        })
                })
                .unwrap_or(0);
        defaults.insert(name.clone(), Value::from(default_index));
        registry.insert(name.clone());
        let mut possible = Vec::with_capacity(states.len());
        for (properties, fluid) in states {
            // `facts.json` stores a saved block state: its id plus the
            // properties it overrides (the registry file omits empty ones).
            let mut descriptor = Map::new();
            descriptor.insert("id".into(), Value::String(name.clone()));
            if !properties.is_empty() {
                descriptor.insert("properties".into(), Value::Object(properties.clone()));
            }
            facts.push(json!([
                name.as_str(),
                name == "minecraft:air",
                fluid,
                format!("Block{{{name}}}"),
                Value::Object(descriptor),
            ]));
            registry_states.push(BlockState {
                name: name.clone(),
                properties: properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                    .collect(),
            });
            possible.push(Value::Object(properties));
        }
        model_states.insert(
            name.clone(),
            json!({
                "schema": schema_value(&schema),
                "states": possible,
            }),
        );
    }
    fs::create_dir_all(&output)?;
    write_json(
        &output.join("facts.json"),
        &json!({"states": facts, "defaults": defaults}),
    )?;
    write_json(
        &output.join("model-states.json"),
        &Value::Object(model_states),
    )?;
    write_json(&output.join("missing-models.json"), &json!(missing_models))?;
    write_json(
        &output.join("horizon-resources.json"),
        &json!({
            "schema": 7,
            "minecraft": "1.8.9",
            "data_version": horizon_world::DATA_VERSION,
            "resource_files": 0,
        }),
    )?;
    let tag_bundle: Value = serde_json::from_slice(&fs::read(&tags)?)?;
    let tag_object = tag_bundle.as_object().ok_or("invalid block tag bundle")?;
    // `#tag` entries are references to other tags, not block names: keep them
    // when the referenced tag is in the bundle, and plain names when the 1.8
    // registry has the block.
    let tag_names: BTreeSet<&str> = tag_object.keys().map(String::as_str).collect();
    let mut filtered_tags = Map::new();
    for (tag, text) in tag_object {
        let parsed: Value = serde_json::from_str(text.as_str().ok_or("tag is not text")?)?;
        let entries = parsed["values"].as_array().ok_or("tag has no values")?;
        let kept: BTreeSet<String> = entries
            .iter()
            .filter_map(Value::as_str)
            .filter(|entry| match entry.strip_prefix('#') {
                Some(reference) => tag_names.contains(reference),
                None => registry.contains(*entry),
            })
            .map(str::to_owned)
            .collect();
        let mut out = Map::new();
        if let Some(replace) = parsed.get("replace") {
            out.insert("replace".into(), replace.clone());
        }
        out.insert(
            "values".into(),
            Value::Array(kept.into_iter().map(Value::String).collect()),
        );
        filtered_tags.insert(tag.clone(), Value::String(serde_json::to_string(&out)?));
    }
    write_json(
        &output.join("block-tags.json"),
        &Value::Object(filtered_tags),
    )?;
    // The engine's other per-state tables still come from the current-version
    // export in `--source`: project each 1.8 state onto the table row of the
    // matching source state, so their state identities line up with this
    // registry. Blocks 1.8 renamed keep their 26.3 counterpart's facts.
    let mut block_names: Vec<String> = Vec::new();
    for state in &registry_states {
        if block_names.last() != Some(&state.name) {
            block_names.push(state.name.clone());
        }
    }
    let (state_map, exact, fallback, unknown) = map_source_states(&registry_states, &source_tables);
    project_gameplay(&output, &source_tables, &state_map)?;
    project_behaviours(&output, &source_tables, &state_map, &block_names)?;
    project_landing(&source, &output, &block_names)?;
    project_render(&source, &output, &registry_states, &block_names)?;
    for file in [
        "cached-collision.nbt",
        "occlusion.nbt",
        "movement-properties.nbt",
    ] {
        project_table(
            &source,
            &output,
            file,
            &state_map,
            &registry_states,
            &block_names,
        )?;
    }
    project_worldgen(&source, &output, &state_map, &registry_states, &block_names)?;
    if copy_assets {
        copy_client_assets(&mut archive, &output)?;
        // 1.8 draws skulls in the tile entity renderer and ships no block
        // model; the engine still needs one for their breaking particles.
        let model_dir = output.join("assets/minecraft/models/block");
        fs::create_dir_all(&model_dir)?;
        write_json(
            &model_dir.join("skull.json"),
            &json!({"textures": {"particle": "minecraft:block/soul_sand"}}),
        )?;
    }
    // After the jar's own files: a derived definition replaces the 1.8 file
    // whose name the engine reuses with a different meaning (`tall_grass`).
    let blockstate_dir = output.join("assets/minecraft/blockstates");
    fs::create_dir_all(&blockstate_dir)?;
    for (name, definition) in &derived {
        write_json(&blockstate_dir.join(format!("{name}.json")), definition)?;
    }
    println!(
        "{} blocks, {} states, {} blocks without a 1.8 blockstate -> {} \
         (mapped {} states: {exact} exact, {fallback} block default, {unknown} unknown)",
        registry.len(),
        facts_len(&output)?,
        missing_models.len(),
        output.display(),
        state_map.len(),
    );
    Ok(())
}

/// The tables the engine still takes from the current-version export.
struct SourceTables {
    states: Vec<BlockState>,
    /// The 26.3 default state of every block, for the same property defaults.
    defaults: BTreeMap<String, BlockState>,
    /// The 26.3 property values in vanilla's order, which is the order the
    /// engine's state table expects.
    values: BTreeMap<(String, String), Vec<String>>,
    gameplay: Vec<Value>,
    behaviour_states: Vec<Value>,
    behaviour_blocks: BTreeMap<String, Value>,
    behaviours: Value,
}

fn load_source(root: &Path) -> Result<SourceTables, Box<dyn std::error::Error>> {
    let facts: Value = serde_json::from_slice(&fs::read(root.join("facts.json"))?)?;
    let states = facts["states"]
        .as_array()
        .ok_or("source facts have no states")?
        .iter()
        .map(|row| -> Result<BlockState, Box<dyn std::error::Error>> {
            let descriptor = &row[4];
            let name = descriptor["id"]
                .as_str()
                .ok_or("source facts state has no id")?;
            let properties = descriptor
                .get("properties")
                .and_then(Value::as_object)
                .map(|properties| {
                    properties
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                        .collect()
                })
                .unwrap_or_default();
            Ok(BlockState {
                name: name.to_owned(),
                properties,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let defaults = facts["defaults"]
        .as_object()
        .ok_or("source facts have no defaults")?
        .iter()
        .filter_map(|(name, index)| {
            let state = states.get(index.as_u64()? as usize)?;
            Some((name.clone(), state.clone()))
        })
        .collect();
    let mut values: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for state in &states {
        for (property, value) in &state.properties {
            let entry = values
                .entry((state.name.clone(), property.clone()))
                .or_default();
            if !entry.contains(value) {
                entry.push(value.clone());
            }
        }
    }
    let gameplay: Value = serde_json::from_slice(&fs::read(root.join("gameplay-facts.json"))?)?;
    let gameplay = gameplay["states"]
        .as_array()
        .ok_or("source gameplay facts have no states")?
        .clone();
    let behaviours: Value = serde_json::from_slice(&fs::read(root.join("block-behaviours.json"))?)?;
    let behaviour_states = behaviours["states"]
        .as_array()
        .ok_or("source behaviours have no states")?
        .clone();
    let behaviour_blocks = behaviours["blocks"]
        .as_array()
        .ok_or("source behaviours have no blocks")?
        .iter()
        .filter_map(|b| Some((b["name"].as_str()?.to_owned(), b.clone())))
        .collect();
    Ok(SourceTables {
        states,
        defaults,
        values,
        gameplay,
        behaviour_states,
        behaviour_blocks,
        behaviours,
    })
}

/// The current-version name whose table rows a 1.8 name takes.
fn source_name(name: &str) -> Option<String> {
    let path = name.strip_prefix("minecraft:")?;
    if let Some(stem) = path.strip_suffix("_bark") {
        return Some(format!("minecraft:{stem}_wood"));
    }
    Some(
        match path {
            "melon_block" => "minecraft:melon",
            "mob_spawner" => "minecraft:spawner",
            "portal" => "minecraft:nether_portal",
            "pumpkin" => "minecraft:carved_pumpkin",
            _ => return None,
        }
        .to_owned(),
    )
}

fn state_key(state: &BlockState) -> String {
    state
        .properties
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// `(source index, exact, fallback, unknown)` per registry state.
fn map_source_states(
    registry: &[BlockState],
    source: &SourceTables,
) -> (Vec<usize>, usize, usize, usize) {
    let mut exact: HashMap<(String, String), usize> = HashMap::new();
    let mut default: HashMap<String, usize> = HashMap::new();
    for (i, state) in source.states.iter().enumerate() {
        exact
            .entry((state.name.clone(), state_key(state)))
            .or_insert(i);
        default.entry(state.name.clone()).or_insert(i);
    }
    let mut map = Vec::with_capacity(registry.len());
    let (mut hits, mut fallbacks, mut unknown) = (0, 0, 0);
    for state in registry {
        let target = source_name(&state.name).unwrap_or_else(|| state.name.clone());
        if let Some(index) = exact.get(&(target.clone(), state_key(state))) {
            map.push(*index);
            hits += 1;
            continue;
        }
        // 1.8 distinguishes fewer properties than the current state: a slab
        // has no `waterlogged`, a fence no `waterlogged`, a grass block no
        // `snowy`. Fill those from the current block's default state and
        // look again before falling back to the block's default state, which
        // would give every 1.8 slab the modern default's *top* shape.
        let filled = source
            .defaults
            .get(&target)
            .or_else(|| source.defaults.get(state.name.as_str()));
        if let Some(index) = filled
            .and_then(|default| expanded_state_key(state, default))
            .and_then(|key| exact.get(&(target.clone(), key)))
        {
            map.push(*index);
            fallbacks += 1;
            continue;
        }
        if let Some(index) = default.get(&target).or_else(|| default.get(&state.name)) {
            map.push(*index);
            fallbacks += 1;
        } else {
            map.push(0);
            unknown += 1;
        }
    }
    (map, hits, fallbacks, unknown)
}

/// [`state_key`] for a 1.8 state with the properties it does not know filled
/// from the current block's default state. None when the 1.8 state has a
/// property the current block does not define under that name.
fn expanded_state_key(state: &BlockState, default: &BlockState) -> Option<String> {
    let mut properties = default.properties.clone();
    for (property, value) in &state.properties {
        if !properties.contains_key(property) {
            return None;
        }
        properties.insert(property.clone(), value.clone());
    }
    Some(
        properties
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(";"),
    )
}

fn project_gameplay(
    output: &Path,
    source: &SourceTables,
    map: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut states = Vec::with_capacity(map.len());
    for (index, &source_index) in map.iter().enumerate() {
        let mut entry = source.gameplay[source_index].clone();
        entry["id"] = json!(index);
        states.push(entry);
    }
    write_json(
        &output.join("gameplay-facts.json"),
        &json!({"states": states}),
    )
}

fn project_behaviours(
    output: &Path,
    source: &SourceTables,
    map: &[usize],
    block_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let states: Vec<Value> = map
        .iter()
        .map(|&index| source.behaviour_states[index].clone())
        .collect();
    let blocks: Vec<Value> = block_names
        .iter()
        .map(|name| {
            let target = source_name(name).unwrap_or_else(|| name.clone());
            let mut entry = source
                .behaviour_blocks
                .get(&target)
                .or_else(|| source.behaviour_blocks.get(name))
                .cloned()
                .unwrap_or_else(|| {
                    json!({
                        "name": name,
                        "class": [],
                        "interfaces": [],
                        "fields": {},
                        "dynamic_shape": false,
                        "item": name,
                    })
                });
            entry["name"] = json!(name);
            entry
        })
        .collect();
    let mut behaviours = source.behaviours.clone();
    behaviours["states"] = Value::Array(states);
    behaviours["blocks"] = Value::Array(blocks);
    write_json(&output.join("block-behaviours.json"), &behaviours)
}

/// Keep the current-version landing families of the 1.8 blocks; a block with
/// no current counterpart keeps its registry entry without a landing family
/// (the default `Block` family applies).
fn project_landing(
    source: &Path,
    output: &Path,
    block_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let raw: BTreeMap<String, Value> =
        serde_json::from_slice(&fs::read(source.join("landing-definitions.json"))?)?;
    let mut projected = Map::new();
    for block in block_names {
        let target = source_name(block).unwrap_or_else(|| block.clone());
        let entry = raw.get(&target).or_else(|| raw.get(block));
        if let Some(entry) = entry {
            projected.insert(block.clone(), entry.clone());
        }
    }
    write_json(
        &output.join("landing-definitions.json"),
        &Value::Object(projected),
    )
}

/// Face-visibility rules per 1.8 block, with the state ids of this registry.
fn project_render(
    source: &Path,
    output: &Path,
    registry_states: &[BlockState],
    block_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let raw: BTreeMap<String, Value> =
        serde_json::from_slice(&fs::read(source.join("render-definitions.json"))?)?;
    let mut states: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for (id, state) in registry_states.iter().enumerate() {
        states
            .entry(state.name.as_str())
            .or_default()
            .push(id as u32);
    }
    let mut projected = Map::new();
    for block in block_names {
        let target = source_name(block).unwrap_or_else(|| block.clone());
        let rule = raw.get(&target).or_else(|| raw.get(block));
        let ids = states.get(block.as_str()).cloned().unwrap_or_default();
        let mut entry = Map::new();
        entry.insert(
            "rule".into(),
            rule.and_then(|r| r.get("rule"))
                .cloned()
                .unwrap_or_else(|| json!("never")),
        );
        entry.insert(
            "can_occlude".into(),
            rule.and_then(|r| r.get("can_occlude"))
                .cloned()
                .unwrap_or_else(|| json!(false)),
        );
        entry.insert(
            "bars_tag".into(),
            rule.and_then(|r| r.get("bars_tag"))
                .cloned()
                .unwrap_or_else(|| json!(false)),
        );
        entry.insert(
            "states".into(),
            Value::Array(ids.iter().map(|&id| json!(id)).collect()),
        );
        entry.insert("default".into(), json!(ids.first().copied().unwrap_or(0)));
        projected.insert(block.clone(), Value::Object(entry));
    }
    write_json(
        &output.join("render-definitions.json"),
        &Value::Object(projected),
    )
}

fn project_table(
    source: &Path,
    output: &Path,
    file: &str,
    map: &[usize],
    registry_states: &[BlockState],
    block_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(source.join(file))?;
    let (named, _) = nbt::read_named(&bytes, Limits::tables())?;
    let NamedTag { name, mut tag } = named;
    let Tag::Compound(root) = &mut tag else {
        return Err(format!("{file}: root is not a compound").into());
    };
    root.insert(
        "states".into(),
        Tag::List(registry_states.iter().map(BlockState::to_nbt).collect()),
    );
    if file == "movement-properties.nbt" {
        let Tag::Compound(blocks) = root
            .get(&JavaString::from("blocks"))
            .cloned()
            .ok_or("movement table has no blocks")?
        else {
            return Err("movement table blocks are not a compound".into());
        };
        let fallback = blocks
            .get(&JavaString::from("minecraft:stone"))
            .cloned()
            .or_else(|| blocks.values().next().cloned())
            .ok_or("movement table has no definitions")?;
        let mut projected = Compound::new();
        for block in block_names {
            let target = source_name(block).unwrap_or_else(|| block.clone());
            let entry = blocks
                .get(&JavaString::from(target.as_str()))
                .or_else(|| blocks.get(&JavaString::from(block.as_str())))
                .cloned()
                .unwrap_or_else(|| fallback.clone());
            projected.insert(block.as_str().into(), entry);
        }
        root.insert("blocks".into(), Tag::Compound(projected));
    } else {
        let Some(Tag::IntArray(source_states)) =
            root.get(&JavaString::from("cached_shapes")).cloned()
        else {
            return Err(format!("{file}: no cached_shapes").into());
        };
        let Some(Tag::List(source_shapes)) = root.get(&JavaString::from("shapes")).cloned() else {
            return Err(format!("{file}: no shapes").into());
        };
        // The source export has a shape for every distinct current-version
        // geometry; 1.8 has fewer states. Keep only the shapes its states
        // reference, and shift the per-state ids onto the shorter list.
        let mut remap: HashMap<i32, i32> = HashMap::new();
        let mut shapes = Vec::new();
        let mut projected = Vec::with_capacity(map.len());
        for &index in map {
            let old = *source_states
                .get(index)
                .ok_or_else(|| format!("{file}: state {index} has no cached shape"))?;
            if old < 0 {
                projected.push(old);
                continue;
            }
            let shape = source_shapes
                .get(old as usize)
                .ok_or_else(|| format!("{file}: cached shape {old} is out of range"))?
                .clone();
            let new = *remap.entry(old).or_insert_with(|| {
                shapes.push(shape);
                shapes.len() as i32 - 1
            });
            projected.push(new);
        }
        root.insert("shapes".into(), Tag::List(shapes));
        root.insert("cached_shapes".into(), Tag::IntArray(projected));
    }
    fs::write(
        output.join(file),
        nbt::write_named(&NamedTag { name, tag }, Limits::tables())?,
    )?;
    Ok(())
}

/// Recursively copy `from` over `to`, creating directories.
fn copy_tree(from: &Path, to: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The 1.8 state index of every source state that has one (the first wins).
fn inverse_states(map: &[usize]) -> HashMap<usize, u32> {
    let mut inverse = HashMap::with_capacity(map.len());
    for (id, &source) in map.iter().enumerate() {
        inverse.entry(source).or_insert(id as u32);
    }
    inverse
}

/// Renumber a cached-shape table's per-state shapes onto the shorter
/// projected list; the caller has already replaced its `states` list.
fn project_shapes(
    root: &mut Compound,
    map: &[usize],
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(Tag::IntArray(source_states)) = root.get(&JavaString::from("cached_shapes")).cloned()
    else {
        return Err(format!("{what}: no cached_shapes").into());
    };
    let Some(Tag::List(source_shapes)) = root.get(&JavaString::from("shapes")).cloned() else {
        return Err(format!("{what}: no shapes").into());
    };
    let mut remap: HashMap<i32, i32> = HashMap::new();
    let mut shapes = Vec::new();
    let mut projected = Vec::with_capacity(map.len());
    for &index in map {
        let old = *source_states
            .get(index)
            .ok_or_else(|| format!("{what}: state {index} has no cached shape"))?;
        if old < 0 {
            projected.push(old);
            continue;
        }
        let shape = source_shapes
            .get(old as usize)
            .ok_or_else(|| format!("{what}: cached shape {old} is out of range"))?
            .clone();
        let new = *remap.entry(old).or_insert_with(|| {
            shapes.push(shape);
            shapes.len() as i32 - 1
        });
        projected.push(new);
    }
    root.insert("shapes".into(), Tag::List(shapes));
    root.insert("cached_shapes".into(), Tag::IntArray(projected));
    Ok(())
}

/// Project the 26.3 worldgen bundle's per-state block tables and block-name
/// tables onto the 1.8 registry. The rest of the bundle is copied unchanged:
/// single-player world generation remains 26.3-shaped (documented debt).
fn project_worldgen(
    source: &Path,
    output: &Path,
    map: &[usize],
    registry_states: &[BlockState],
    block_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let from = source.join("worldgen");
    let to = output.join("worldgen");
    copy_tree(&from, &to)?;
    let names: BTreeSet<&str> = block_names.iter().map(String::as_str).collect();
    let inverse = inverse_states(map);
    let target_of = |block: &str| source_name(block).unwrap_or_else(|| block.to_owned());
    let state_nbt = || Tag::List(registry_states.iter().map(BlockState::to_nbt).collect());

    // `[is_air, has_fluid]` per registry state.
    let flags: Vec<(bool, bool)> =
        serde_json::from_slice(&fs::read(from.join("block_flags.json"))?)?;
    let projected: Vec<Value> = map
        .iter()
        .map(|&index| {
            let (air, fluid) = flags.get(index).copied().unwrap_or((false, false));
            json!([air, fluid])
        })
        .collect();
    write_json(&to.join("block_flags.json"), &Value::Array(projected))?;

    // Drop entries 1.8 does not have. Every tag is kept, empty ones included,
    // so `#tag` references still resolve.
    let raw_tags: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(from.join("block_tags.json"))?)?;
    let mut tags = Map::new();
    for (name, text) in &raw_tags {
        let mut tag: Value = serde_json::from_str(text)?;
        if let Some(values) = tag.get_mut("values").and_then(Value::as_array_mut) {
            values.retain(|entry| {
                entry
                    .as_str()
                    .is_some_and(|entry| match entry.strip_prefix('#') {
                        Some(tag) => raw_tags.contains_key(tag),
                        None => names.contains(entry),
                    })
            });
        }
        tags.insert(name.clone(), Value::String(serde_json::to_string(&tag)?));
    }
    write_json(&to.join("block_tags.json"), &Value::Object(tags))?;

    // Block definition flags, and the nested occlusion table. Every block
    // needs a flags entry; a 1.8-only block takes air's (no overrides).
    let bytes = fs::read(from.join("blocks/block-properties.nbt"))?;
    let (named, _) = nbt::read_named(&bytes, Limits::tables())?;
    let NamedTag { name, mut tag } = named;
    let Tag::Compound(root) = &mut tag else {
        return Err("block-properties.nbt: root is not a compound".into());
    };
    let source_blocks = match root.get(&JavaString::from("blocks")) {
        Some(Tag::Compound(blocks)) => blocks.clone(),
        _ => return Err("block-properties.nbt: no blocks".into()),
    };
    let fallback = source_blocks
        .get(&JavaString::from("minecraft:air"))
        .cloned()
        .unwrap_or(Tag::Byte(0));
    let mut blocks = Compound::new();
    for block in block_names {
        let target = target_of(block);
        let flags = source_blocks
            .get(&JavaString::from(target.as_str()))
            .or_else(|| source_blocks.get(&JavaString::from(block.as_str())))
            .cloned()
            .unwrap_or_else(|| fallback.clone());
        blocks.insert(block.as_str().into(), flags);
    }
    root.insert("blocks".into(), Tag::Compound(blocks));
    root.insert("states".into(), state_nbt());
    let Some(Tag::Compound(occlusion)) = root.get_mut(&JavaString::from("occlusion")) else {
        return Err("block-properties.nbt: no occlusion".into());
    };
    occlusion.insert("states".into(), state_nbt());
    project_shapes(occlusion, map, "block-properties.nbt occlusion")?;
    fs::write(
        to.join("blocks/block-properties.nbt"),
        nbt::write_named(&NamedTag { name, tag }, Limits::tables())?,
    )?;

    for file in ["cached-support.nbt", "cached-collision.nbt"] {
        let bytes = fs::read(from.join("blocks").join(file))?;
        let (named, _) = nbt::read_named(&bytes, Limits::tables())?;
        let NamedTag { name, mut tag } = named;
        let Tag::Compound(root) = &mut tag else {
            return Err(format!("{file}: root is not a compound").into());
        };
        root.insert("states".into(), state_nbt());
        project_shapes(root, map, file)?;
        fs::write(
            to.join("blocks").join(file),
            nbt::write_named(&NamedTag { name, tag }, Limits::tables())?,
        )?;
    }

    // Template rotation/mirroring: each row is indexed by state and holds
    // state ids, so every id goes through the inverse map.
    let raw: Value = serde_json::from_slice(&fs::read(from.join("blocks/structure-blocks.json"))?)?;
    let arr = |key: &str| -> Result<&Vec<Value>, Box<dyn std::error::Error>> {
        raw.get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("structure-blocks.json: {key} is not an array").into())
    };
    let source_id = |value: &Value| value.as_u64().map(|id| id as usize);
    let mapped = |target: usize, own: usize| inverse.get(&target).copied().unwrap_or(own as u32);
    let rotate_src = arr("rotate")?;
    let mirror_src = arr("mirror")?;
    let full_src = arr("full_block")?;
    let mut rotate = Vec::with_capacity(map.len());
    let mut mirror = Vec::with_capacity(map.len());
    let mut full = Vec::with_capacity(map.len());
    for (own, &source) in map.iter().enumerate() {
        let row = rotate_src
            .get(source)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("structure-blocks.json: rotate has no state {source}"))?;
        rotate.push(Value::Array(
            row.iter()
                .take(3)
                .map(|value| json!(mapped(source_id(value).unwrap_or(own), own)))
                .collect(),
        ));
        let row = mirror_src
            .get(source)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("structure-blocks.json: mirror has no state {source}"))?;
        mirror.push(Value::Array(
            row.iter()
                .take(2)
                .map(|value| json!(mapped(source_id(value).unwrap_or(own), own)))
                .collect(),
        ));
        full.push(json!(
            full_src
                .get(source)
                .and_then(Value::as_bool)
                .unwrap_or(false)
        ));
    }
    let mut loot: Vec<u32> = Vec::new();
    if let Some(ids) = raw.get("loot_containers").and_then(Value::as_array) {
        for value in ids {
            if let Some(id) = source_id(value).and_then(|id| inverse.get(&id).copied()) {
                if !loot.contains(&id) {
                    loot.push(id);
                }
            }
        }
    }
    let mut entities = Map::new();
    if let Some(source) = raw.get("block_entities").and_then(Value::as_object) {
        for (name, ids) in source {
            let mut projected = Vec::new();
            for id in ids.as_array().into_iter().flatten() {
                if let Some(id) = source_id(id).and_then(|id| inverse.get(&id).copied()) {
                    if !projected.contains(&id) {
                        projected.push(id);
                    }
                }
            }
            if !projected.is_empty() {
                entities.insert(
                    name.clone(),
                    Value::Array(projected.into_iter().map(|id| json!(id)).collect()),
                );
            }
        }
    }
    let mut out = raw.as_object().cloned().unwrap_or_default();
    out.insert("states".into(), json!(registry_states.len()));
    out.insert("rotate".into(), Value::Array(rotate));
    out.insert("mirror".into(), Value::Array(mirror));
    out.insert("full_block".into(), Value::Array(full));
    out.insert(
        "loot_containers".into(),
        Value::Array(loot.into_iter().map(|id| json!(id)).collect()),
    );
    out.insert("block_entities".into(), Value::Object(entities));
    write_json(
        &to.join("blocks/structure-blocks.json"),
        &Value::Object(out),
    )?;

    // The survival registries reject names the registry does not have.
    let source_rules: Map<String, Value> =
        serde_json::from_slice(&fs::read(from.join("blocks/survival-rules.json"))?)?;
    let rules: Map<String, Value> = block_names
        .iter()
        .filter_map(|block| {
            let target = target_of(block);
            source_rules
                .get(&target)
                .or_else(|| source_rules.get(block))
                .map(|rule| (block.clone(), rule.clone()))
        })
        .collect();
    write_json(
        &to.join("blocks/survival-rules.json"),
        &Value::Object(rules),
    )?;

    let mut extra: Map<String, Value> = serde_json::from_slice(&fs::read(
        from.join("blocks/survival-extra-definitions.json"),
    )?)?;
    let gates: BTreeSet<String> = extra
        .get("fence_gates")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let projected: Vec<Value> = block_names
        .iter()
        .filter(|block| {
            let target = target_of(block.as_str());
            gates.contains(&target) || gates.contains(block.as_str())
        })
        .map(|block| json!(block))
        .collect();
    extra.insert("fence_gates".into(), Value::Array(projected));
    let odds: Map<String, Value> = extra
        .get("ignition_odds")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut projected = Map::new();
    for block in block_names {
        let target = target_of(block);
        if let Some(value) = odds.get(&target).or_else(|| odds.get(block)) {
            projected.insert(block.clone(), value.clone());
        }
    }
    extra.insert("ignition_odds".into(), Value::Object(projected));
    write_json(
        &to.join("blocks/survival-extra-definitions.json"),
        &Value::Object(extra),
    )?;

    let mut placement: Map<String, Value> =
        serde_json::from_slice(&fs::read(from.join("blocks/placement-definitions.json"))?)?;
    for key in ["double_plants", "mossy_carpets"] {
        let source: BTreeSet<String> = placement
            .get(key)
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let projected: Vec<Value> = block_names
            .iter()
            .filter(|block| {
                let target = target_of(block.as_str());
                source.contains(&target) || source.contains(block.as_str())
            })
            .map(|block| json!(block))
            .collect();
        placement.insert(key.into(), Value::Array(projected));
    }
    write_json(
        &to.join("blocks/placement-definitions.json"),
        &Value::Object(placement),
    )?;
    Ok(())
}

fn facts_len(output: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let facts: Value = serde_json::from_slice(&fs::read(output.join("facts.json"))?)?;
    Ok(facts["states"].as_array().map_or(0, Vec::len))
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}

/// The property names and values a 1.8 blockstate file mentions, from both
/// `variants` keys and `multipart` conditions.
fn schema_of(definition: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut schema: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(variants) = definition.get("variants").and_then(Value::as_object) {
        for key in variants.keys() {
            for clause in key.split(',') {
                let (property, value) = clause
                    .split_once('=')
                    .map_or((clause, None), |(k, v)| (k, Some(v)));
                let Some(value) = value else {
                    // 1.8 spells a single unnamed variant `normal` or
                    // `inventory`; it narrows no property.
                    continue;
                };
                if property.is_empty() {
                    continue;
                }
                schema
                    .entry(property.to_owned())
                    .or_default()
                    .insert(value.to_owned());
            }
        }
    }
    if let Some(multipart) = definition.get("multipart").and_then(Value::as_array) {
        for selector in multipart {
            if let Some(when) = selector.get("when") {
                condition_schema(when, &mut schema);
            }
        }
    }
    schema
}

/// Collect the properties of a `when` condition, including `OR`/`AND` groups.
fn condition_schema(condition: &Value, schema: &mut BTreeMap<String, BTreeSet<String>>) {
    let Some(object) = condition.as_object() else {
        return;
    };
    for (key, value) in object {
        match key.as_str() {
            "OR" | "AND" => {
                if let Some(terms) = value.as_array() {
                    for term in terms {
                        condition_schema(term, schema);
                    }
                }
            }
            _ => record(schema, key, value),
        }
    }
}

fn record(schema: &mut BTreeMap<String, BTreeSet<String>>, key: &str, value: &Value) {
    match value {
        Value::String(value) => {
            schema
                .entry(key.to_owned())
                .or_default()
                .insert(value.clone());
        }
        Value::Array(values) => {
            for value in values {
                if let Value::String(value) = value {
                    schema
                        .entry(key.to_owned())
                        .or_default()
                        .insert(value.clone());
                }
            }
        }
        _ => {}
    }
}

fn schema_value(schema: &BTreeMap<String, BTreeSet<String>>) -> Value {
    Value::Object(
        schema
            .iter()
            .map(|(name, values)| {
                (
                    name.clone(),
                    json!({"named": values.iter().collect::<Vec<_>>()}),
                )
            })
            .collect(),
    )
}

type BlockStates = Vec<(Map<String, Value>, bool)>;

/// A state's properties as a `variants` key, in a stable order.
fn variant_key(properties: &Map<String, Value>) -> String {
    let mut parts: Vec<String> = properties
        .iter()
        .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or_default()))
        .collect();
    parts.sort();
    parts.join(",")
}

fn prop<'a>(properties: &'a Map<String, Value>, key: &str) -> &'a str {
    properties.get(key).and_then(Value::as_str).unwrap_or("")
}

/// The model (and transform) of `key`, with 1.8's random-model arrays reduced
/// to their first entry.
fn variant_at<'a>(definition: &'a Value, key: &str) -> Option<&'a Value> {
    let value = definition.get("variants")?.get(key)?;
    Some(match value {
        Value::Array(models) => models.first().unwrap_or(value),
        other => other,
    })
}

fn any_variant(definition: &Value) -> Option<&Value> {
    definition
        .get("variants")?
        .as_object()?
        .values()
        .next()
        .map(|value| match value {
            Value::Array(models) => models.first().unwrap_or(value),
            other => other,
        })
}

fn variants(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
    json!({
        "variants": Value::Object(entries.into_iter().collect()),
    })
}

/// One variant per registry state, keyed by that state's full properties.
///
/// 1.8 metadata covers a subset of the modern state combinations (doors do not
/// encode `powered` on both halves, beds not every `occupied`/`part` pair), so a
/// missing combination borrows the model of the closest state that 1.8 can
/// represent. Those extra states are unreachable from a 1.8 save; they only
/// exist so the engine's state table is a full property product.
fn per_state(
    states: &BlockStates,
    variant: impl Fn(&Map<String, Value>) -> Option<Value>,
) -> Option<Value> {
    let mut exact: Vec<(&Map<String, Value>, Value)> = Vec::with_capacity(states.len());
    let mut missing: Vec<&Map<String, Value>> = Vec::new();
    for (properties, _) in states {
        match variant(properties) {
            Some(model) => exact.push((properties, model)),
            None => missing.push(properties),
        }
    }
    if exact.is_empty() {
        return None;
    }
    let mut models: HashMap<String, Value> = exact
        .iter()
        .map(|(properties, model)| (variant_key(properties), model.clone()))
        .collect();
    for properties in missing {
        let (_, model) = exact
            .iter()
            .max_by_key(|(candidate, _)| {
                candidate
                    .iter()
                    .filter(|(key, value)| properties.get(*key) == Some(*value))
                    .count()
            })
            .unwrap();
        models.insert(variant_key(properties), model.clone());
    }
    let entries = states.iter().map(|(properties, _)| {
        (
            variant_key(properties),
            models[&variant_key(properties)].clone(),
        )
    });
    Some(variants(entries))
}

/// A source state's properties as the JSON the registry stores.
fn state_properties(state: &BlockState) -> Map<String, Value> {
    state
        .properties
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect()
}

/// Expand a block's 1.8 states to every combination of their properties, in
/// property-name order with the last property fastest, and with each property's
/// values in vanilla's order. This is the order `StateTable` assumes.
fn expand_states(states: BlockStates, source: &SourceTables, name: &str) -> BlockStates {
    let Some((_, fluid)) = states.first() else {
        return states;
    };
    let fluid = *fluid;
    let mut values: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (properties, _) in &states {
        for (property, value) in properties {
            let seen = values.entry(property.as_str()).or_default();
            let value = value.as_str().unwrap_or_default();
            if !seen.contains(&value) {
                seen.push(value);
            }
        }
    }
    if values.is_empty() {
        return states;
    }
    let target = source_name(name).unwrap_or_else(|| name.to_owned());
    let mut expanded: Vec<Map<String, Value>> = vec![Map::new()];
    for (property, seen) in values {
        // Take the property's whole current-version domain when the block has
        // it: 1.8 metadata may encode only some values (leaf distance is 0 or
        // 7, never 1..6), but the engine's rules walk every value. 1.8-only
        // values keep their metadata order.
        let mut order: Vec<String> = source
            .values
            .get(&(target.clone(), property.to_owned()))
            .cloned()
            .unwrap_or_default();
        for value in seen {
            if !order.iter().any(|known| known == value) {
                order.push(value.to_owned());
            }
        }
        if order.is_empty() {
            continue;
        }
        let mut next = Vec::with_capacity(expanded.len() * order.len());
        for properties in &expanded {
            for value in &order {
                let mut combined = properties.clone();
                combined.insert(property.to_owned(), Value::String(value.clone()));
                next.push(combined);
            }
        }
        expanded = next;
    }
    expanded
        .into_iter()
        .map(|properties| (properties, fluid))
        .collect()
}

fn single(model: Value) -> Value {
    variants([(String::new(), model)])
}

/// A slab family: 1.8 has one file for the bottom/top halves and another for
/// the double block; the engine's state has a single `type` property.
fn slab(states: &BlockStates, single_file: &Value, double_file: &Value) -> Option<Value> {
    let bottom = variant_at(single_file, "half=bottom")?.clone();
    let top = variant_at(single_file, "half=top")?.clone();
    let doubled = variant_at(double_file, "normal")
        .or_else(|| variant_at(double_file, "all"))?
        .clone();
    per_state(states, |properties| {
        Some(match prop(properties, "type") {
            "bottom" => bottom.clone(),
            "top" => top.clone(),
            "double" => doubled.clone(),
            _ => return None,
        })
    })
}

fn define_slab(
    out: &mut BTreeMap<String, Value>,
    blockstates: &BTreeMap<String, Value>,
    by_name: &BTreeMap<String, BlockStates>,
    target: &str,
    single_file: &str,
    double_file: &str,
) {
    let (Some(single_definition), Some(double_definition)) =
        (blockstates.get(single_file), blockstates.get(double_file))
    else {
        return;
    };
    let Some(states) = by_name.get(&format!("minecraft:{target}")) else {
        return;
    };
    if let Some(definition) = slab(states, single_definition, double_definition) {
        out.insert(target.to_owned(), definition);
    }
}

/// The 1.8 jar's names, splits and dropped variants, translated to the state
/// vocabulary `facts.json` uses. A recipe only claims the blocks it can
/// describe completely; the rest keep their 1.8 file (or stay in
/// `missing-models.json`).
fn derive_definitions(
    blockstates: &BTreeMap<String, Value>,
    by_name: &BTreeMap<String, BlockStates>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let pass = |out: &mut BTreeMap<String, Value>, target: &str, source: &str| {
        if let Some(definition) = blockstates.get(source) {
            out.insert(target.to_owned(), definition.clone());
        }
    };
    // Renamed files whose properties are the engine's already.
    for (target, source) in [
        ("bricks", "brick_block"),
        ("chiseled_stone_bricks", "chiseled_stonebrick"),
        ("mossy_stone_bricks", "mossy_stonebrick"),
        ("cracked_stone_bricks", "cracked_stonebrick"),
        ("stone_bricks", "stonebrick"),
        ("nether_bricks", "nether_brick"),
        ("nether_quartz_ore", "quartz_ore"),
        ("note_block", "noteblock"),
        ("slime_block", "slime"),
        ("snow_block", "snow"),
        ("sugar_cane", "reeds"),
        ("lily_pad", "waterlily"),
        ("cobweb", "web"),
        ("azure_bluet", "houstonia"),
        ("grass_block", "grass"),
        ("oak_fence", "fence"),
        ("oak_fence_gate", "fence_gate"),
        ("oak_pressure_plate", "wooden_pressure_plate"),
        ("oak_trapdoor", "trapdoor"),
        ("quartz_pillar", "quartz_column"),
        ("tall_grass", "double_grass"),
        ("large_fern", "double_fern"),
        ("lilac", "syringa"),
        ("peony", "paeonia"),
        ("rose_bush", "double_rose"),
        ("snow", "snow_layer"),
        ("powered_rail", "golden_rail"),
        ("carved_pumpkin", "pumpkin"),
        ("jack_o_lantern", "lit_pumpkin"),
        ("cut_sandstone", "smooth_sandstone"),
        ("cut_red_sandstone", "smooth_red_sandstone"),
        ("polished_granite", "smooth_granite"),
        ("polished_diorite", "smooth_diorite"),
        ("polished_andesite", "smooth_andesite"),
        ("infested_stone", "stone_monster_egg"),
        ("infested_cobblestone", "cobblestone_monster_egg"),
        ("infested_stone_bricks", "stone_brick_monster_egg"),
        ("infested_mossy_stone_bricks", "mossy_brick_monster_egg"),
        ("infested_cracked_stone_bricks", "cracked_brick_monster_egg"),
        (
            "infested_chiseled_stone_bricks",
            "chiseled_brick_monster_egg",
        ),
    ] {
        pass(&mut out, target, source);
    }
    pass(&mut out, "terracotta", "hardened_clay");
    for color in [
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ] {
        let old = if color == "light_gray" {
            "silver"
        } else {
            color
        };
        pass(
            &mut out,
            &format!("{color}_terracotta"),
            &format!("{old}_stained_hardened_clay"),
        );
    }
    for (target, source) in [
        ("light_gray_carpet", "silver_carpet"),
        ("light_gray_stained_glass", "silver_stained_glass"),
        ("light_gray_stained_glass_pane", "silver_stained_glass_pane"),
        ("light_gray_wool", "silver_wool"),
    ] {
        pass(&mut out, target, source);
    }
    // Bark blocks: 1.8 keeps them as the `axis=none` variant of the log.
    for wood in ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak"] {
        if let Some(definition) = blockstates.get(&format!("{wood}_log")) {
            if let Some(model) = variant_at(definition, "axis=none") {
                out.insert(format!("{wood}_bark"), single(model.clone()));
            }
        }
    }
    // Slabs and smooth stone came from the double-slab family in 1.8.
    for (target, single_file, double_file) in [
        ("oak_slab", "oak_slab", "oak_double_slab"),
        ("spruce_slab", "spruce_slab", "spruce_double_slab"),
        ("birch_slab", "birch_slab", "birch_double_slab"),
        ("jungle_slab", "jungle_slab", "jungle_double_slab"),
        ("acacia_slab", "acacia_slab", "acacia_double_slab"),
        ("dark_oak_slab", "dark_oak_slab", "dark_oak_double_slab"),
        ("smooth_stone_slab", "stone_slab", "stone_double_slab"),
        ("sandstone_slab", "sandstone_slab", "sandstone_double_slab"),
        (
            "red_sandstone_slab",
            "red_sandstone_slab",
            "red_sandstone_double_slab",
        ),
        (
            "cobblestone_slab",
            "cobblestone_slab",
            "cobblestone_double_slab",
        ),
        ("brick_slab", "brick_slab", "brick_double_slab"),
        (
            "stone_brick_slab",
            "stone_brick_slab",
            "stone_brick_double_slab",
        ),
        (
            "nether_brick_slab",
            "nether_brick_slab",
            "nether_brick_double_slab",
        ),
        ("quartz_slab", "quartz_slab", "quartz_double_slab"),
        (
            "petrified_oak_slab",
            "wood_old_slab",
            "wood_old_double_slab",
        ),
    ] {
        define_slab(
            &mut out,
            blockstates,
            by_name,
            target,
            single_file,
            double_file,
        );
    }
    for (target, source, key) in [
        ("smooth_stone", "stone_double_slab", "all"),
        ("smooth_sandstone", "sandstone_double_slab", "all"),
        ("smooth_quartz", "quartz_double_slab", "all"),
    ] {
        if let Some(model) = blockstates.get(source).and_then(|d| variant_at(d, key)) {
            out.insert(target.to_owned(), single(model.clone()));
        }
    }
    // Anvil damage became three blocks.
    if let Some(definition) = blockstates.get("anvil") {
        for (target, damage) in [("anvil", 0), ("chipped_anvil", 1), ("damaged_anvil", 2)] {
            let Some(states) = by_name.get(&format!("minecraft:{target}")) else {
                continue;
            };
            if let Some(derived) = per_state(states, |properties| {
                variant_at(
                    definition,
                    &format!("damage={damage},facing={}", prop(properties, "facing")),
                )
                .cloned()
            }) {
                out.insert(target.to_owned(), derived);
            }
        }
    }
    // The powed state was a block id, not a property.
    if let Some(states) = by_name.get("minecraft:comparator") {
        if let Some(derived) = per_state(states, |properties| {
            let powered = prop(properties, "powered") == "true";
            let source = blockstates.get(if powered {
                "powered_comparator"
            } else {
                "unpowered_comparator"
            })?;
            variant_at(
                source,
                &format!(
                    "facing={},mode={},powered={}",
                    prop(properties, "facing"),
                    prop(properties, "mode"),
                    prop(properties, "powered")
                ),
            )
            .cloned()
        }) {
            out.insert("comparator".into(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:repeater") {
        if let Some(derived) = per_state(states, |properties| {
            let powered = prop(properties, "powered") == "true";
            let source = blockstates.get(if powered {
                "powered_repeater"
            } else {
                "unpowered_repeater"
            })?;
            variant_at(
                source,
                &format!(
                    "delay={},facing={},locked={}",
                    prop(properties, "delay"),
                    prop(properties, "facing"),
                    prop(properties, "locked")
                ),
            )
            .cloned()
        }) {
            out.insert("repeater".into(), derived);
        }
    }
    // Torches: 1.8 has one block with a `facing`; the engine splits the wall
    // form out and gives redstone torches a `lit` property.
    if let Some(torch) = blockstates.get("torch") {
        if let Some(up) = variant_at(torch, "facing=up") {
            out.insert("torch".into(), single(up.clone()));
        }
        if let Some(states) = by_name.get("minecraft:wall_torch") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(torch, &format!("facing={}", prop(properties, "facing"))).cloned()
            }) {
                out.insert("wall_torch".into(), derived);
            }
        }
    }
    if let Some(states) = by_name.get("minecraft:redstone_torch") {
        if let Some(derived) = per_state(states, |properties| {
            let lit = prop(properties, "lit") == "true";
            let source = blockstates.get(if lit {
                "redstone_torch"
            } else {
                "unlit_redstone_torch"
            })?;
            variant_at(source, "facing=up").cloned()
        }) {
            out.insert("redstone_torch".into(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:redstone_wall_torch") {
        if let Some(derived) = per_state(states, |properties| {
            let lit = prop(properties, "lit") == "true";
            let source = blockstates.get(if lit {
                "redstone_torch"
            } else {
                "unlit_redstone_torch"
            })?;
            variant_at(source, &format!("facing={}", prop(properties, "facing"))).cloned()
        }) {
            out.insert("redstone_wall_torch".into(), derived);
        }
    }
    // Flowing fire: the 1.8 models depend on the neighbouring sides, which the
    // engine's 1.8 states do not carry. Use the floor model for every age.
    if let Some(states) = by_name.get("minecraft:fire") {
        if let Some(derived) = per_state(states, |_| Some(json!({"model": "fire_floor_main"}))) {
            out.insert("fire".into(), derived);
        }
    }
    // Stems: 1.8 adds a `facing` the engine's state dropped.
    for kind in ["melon_stem", "pumpkin_stem"] {
        let Some(states) = by_name.get(&format!("minecraft:{kind}")) else {
            continue;
        };
        let Some(source) = blockstates.get(kind) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            variant_at(
                source,
                &format!("age={},facing=up", prop(properties, "age")),
            )
            .cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    // 1.8 wires share the state; `suspended` has no engine property.
    if let Some(states) = by_name.get("minecraft:tripwire") {
        if let Some(derived) = per_state(states, |properties| {
            let source = blockstates.get("tripwire")?;
            variant_at(
                source,
                &format!(
                    "attached={},east={},north={},south={},suspended=false,west={}",
                    prop(properties, "attached"),
                    prop(properties, "east"),
                    prop(properties, "north"),
                    prop(properties, "south"),
                    prop(properties, "west"),
                ),
            )
            .cloned()
        }) {
            out.insert("tripwire".into(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:tripwire_hook") {
        if let Some(derived) = per_state(states, |properties| {
            let source = blockstates.get("tripwire_hook")?;
            variant_at(
                source,
                &format!(
                    "attached={},facing={},powered={},suspended=false",
                    prop(properties, "attached"),
                    prop(properties, "facing"),
                    prop(properties, "powered"),
                ),
            )
            .cloned()
        }) {
            out.insert("tripwire_hook".into(), derived);
        }
    }
    // Walls: the datafixer's 1.8 wall states carry `none` connections the 1.8
    // file spells as booleans. Draw the post; connected walls need the saved
    // metadata, which the context-free table does not keep.
    for wall in ["cobblestone_wall", "mossy_cobblestone_wall"] {
        let Some(states) = by_name.get(&format!("minecraft:{wall}")) else {
            continue;
        };
        let Some(source) = blockstates.get(wall) else {
            continue;
        };
        let Some(post) = variant_at(
            source,
            "east=false,north=false,south=false,up=false,west=false",
        ) else {
            continue;
        };
        if let Some(derived) = per_state(states, |_| Some(post.clone())) {
            out.insert(wall.to_owned(), derived);
        }
    }
    // Huge mushrooms: booleans replaced the 13-way variant.
    for kind in ["brown_mushroom_block", "red_mushroom_block"] {
        let Some(states) = by_name.get(&format!("minecraft:{kind}")) else {
            continue;
        };
        let Some(source) = blockstates.get(kind) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            variant_at(source, &format!("variant={}", mushroom_variant(properties))).cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    // The stem block: its two states are the `stem` and `all_stem` textures.
    if let Some(states) = by_name.get("minecraft:mushroom_stem") {
        if let Some(source) = blockstates.get("brown_mushroom_block") {
            if let Some(derived) = per_state(states, |properties| {
                let all = ["down", "east", "north", "south", "up", "west"]
                    .iter()
                    .all(|key| prop(properties, key) == "true");
                variant_at(
                    source,
                    if all {
                        "variant=all_stem"
                    } else {
                        "variant=stem"
                    },
                )
                .cloned()
            }) {
                out.insert("mushroom_stem".into(), derived);
            }
        }
    }
    // Cauldron levels became a separate water cauldron block.
    if let Some(definition) = blockstates.get("cauldron") {
        if let Some(empty) = variant_at(definition, "level=0") {
            out.insert("cauldron".into(), single(empty.clone()));
        }
        if let Some(states) = by_name.get("minecraft:water_cauldron") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(definition, &format!("level={}", prop(properties, "level"))).cloned()
            }) {
                out.insert("water_cauldron".into(), derived);
            }
        }
    }
    // Buttons: `facing` became `face` plus a horizontal `facing`.
    for (kind, source_name) in [
        ("oak_button", "wooden_button"),
        ("stone_button", "stone_button"),
    ] {
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(source_name),
        ) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            let old_facing = match prop(properties, "face") {
                "floor" => "up",
                "ceiling" => "down",
                _ => prop(properties, "facing"),
            };
            variant_at(
                source,
                &format!(
                    "facing={old_facing},powered={}",
                    prop(properties, "powered")
                ),
            )
            .cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    // Doors and fence gates gained a `powered` property the 1.8 file has not.
    for (kind, source_name) in [
        ("oak_door", "wooden_door"),
        ("spruce_door", "spruce_door"),
        ("birch_door", "birch_door"),
        ("jungle_door", "jungle_door"),
        ("acacia_door", "acacia_door"),
        ("dark_oak_door", "dark_oak_door"),
        ("iron_door", "iron_door"),
    ] {
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(source_name),
        ) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            variant_at(
                source,
                &format!(
                    "facing={},half={},hinge={},open={}",
                    prop(properties, "facing"),
                    prop(properties, "half"),
                    prop(properties, "hinge"),
                    prop(properties, "open"),
                ),
            )
            .cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    for (kind, source_name) in [
        ("oak_fence_gate", "fence_gate"),
        ("spruce_fence_gate", "spruce_fence_gate"),
        ("birch_fence_gate", "birch_fence_gate"),
        ("jungle_fence_gate", "jungle_fence_gate"),
        ("acacia_fence_gate", "acacia_fence_gate"),
        ("dark_oak_fence_gate", "dark_oak_fence_gate"),
    ] {
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(source_name),
        ) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            variant_at(
                source,
                &format!(
                    "facing={},in_wall={},open={}",
                    prop(properties, "facing"),
                    prop(properties, "in_wall"),
                    prop(properties, "open"),
                ),
            )
            .cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    // Levers: `facing` split into `face` plus a horizontal `facing`; the
    // vertical forms name the axis they lean along.
    if let Some(states) = by_name.get("minecraft:lever") {
        if let Some(source) = blockstates.get("lever") {
            if let Some(derived) = per_state(states, |properties| {
                let along_z = matches!(prop(properties, "facing"), "north" | "south");
                let old_facing = match prop(properties, "face") {
                    "floor" => {
                        if along_z {
                            "up_z"
                        } else {
                            "up_x"
                        }
                    }
                    "ceiling" => {
                        if along_z {
                            "down_z"
                        } else {
                            "down_x"
                        }
                    }
                    _ => prop(properties, "facing"),
                };
                variant_at(
                    source,
                    &format!(
                        "facing={old_facing},powered={}",
                        prop(properties, "powered")
                    ),
                )
                .cloned()
            }) {
                out.insert("lever".into(), derived);
            }
        }
    }
    // 1.8 stairs still carry the shape the engine's datafixer kept.
    if let Some(source) = blockstates.get("stone_stairs") {
        if let Some(states) = by_name.get("minecraft:cobblestone_stairs") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(
                    source,
                    &format!(
                        "facing={},half={},shape={}",
                        prop(properties, "facing"),
                        prop(properties, "half"),
                        prop(properties, "shape"),
                    ),
                )
                .cloned()
            }) {
                out.insert("cobblestone_stairs".into(), derived);
            }
        }
    }
    // Lit blocks were separate block ids in 1.8.
    if let Some(states) = by_name.get("minecraft:furnace") {
        if let Some(derived) = per_state(states, |properties| {
            let lit = prop(properties, "lit") == "true";
            let source = blockstates.get(if lit { "lit_furnace" } else { "furnace" })?;
            variant_at(source, &format!("facing={}", prop(properties, "facing"))).cloned()
        }) {
            out.insert("furnace".into(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:daylight_detector") {
        if let Some(derived) = per_state(states, |properties| {
            let inverted = prop(properties, "inverted") == "true";
            let source = blockstates.get(if inverted {
                "daylight_detector_inverted"
            } else {
                "daylight_detector"
            })?;
            variant_at(source, &format!("power={}", prop(properties, "power"))).cloned()
        }) {
            out.insert("daylight_detector".into(), derived);
        }
    }
    for (kind, lit_file) in [
        ("redstone_lamp", "lit_redstone_lamp"),
        ("redstone_ore", "lit_redstone_ore"),
    ] {
        let Some(states) = by_name.get(&format!("minecraft:{kind}")) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            let lit = prop(properties, "lit") == "true";
            let source = blockstates.get(if lit { lit_file } else { kind })?;
            any_variant(source).cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:sponge") {
        if let Some(model) = blockstates
            .get("sponge")
            .and_then(|d| variant_at(d, "wet=false"))
        {
            if let Some(derived) = per_state(states, |_| Some(model.clone())) {
                out.insert("sponge".into(), derived);
            }
        }
    }
    if let Some(states) = by_name.get("minecraft:wet_sponge") {
        if let Some(model) = blockstates
            .get("sponge")
            .and_then(|d| variant_at(d, "wet=true"))
        {
            if let Some(derived) = per_state(states, |_| Some(model.clone())) {
                out.insert("wet_sponge".into(), derived);
            }
        }
    }
    // Piston heads: the moving block has no 1.8 blockstate file.
    if let Some(source) = blockstates.get("piston_head") {
        if let Some(states) = by_name.get("minecraft:moving_piston") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(
                    source,
                    &format!(
                        "facing={},short=false,type={}",
                        prop(properties, "facing"),
                        prop(properties, "type"),
                    ),
                )
                .cloned()
            }) {
                out.insert("moving_piston".into(), derived);
            }
        }
    }
    // Flower pots: one 1.8 blockstate file, one model per content.
    if let Some(source) = blockstates.get("flower_pot") {
        for (contents, name) in FLOWER_POT_CONTENTS {
            let Some(states) = by_name.get(&format!("minecraft:{name}")) else {
                continue;
            };
            let Some(model) = variant_at(source, &format!("contents={contents}")) else {
                continue;
            };
            if let Some(derived) = per_state(states, |_| Some(model.clone())) {
                out.insert(name.to_owned(), derived);
            }
        }
    }
    // Skulls: 1.8 has no blockstate file; its renderer draws the tile entity.
    // Use one placeholder model (particle texture only), as 26.3 does, and
    // key every property state to it so the engine's binder covers them all.
    for (name, _) in horizon_world::legacy_1_8_9::SKULLS {
        let Some(states) = by_name.get(name) else {
            continue;
        };
        let path = name.strip_prefix("minecraft:").unwrap_or(name);
        let model = json!({"model": "minecraft:block/skull"});
        out.insert(
            path.to_owned(),
            variants(
                states
                    .iter()
                    .map(|(properties, _)| (variant_key(properties), model.clone())),
            ),
        );
    }
    // A 1.8 file that reads fewer properties than the engine's state draws the
    // same model for every value of the extra ones.
    for kind in ["dispenser", "dropper", "hopper"] {
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(kind),
        ) else {
            continue;
        };
        if let Some(derived) = per_state(states, |properties| {
            variant_at(source, &format!("facing={}", prop(properties, "facing"))).cloned()
        }) {
            out.insert(kind.to_owned(), derived);
        }
    }
    if let Some(states) = by_name.get("minecraft:redstone_wire") {
        if let Some(source) = blockstates.get("redstone_wire") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(
                    source,
                    &format!(
                        "east={},north={},south={},west={}",
                        prop(properties, "east"),
                        prop(properties, "north"),
                        prop(properties, "south"),
                        prop(properties, "west"),
                    ),
                )
                .cloned()
            }) {
                out.insert("redstone_wire".into(), derived);
            }
        }
    }
    // One 1.8 model, several engine states: block ids or metadata that the
    // 1.8 file does not name.
    for (kind, source_name) in [
        ("cactus", "cactus"),
        ("command_block", "command_block"),
        ("jukebox", "jukebox"),
        ("tnt", "tnt"),
        ("short_grass", "tall_grass"),
        ("sugar_cane", "reeds"),
    ] {
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(source_name),
        ) else {
            continue;
        };
        if let Some(model) = any_variant(source) {
            if let Some(derived) = per_state(states, |_| Some(model.clone())) {
                out.insert(kind.to_owned(), derived);
            }
        }
    }
    // 1.8 writes one `normal` model per leaves block; every `persistent`/
    // `distance` state the engine walks draws it.
    for wood in ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak"] {
        let kind = format!("{wood}_leaves");
        let (Some(states), Some(source)) = (
            by_name.get(&format!("minecraft:{kind}")),
            blockstates.get(&kind),
        ) else {
            continue;
        };
        if let Some(model) = any_variant(source) {
            if let Some(derived) = per_state(states, |_| Some(model.clone())) {
                out.insert(kind, derived);
            }
        }
    }
    // Beds gained `occupied`.
    if let Some(states) = by_name.get("minecraft:red_bed") {
        if let Some(source) = blockstates.get("bed") {
            if let Some(derived) = per_state(states, |properties| {
                variant_at(
                    source,
                    &format!(
                        "facing={},part={}",
                        prop(properties, "facing"),
                        prop(properties, "part")
                    ),
                )
                .cloned()
            }) {
                out.insert("red_bed".into(), derived);
            }
        }
    }
    // Blocks drawn by a block entity renderer, or not drawn at all. A native
    // empty multipart binds every state to no model.
    let empty = json!({"multipart": []});
    for name in [
        "air",
        "barrier",
        "end_portal",
        "water",
        "lava",
        "oak_sign",
        "oak_wall_sign",
        "white_banner",
        "white_wall_banner",
        "chest",
        "trapped_chest",
        "ender_chest",
    ] {
        out.insert(name.to_owned(), empty.clone());
    }
    out
}

/// The 1.8 `variant` of a huge mushroom's six face booleans, from the
/// datafixer's own mapping.
fn mushroom_variant(properties: &Map<String, Value>) -> &'static str {
    let on = |key: &str| prop(properties, key) == "true";
    let (up, down, north, east, south, west) = (
        on("up"),
        on("down"),
        on("north"),
        on("east"),
        on("south"),
        on("west"),
    );
    match (up, down, north, east, south, west) {
        (false, false, false, false, false, false) => "all_inside",
        (true, true, true, true, true, true) => "all_outside",
        (true, false, true, false, false, true) => "north_west",
        (true, false, true, false, false, false) => "north",
        (true, false, true, true, false, false) => "north_east",
        (true, false, false, false, false, true) => "west",
        (true, false, false, false, false, false) => "center",
        (true, false, false, true, false, false) => "east",
        (true, false, false, false, true, true) => "south_west",
        (true, false, false, false, true, false) => "south",
        (true, false, false, true, true, false) => "south_east",
        (false, false, true, true, true, true) => "stem",
        // The 1.8 table's remaining states are unreachable through the
        // context-free mapping; a full cap is the closest model.
        _ => "all_outside",
    }
}

fn copy_client_assets(
    archive: &mut zip::ZipArchive<fs::File>,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let name = name.to_path_buf();
        if !name.starts_with("assets/") || entry.is_dir() {
            continue;
        }
        let target = output.join(&name);
        fs::create_dir_all(target.parent().ok_or("missing asset parent")?)?;
        let mut file = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}
