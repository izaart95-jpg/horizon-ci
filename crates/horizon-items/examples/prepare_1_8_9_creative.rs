//! Build the 1.8.9 creative inventory from a `CreativeTabsProbe18` dump.
//!
//! 1.8's creative menu is not 26.3's tab set: it has the twelve
//! `ItemGroup.itemGroups` entries (ten categories, search and inventory), and
//! each category's stacks are `ItemGroup.showItems`' own subtype expansion.
//! The probe runs the real 1.8.9 jar, so the items, their damage values, the
//! enchantment books and the search tab's tooltip lines are vanilla's; this
//! tool turns each numeric stack through the datafixer
//! ([`horizon_items::legacy_1_8_9::stack`]) into the engine's current
//! item/components and writes the 26.3-shaped `creative-inventory.json` the
//! client already loads.
//!
//! ```sh
//! javac -cp <1.8.9 named jar + client jar + libraries> tools/CreativeTabsProbe18.java
//! java -cp <the same>:. CreativeTabsProbe18 > creative-tabs-1.8.9.txt
//! cargo run -p horizon-items --example prepare_1_8_9_creative -- \
//!     --dump creative-tabs-1.8.9.txt \
//!     --out .local/horizon-client/1.8.9/creative-inventory.json
//! ```
use horizon_items::{item_nbt, items::ItemStack, legacy_1_8_9};
use horizon_world::{nbt, snbt_text::to_snbt};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, error::Error, fs, path::PathBuf};

/// `ItemGroup.itemGroups` in index order.
struct TabDef {
    id: &'static str,
    title: &'static str,
    kind: &'static str,
    column: u32,
    top: bool,
}

const TABS: [TabDef; 12] = [
    TabDef {
        id: "minecraft:building_blocks",
        title: "Building Blocks",
        kind: "CATEGORY",
        column: 0,
        top: true,
    },
    TabDef {
        id: "minecraft:decorations",
        title: "Decoration Blocks",
        kind: "CATEGORY",
        column: 1,
        top: true,
    },
    TabDef {
        id: "minecraft:redstone",
        title: "Redstone",
        kind: "CATEGORY",
        column: 2,
        top: true,
    },
    TabDef {
        id: "minecraft:transportation",
        title: "Transportation",
        kind: "CATEGORY",
        column: 3,
        top: true,
    },
    TabDef {
        id: "minecraft:misc",
        title: "Miscellaneous",
        kind: "CATEGORY",
        column: 4,
        top: true,
    },
    TabDef {
        id: "minecraft:search",
        title: "Search Items",
        kind: "SEARCH",
        column: 5,
        top: true,
    },
    TabDef {
        id: "minecraft:food",
        title: "Foodstuffs",
        kind: "CATEGORY",
        column: 0,
        top: false,
    },
    TabDef {
        id: "minecraft:tools",
        title: "Tools",
        kind: "CATEGORY",
        column: 1,
        top: false,
    },
    TabDef {
        id: "minecraft:combat",
        title: "Combat",
        kind: "CATEGORY",
        column: 2,
        top: false,
    },
    TabDef {
        id: "minecraft:brewing",
        title: "Brewing",
        kind: "CATEGORY",
        column: 3,
        top: false,
    },
    TabDef {
        id: "minecraft:materials",
        title: "Materials",
        kind: "CATEGORY",
        column: 4,
        top: false,
    },
    TabDef {
        id: "minecraft:inventory",
        title: "Survival Inventory",
        kind: "INVENTORY",
        column: 5,
        top: false,
    },
];

/// One dumped stack: the engine stack, its 1.8 display name and the tooltip
/// lines 1.8's `search()` matches.
#[derive(Clone)]
struct Dump {
    stack: ItemStack,
    name: String,
    lines: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut dump = PathBuf::new();
    let mut out = PathBuf::from(".local/horizon-client/1.8.9/creative-inventory.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dump" => dump = args.next().ok_or("--dump needs a path")?.into(),
            "--out" => out = args.next().ok_or("--out needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    if dump.as_os_str().is_empty() {
        return Err("--dump needs the CreativeTabsProbe18 dump".into());
    }
    let text = fs::read_to_string(&dump)?;
    let mut items: BTreeMap<usize, Vec<Dump>> = BTreeMap::new();
    let mut icons: BTreeMap<usize, Dump> = BTreeMap::new();
    let mut search: Vec<Dump> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.splitn(8, '\t').collect();
        let row = fields[0];
        let rest = match row {
            "G" => &fields[1..],
            "I" => &fields[1..],
            "S" => &fields[1..],
            other => return Err(format!("line {}: unknown row {other:?}", number + 1).into()),
        };
        let dump = parse_dump(rest)?;
        match row {
            "G" => items.entry(rest[0].parse()?).or_default().push(dump),
            "I" => {
                icons.insert(rest[0].parse()?, dump);
            }
            "S" => search.push(dump),
            _ => unreachable!(),
        }
    }
    let mut tabs = Vec::new();
    for (index, def) in TABS.iter().enumerate() {
        let stacks = items.get(&index).cloned().unwrap_or_default();
        let icon = icons
            .get(&index)
            .map(|d| to_snbt(&item_nbt::item_to_nbt(&d.stack)))
            .unwrap_or_else(|| "{count:1,id:\"minecraft:stone\"}".to_owned());
        let mut tab = Map::new();
        tab.insert("id".into(), json!(def.id));
        tab.insert("type".into(), json!(def.kind));
        tab.insert("row".into(), json!(if def.top { "TOP" } else { "BOTTOM" }));
        tab.insert("column".into(), json!(def.column));
        tab.insert("title".into(), json!(def.title));
        tab.insert("icon".into(), json!(icon));
        tab.insert("visible".into(), json!(true));
        if def.kind == "SEARCH" {
            tab.insert(
                "items".into(),
                json!(search.iter().map(snbt).collect::<Vec<_>>()),
            );
            tab.insert(
                "lines".into(),
                json!(
                    search
                        .iter()
                        .map(|d| if d.lines.is_empty() {
                            vec![search_line(&d.name)]
                        } else {
                            d.lines.clone()
                        })
                        .collect::<Vec<_>>()
                ),
            );
            tab.insert(
                "tags".into(),
                json!(
                    search
                        .iter()
                        .map(|_| Vec::<String>::new())
                        .collect::<Vec<_>>()
                ),
            );
        } else {
            tab.insert(
                "items".into(),
                json!(stacks.iter().map(snbt).collect::<Vec<_>>()),
            );
        }
        tabs.push(Value::Object(tab));
    }
    let document = json!({"schema": 3, "tabs": tabs});
    let mut text = serde_json::to_string_pretty(&document)?;
    text.push('\n');
    fs::write(&out, text)?;
    println!(
        "{} categories, {} search stacks -> {}",
        items.len(),
        search.len(),
        out.display()
    );
    Ok(())
}

fn parse_dump(fields: &[&str]) -> Result<Dump, Box<dyn Error>> {
    // A category or icon row carries its tab index first; a search row does not.
    let offset = if fields.len() == 7 { 1 } else { 0 };
    let id: i16 = fields[offset].parse()?;
    let count: i8 = fields[offset + 1].parse()?;
    let damage: i16 = fields[offset + 2].parse()?;
    let name = fields[offset + 3].to_owned();
    let tag = fields[offset + 4];
    let tag = if tag == "-" {
        nbt::Tag::End
    } else {
        nbt::parse_snbt(&java_list_syntax(tag))?.0
    };
    // The tooltip column is newer than the dump's first format: an older dump
    // has only the name to search.
    let lines: Vec<String> = fields
        .get(offset + 5)
        .map(|tooltip| {
            tooltip
                .split('\u{1f}')
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let Some(stack) = legacy_1_8_9::stack(id, count, damage, &tag) else {
        return Err(format!("item {id} damage {damage} does not datafix to an item").into());
    };
    Ok(Dump { stack, name, lines })
}

/// 1.8's `NbtList.toString` writes each element as `<index>:<value>`
/// (`[0:{lvl:5s,id:32s}]`); SNBT lists have no indices. Drop them, leaving
/// quoted strings alone.
fn java_list_syntax(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        if quoted {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        if c == '"' {
            quoted = true;
            out.push(c);
            continue;
        }
        if c == '[' || c == ',' {
            out.push(c);
            // An index only follows the opening bracket of a list; after a comma
            // it may be a compound key, so only digits followed by a colon are
            // an index. A quoted key is never digits.
            let mut look = chars.clone();
            let mut digits = String::new();
            while let Some(d) = look.peek().copied().filter(char::is_ascii_digit) {
                digits.push(d);
                look.next();
            }
            if !digits.is_empty() && look.peek() == Some(&':') {
                for _ in 0..digits.len() {
                    chars.next();
                }
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn snbt(dump: &Dump) -> String {
    to_snbt(&item_nbt::item_to_nbt(&dump.stack))
}

/// The display name as an older dump's search matches it: formatting codes
/// removed and lower-cased. Newer dumps carry 1.8's full tooltip lines.
fn search_line(name: &str) -> String {
    let mut plain = String::with_capacity(name.len());
    let mut chars = name.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            plain.push(c);
        }
    }
    plain.to_lowercase()
}
