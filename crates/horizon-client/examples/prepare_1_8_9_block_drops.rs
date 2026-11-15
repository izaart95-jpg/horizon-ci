//! Build the 1.8.9 block-drop table from a `BlockDropProbe18` dump.
//!
//! 1.8 drops are code, not loot tables: `Block.randomDropAsItem` multiplies
//! `getBonusDrops` into `getDropItem` calls and `getMeta` writes the item
//! damage. The dump this reads is produced by `tools/BlockDropProbe18.java`,
//! which calls those methods (and `createStackFromBlock` for silk touch) on
//! the real 1.8.9 jar:
//!
//! ```text
//! BLOCK <id> <name> <class>
//! STATE <id> <meta> drop=<item>@<meta> count=<n> bonus=<f0,f1,f2,f3>
//!       silk=<item>@<meta> silkrequired=<bool> metaout=<n> ...
//! ```
//!
//! The table stores `[key, drop, meta, count, silk, silk_meta, flags,
//! bonus]` per state, keyed by 1.8's wire state (`id << 4 | meta`). The rules
//! whose count depends on the random stream (ores, glowstone, melon, sea
//! lantern, mushroom blocks) stay in `horizon-items::block_drops_1_8_9`, so
//! the table marks them with a `bonus` code instead of baking one sample.
//!
//! ```sh
//! javac -cp <1.8.9 named jar + libraries> tools/BlockDropProbe18.java
//! java -cp <1.8.9 named jar + libraries>:. BlockDropProbe18 > block-drops18.txt
//! cargo run -p horizon-client --example prepare_1_8_9_block_drops -- \
//!     --drops block-drops18.txt --output .local/horizon-client/1.8.9
//! ```
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::PathBuf};

/// The 1.8 `getBonusDrops` override this block has, as the code the game
/// reads back. Everything else uses the table's constant count.
fn bonus(id: u16) -> u8 {
    match id {
        16 | 56 | 129 | 153 => 1, // coal, diamond, emerald, quartz
        21 => 2,                  // lapis
        73 | 74 => 3,             // redstone ore (lit and unlit)
        89 => 4,                  // glowstone
        103 => 5,                 // melon
        169 => 6,                 // sea lantern
        99 => 7,                  // brown mushroom block
        100 => 8,                 // red mushroom block
        _ => 0,
    }
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split_whitespace()
        .find_map(|item| item.strip_prefix(&format!("{key}=")))
}

fn stack(text: &str, key: &str) -> Result<(i16, u8), Box<dyn std::error::Error>> {
    let value = field(text, key).ok_or_else(|| format!("missing {key}"))?;
    let (id, meta) = value.split_once('@').ok_or("stack has no @meta")?;
    Ok((id.parse()?, meta.parse()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut drops = PathBuf::new();
    let mut output = PathBuf::from(".local/horizon-client/1.8.9");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--drops" => drops = args.next().ok_or("--drops needs a path")?.into(),
            "--output" => output = args.next().ok_or("--output needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    if drops.as_os_str().is_empty() {
        return Err("--drops needs the BlockDropProbe18 dump".into());
    }
    let text = fs::read_to_string(&drops)?;
    let mut rows: Vec<[i64; 8]> = Vec::new();
    // Several 1.8 states share one `(id, metadata)` pair (grass and mycelium
    // have a snowy state that still reports metadata 0, fire has 192 states
    // for 16 metadata values); their drops are identical, so the first row
    // wins and a differing duplicate is a generator bug.
    let mut keys: HashMap<i64, usize> = HashMap::new();
    let mut block = 0u16;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("BLOCK ") {
            block = rest
                .split_whitespace()
                .next()
                .ok_or("BLOCK has no id")?
                .parse()?;
            continue;
        }
        let Some(rest) = line.strip_prefix("STATE ") else {
            continue;
        };
        let meta: u8 = rest
            .split_whitespace()
            .nth(1)
            .ok_or("STATE has no metadata")?
            .parse()?;
        let key = i64::from(block) << 4 | i64::from(meta);
        // `SkullBlock.randomDropAsItem` returns immediately: the item comes
        // from `onBreaking` (the block's tile entity names the type), not
        // from the block's drop table.
        if block == 144 {
            continue;
        }
        // `drop` is a bare item id (`getDropItem`); the metadata comes from
        // `metaout`. `silk` is the full `createStackFromBlock` stack.
        let drop = match field(rest, "drop") {
            Some("-") | None => None,
            Some(id) => Some(id.parse::<i16>()?),
        };
        let (silk, silk_meta) =
            stack(rest, "silk").map_or((-1, 0), |(id, meta)| (i64::from(id), i64::from(meta)));
        let silk_required = field(rest, "silkrequired") == Some("true");
        let row = [
            key,
            drop.map_or(-1, i64::from),
            field(rest, "metaout").ok_or("missing metaout")?.parse()?,
            field(rest, "count").ok_or("missing count")?.parse()?,
            silk,
            silk_meta,
            i64::from(silk_required),
            i64::from(bonus(block)),
        ];
        match keys.entry(key) {
            std::collections::hash_map::Entry::Occupied(existing) => {
                if rows[*existing.get()] != row {
                    return Err(format!("state key {key} has two different drops").into());
                }
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(rows.len());
                rows.push(row);
            }
        }
    }
    // 7,806 states collapse to 1,394 (id, metadata) rows.
    if rows.len() < 1_300 {
        return Err(format!("only {} distinct states in the dump", rows.len()).into());
    }
    rows.sort_unstable_by_key(|row| row[0]);
    let path = output.join("block-drops.json");
    let json: Value = json!({"format": 1, "rows": rows});
    fs::write(&path, serde_json::to_vec(&json)?)?;
    println!(
        "{} states, {} blocks -> {}",
        rows.len(),
        keys.len(),
        path.display()
    );
    Ok(())
}
