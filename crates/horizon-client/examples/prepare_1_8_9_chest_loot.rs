//! Write the 1.8.9 chest loot tables from a `ChestLootProbe18` dump.
//!
//! 1.8 has no loot-table files: each structure holds a
//! `List<WeightedRandomChestContent>` in its class and fills the chest from
//! it. The dump this reads is produced by `tools/ChestLootProbe18.java`,
//! which reflects those lists out of the real 1.8.9 jar:
//!
//! ```text
//! category <tab> item id <tab> damage <tab> min <tab> max <tab> weight <tab> nbt
//! ```
//!
//! The tables are written under the 1.8.9 root's
//! `data/minecraft/loot_table/chests/`, in the shape the engine's loot reader
//! expects (one pool, weighted entries, `set_count` for the stack range, and
//! the structure's random enchanted book). `horizon_items::chest_loot_1_8_9`
//! owns the category-to-table mapping and the JSON.
//!
//! ```sh
//! javac -cp <1.8.9 named jar + libraries> tools/ChestLootProbe18.java
//! java -cp <1.8.9 named jar + libraries>:. ChestLootProbe18 > chest-loot18.txt
//! cargo run --profile fast -p horizon-client --example prepare_1_8_9_chest_loot -- \
//!     --dump /tmp/opencode/chest-loot-1.8.9.txt --root .local/horizon-client/1.8.9
//! ```
use horizon_items::chest_loot_1_8_9::{self, TABLES};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut dump = PathBuf::from("/tmp/opencode/chest-loot-1.8.9.txt");
    let mut root = PathBuf::from(".local/horizon-client/1.8.9");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dump" => dump = args.next().ok_or("--dump needs a path")?.into(),
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let text = fs::read_to_string(&dump)?;
    let groups = chest_loot_1_8_9::parse_dump(&text)?;
    let loot = root.join("data/minecraft/loot_table");
    let mut rows = 0;
    for (category, entries) in &groups {
        let (_, table) = TABLES
            .iter()
            .find(|(name, _)| name == category)
            .ok_or_else(|| format!("{category}: no chest table"))?;
        let relative = table
            .id
            .strip_prefix("minecraft:")
            .ok_or_else(|| format!("{}: not a minecraft id", table.id))?;
        let path = loot.join(format!("{relative}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = chest_loot_1_8_9::table_json(table, entries);
        fs::write(&path, serde_json::to_vec_pretty(&json)?)?;
        println!(
            "{} -> {} ({} rows, rolls {}..={})",
            category,
            path.display(),
            entries.len(),
            table.rolls.0,
            table.rolls.1
        );
        rows += entries.len();
    }
    for (category, table) in TABLES {
        if !groups.iter().any(|(name, _)| name == category) {
            return Err(format!("{category}: no rows in the dump (wanted {})", table.id).into());
        }
    }
    println!(
        "{} chest tables, {rows} rows under {}",
        groups.len(),
        loot.display()
    );
    Ok(())
}
