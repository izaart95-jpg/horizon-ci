//! Write a small world in the 1.8.9 Anvil layout without prepared resources.
//!
//! The block registry comes from the 1.8 → current datafixer itself, so the
//! example needs only the repository. Point a real 1.8.9 server at the output
//! folder and check the blocks with `/testforblock` and `/blockdata`:
//!
//! ```sh
//! cargo run --profile fast -p horizon-client --example make_1_8_9_world -- /tmp/horizon-1-8-9
//! ```
use horizon_client::{
    chunk::{Chunk, LevelShape},
    containers::{BlockEntity, Furnace, FurnaceKind},
    save::{self, Level, SaveDir, SaveFormat, WorldGen, WorldPreset},
    signs::{SignData, SignText},
};
use horizon_core::position::ChunkPos;
use horizon_world::{
    chunk::{BlockProperties, BlockState},
    compression::Compression,
    legacy_1_8_9,
    nbt::{Limits, Tag},
    region::{ChunkRecord, RegionFile},
    registry::BlockRegistry,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

/// The 1.8 states the world uses, as `(id, metadata)`.
const BLOCKS: [(u16, u8); 9] = [
    (1, 0),   // stone
    (2, 0),   // grass block
    (3, 0),   // dirt
    (4, 0),   // cobblestone
    (5, 0),   // oak planks
    (17, 0),  // oak log
    (35, 14), // red wool
    (54, 2),  // chest, facing north
    (61, 2),  // furnace, facing north
];

fn state_json(state: &BlockState) -> Value {
    if state.properties.is_empty() {
        return json!(state.name);
    }
    let properties: BTreeMap<&str, &str> = state
        .properties
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    json!({ "id": state.name, "properties": properties })
}

/// A facts file with the canonical current states of [`BLOCKS`] (and the 1.8
/// sign state), enough for `BlockRegistry` to hold exactly what the writer
/// needs.
fn registry() -> BlockRegistry {
    let mut rows = Vec::new();
    let mut defaults = serde_json::Map::new();
    let mut seen = BTreeMap::new();
    let mut add = |state: BlockState| {
        if seen.values().any(|s: &BlockState| s == &state) {
            return;
        }
        if seen.contains_key(&state.name) {
            return;
        }
        let index = seen.len();
        let name = state.name.clone();
        let air = name == "minecraft:air";
        rows.push(json!([
            name.clone(),
            air,
            false,
            format!("Block{{{name}}}"),
            state_json(&state),
        ]));
        defaults.insert(name.clone(), json!(index));
        seen.insert(name, state);
    };
    add(BlockState {
        name: "minecraft:air".into(),
        properties: BlockProperties::new(),
    });
    for (id, meta) in BLOCKS {
        add(legacy_1_8_9::block_state(id, meta));
    }
    add(legacy_1_8_9::block_state(63, 0)); // oak sign
    let facts = json!({ "states": rows, "defaults": defaults });
    BlockRegistry::from_facts_json(facts.to_string().as_bytes()).expect("facts")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/horizon-1-8-9".into())
        .into();
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let registry = registry();
    let state = |name: &str| {
        registry
            .default_state(name)
            .unwrap_or_else(|| panic!("no {name}"))
    };

    let mut chunk = Chunk::empty(0, 0);
    for z in 0..16 {
        for x in 0..16 {
            chunk.set(x, 3, z, state("minecraft:stone").0);
        }
    }
    for x in 4..12 {
        chunk.set(x, 4, 3, state("minecraft:oak_planks").0);
    }
    chunk.set(8, 4, 8, state("minecraft:chest").0);
    chunk.set(6, 4, 6, state("minecraft:furnace").0);
    chunk.set(4, 4, 4, state("minecraft:oak_sign").0);
    chunk.set(2, 4, 2, state("minecraft:red_wool").0);
    chunk.set(13, 4, 13, state("minecraft:oak_log").0);
    chunk.set(0, 4, 0, state("minecraft:grass_block").0);
    chunk.set(1, 4, 1, state("minecraft:dirt").0);
    chunk.set(3, 4, 3, state("minecraft:cobblestone").0);

    let stone = horizon_client::items::ItemStack::new("minecraft:stone", 12);
    let mut chest_slots = vec![None; 27];
    chest_slots[0] = Some(stone);
    chunk
        .block_entities
        .push(((8, 4, 8), BlockEntity::Chest(chest_slots)));
    chunk.block_entities.push((
        (6, 4, 6),
        BlockEntity::Furnace(Furnace {
            kind: FurnaceKind::Furnace,
            items: [
                Some(horizon_client::items::ItemStack::new(
                    "minecraft:iron_ore",
                    1,
                )),
                Some(horizon_client::items::ItemStack::new("minecraft:coal", 2)),
                Some(horizon_client::items::ItemStack::new(
                    "minecraft:iron_ingot",
                    1,
                )),
            ],
            lit_remaining: 100,
            lit_total: 200,
            cooking_timer: 42,
            cooking_total: 200,
            speed: 1.0,
            recipes_used: Default::default(),
        }),
    ));
    chunk.block_entities.push((
        (4, 4, 4),
        BlockEntity::Sign(SignData {
            front: SignText {
                lines: ["hello".into(), "1.8.9".into(), "".into(), "".into()],
                ..Default::default()
            },
            ..Default::default()
        }),
    ));

    let column = save::encode_1_8_9_with_biomes(
        &chunk,
        &registry,
        |_, _| 1, // plains
        None,
        LevelShape::OVERWORLD,
    );
    let region_dir = root.join("region");
    std::fs::create_dir_all(&region_dir)?;
    let mut region = RegionFile::open_writable(region_dir.join("r.0.0.mca"), true)?;
    let record = ChunkRecord::from_nbt(&column, Compression::Zlib, 0, Limits::saved())?;
    region.write(ChunkPos::new(0, 0), &record)?;
    region.sync()?;

    // Entities are saved separately, as the game does; the 1.8 writer merges
    // them back into `Level.Entities` of this chunk.
    let pos = |x: f64, y: f64, z: f64| {
        Tag::List(vec![
            Tag::Double(x.to_bits()),
            Tag::Double(y.to_bits()),
            Tag::Double(z.to_bits()),
        ])
    };
    let still = || Tag::List(vec![Tag::Double(0f64.to_bits()); 3]);
    let uuid = |seed: i32| Tag::IntArray(vec![seed, 0, 0, seed]);
    let dropped_item = Tag::compound([
        ("id", Tag::String("minecraft:item".into())),
        ("UUID", uuid(1)),
        ("Pos", pos(8.5, 4.0, 8.5)),
        ("Motion", still()),
        (
            "Item",
            horizon_items::item_nbt::item_to_nbt(&horizon_client::items::ItemStack::new(
                "minecraft:red_wool",
                3,
            )),
        ),
        ("Age", Tag::Short(0)),
        ("PickupDelay", Tag::Short(10)),
    ]);
    let cow = Tag::compound([
        ("id", Tag::String("minecraft:cow".into())),
        ("UUID", uuid(2)),
        ("Pos", pos(10.5, 4.0, 10.5)),
        ("Motion", still()),
        (
            "Rotation",
            Tag::List(vec![Tag::Float(0f32.to_bits()), Tag::Float(0f32.to_bits())]),
        ),
        ("Health", Tag::Float(10f32.to_bits())),
    ]);
    let save = SaveDir::new_with_format(root.clone(), LevelShape::OVERWORLD, SaveFormat::V189);
    save.save_entities(vec![((0, 0), vec![dropped_item, cow])])?;

    save::write_world_gen_settings(
        &root,
        1234,
        WorldGen {
            preset: WorldPreset::Flat,
            structures: false,
            bonus_chest: false,
        },
    )?;
    let mut level = Level {
        format: SaveFormat::V189,
        name: "Horizon 1.8.9".into(),
        seed: 1234,
        time: 6000,
        day_time: 6000,
        allow_commands: Some(true),
        world_spawn: Some([8, 5, 8]),
        ..Level::default()
    };
    level.write(&root)?;
    println!("wrote {}", root.display());
    Ok(())
}
