//! 1.8.9 chest loot: every structure's hard-coded `WeightedRandomChestContent`
//! list, as the chest tables the engine fills when a container is opened.
//!
//! 1.8 has no loot-table files. `DungeonFeature`, the mineshaft, fortress,
//! desert and jungle temples, the three stronghold chests, the village
//! blacksmith and the bonus chest each hold a static list and call
//! `WeightedRandomChestContent.fillInventory(random, list, inventory, count)`
//! while they build. Several append one random enchanted book, either
//! `EnchantedBookItem.method_4610` (one level-30 book, weight 1) or, for the
//! stronghold library, `method_6347(random, 1, 5, 2)` (one to five level-30
//! books, weight 2). `tools/ChestLootProbe18.java` dumps those lists from the
//! real 1.8.9 jar; `examples/prepare_1_8_9_chest_loot.rs` writes them under
//! the 1.8.9 root's `data/minecraft/loot_table/chests/`.
//!
//! The engine fills a chest on open from a 26.3-shaped table, while 1.8 fills
//! it at generation and replaces a random slot per roll. The items, counts and
//! weights here are 1.8's; the slot order and the resulting number of stacks
//! are the engine's (26.3's shuffle-and-split), which is the remaining debt.
use crate::{Result, enchant::ENCHANTING_TABLE_1_8_9, legacy_1_8_9};
use horizon_world::nbt::Tag;
use serde_json::{Value, json};

/// One `WeightedRandomChestContent` row: an item, its stack size range and
/// its weight in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub item: String,
    pub min: i32,
    pub max: i32,
    pub weight: i32,
}

/// The random enchanted book a structure appends after its static list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Book {
    /// `minCount`..=`maxCount` books of one enchantment.
    pub min: i32,
    pub max: i32,
    pub weight: i32,
}

/// One 1.8 structure's chest: the table id the engine's structures store and
/// the `count` argument of its `fillInventory` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Table {
    pub id: &'static str,
    /// `min + nextInt(max - min + 1)` rolls, 1.8's `fillInventory` count.
    pub rolls: (i32, i32),
    pub book: Option<Book>,
}

/// The dump's category (`<class>.<field>`) and the chest it fills.
///
/// 1.8 has one village chest, the blacksmith's; the datafixer renames its
/// table to `village/village_weaponsmith`, the profession the modern village
/// gives that chest, so that is the file the engine can reach.
pub const TABLES: [(&str, Table); 11] = [
    (
        "DungeonFeature.field_10161",
        Table {
            id: "minecraft:chests/simple_dungeon",
            rolls: (8, 8),
            book: Some(Book {
                min: 1,
                max: 1,
                weight: 1,
            }),
        },
    ),
    (
        "MineshaftPieces.LOOT_TABLES",
        Table {
            id: "minecraft:chests/abandoned_mineshaft",
            rolls: (3, 6),
            book: Some(Book {
                min: 1,
                max: 1,
                weight: 1,
            }),
        },
    ),
    (
        "BlacksmithHouse.POSSIBLE_CHEST_CONTENT",
        Table {
            id: "minecraft:chests/village/village_weaponsmith",
            rolls: (3, 8),
            book: None,
        },
    ),
    (
        "AbstractPiece.POSSIBLE_CHEST_CONTENTS",
        Table {
            id: "minecraft:chests/nether_bridge",
            rolls: (2, 5),
            book: None,
        },
    ),
    (
        "DesertPyramid.CHEST_LOOT_TABLE",
        Table {
            id: "minecraft:chests/desert_pyramid",
            rolls: (2, 6),
            book: None,
        },
    ),
    (
        "JunglePyramid.CHEST_LOOT_TABLE",
        Table {
            id: "minecraft:chests/jungle_temple",
            rolls: (2, 6),
            book: None,
        },
    ),
    (
        "JunglePyramid.DISPENSER_LOOT_TABLE",
        Table {
            id: "minecraft:chests/jungle_temple_dispenser",
            rolls: (2, 2),
            book: None,
        },
    ),
    (
        "Library.POSSIBLE_CHEST_CONTENT",
        Table {
            id: "minecraft:chests/stronghold_library",
            rolls: (1, 4),
            book: Some(Book {
                min: 1,
                max: 5,
                weight: 2,
            }),
        },
    ),
    (
        "SquareRoom.POSSIBLE_CHEST_CONTENT",
        Table {
            id: "minecraft:chests/stronghold_crossing",
            rolls: (1, 4),
            book: Some(Book {
                min: 1,
                max: 1,
                weight: 1,
            }),
        },
    ),
    (
        "ChestCorridor.POSSIBLE_CHEST_CONTENT",
        Table {
            id: "minecraft:chests/stronghold_corridor",
            rolls: (2, 3),
            book: Some(Book {
                min: 1,
                max: 1,
                weight: 1,
            }),
        },
    ),
    (
        "ServerWorld.BONUS_CHEST_LOOT_TABLE",
        Table {
            id: "minecraft:chests/spawn_bonus_chest",
            rolls: (10, 10),
            book: None,
        },
    ),
];

/// Parse `tools/ChestLootProbe18.java`'s dump.
///
/// Each line is `category <tab> item id <tab> damage <tab> min <tab> max
/// <tab> weight <tab> nbt`, in list order. Groups come out in first-seen
/// order. 1.8's chest lists carry no NBT, so a line with any is an error
/// rather than a silently dropped row.
pub fn parse_dump(text: &str) -> Result<Vec<(String, Vec<Row>)>> {
    let mut groups: Vec<(String, Vec<Row>)> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 7 {
            return Err(format!("line {}: {} fields, want 7", number + 1, fields.len()).into());
        }
        if !fields[6].is_empty() {
            return Err(format!("line {}: chest loot with NBT {}", number + 1, fields[6]).into());
        }
        let id: i16 = fields[1]
            .parse()
            .map_err(|_| format!("line {}: item id {:?}", number + 1, fields[1]))?;
        let damage: i16 = fields[2]
            .parse()
            .map_err(|_| format!("line {}: damage {:?}", number + 1, fields[2]))?;
        let count = |field: usize, what: &str| -> Result<i32> {
            fields[field]
                .parse()
                .map_err(|_| format!("line {}: {what} {:?}", number + 1, fields[field]).into())
        };
        let min = count(3, "min count")?;
        let max = count(4, "max count")?;
        let weight = count(5, "weight")?;
        let stack = legacy_1_8_9::stack(id, 1, damage, &Tag::End).ok_or_else(|| {
            format!(
                "line {}: item {id}:{damage} has no current item",
                number + 1
            )
        })?;
        let row = Row {
            item: stack.item.to_string(),
            min,
            max,
            weight,
        };
        match groups.iter_mut().find(|(name, _)| name == fields[0]) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((fields[0].to_owned(), vec![row])),
        }
    }
    Ok(groups)
}

/// The chest table JSON for one structure: one pool with 1.8's rolls, one
/// weighted entry per list row in list order, then the random book.
///
/// A row's count becomes `set_count` (a uniform range, or a constant when the
/// range is one stack). The book is a `book` enchanted with
/// `enchant_with_levels` 30 restricted to 1.8's 25 enchantments, the way
/// `EnchantmentHelper.addRandomEnchantment` turns the book into an
/// `enchanted_book`.
pub fn table_json(table: &Table, rows: &[Row]) -> Value {
    let mut entries: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut entry = json!({
                "type": "minecraft:item",
                "name": row.item,
                "weight": row.weight,
            });
            if row.min != 1 || row.max != 1 {
                entry["modifier"] = set_count(row.min, row.max);
            }
            entry
        })
        .collect();
    if let Some(book) = table.book {
        let mut functions = Vec::new();
        if book.min != 1 || book.max != 1 {
            functions.push(set_count(book.min, book.max));
        }
        functions.push(json!({
            "type": "minecraft:enchant_with_levels",
            "levels": 30,
            "options": ENCHANTING_TABLE_1_8_9,
        }));
        entries.push(json!({
            "type": "minecraft:item",
            "name": "minecraft:book",
            "weight": book.weight,
            "modifier": {
                "type": "minecraft:sequence",
                "functions": functions,
            },
        }));
    }
    json!({
        "type": "minecraft:chest",
        "pools": [{
            "rolls": rolls(table.rolls),
            "entries": entries,
        }],
    })
}

/// 1.8's `min + nextInt(max - min + 1)`.
fn rolls((min, max): (i32, i32)) -> Value {
    if min == max {
        json!(min)
    } else {
        json!({"type": "minecraft:uniform", "min": min, "max": max})
    }
}

fn set_count(min: i32, max: i32) -> Value {
    json!({
        "type": "minecraft:set_count",
        "count": rolls((min, max)),
    })
}

#[cfg(test)]
mod tests {
    use super::{TABLES, parse_dump, table_json};

    /// Every 1.8 chest the engine can build has one table, with the count its
    /// structure passes to `fillInventory`.
    #[test]
    fn the_dump_categories_are_1_8s_eleven_chests() {
        let table = |category: &str| {
            TABLES
                .iter()
                .find(|(name, _)| *name == category)
                .map(|(_, table)| table)
                .unwrap_or_else(|| panic!("{category}"))
        };
        assert_eq!(TABLES.len(), 11);
        assert_eq!(
            table("DungeonFeature.field_10161").id,
            "minecraft:chests/simple_dungeon"
        );
        assert_eq!(table("DungeonFeature.field_10161").rolls, (8, 8));
        assert_eq!(table("MineshaftPieces.LOOT_TABLES").rolls, (3, 6));
        assert_eq!(
            table("BlacksmithHouse.POSSIBLE_CHEST_CONTENT").id,
            "minecraft:chests/village/village_weaponsmith"
        );
        assert_eq!(
            table("BlacksmithHouse.POSSIBLE_CHEST_CONTENT").rolls,
            (3, 8)
        );
        assert_eq!(
            table("JunglePyramid.DISPENSER_LOOT_TABLE").id,
            "minecraft:chests/jungle_temple_dispenser"
        );
        assert_eq!(table("JunglePyramid.DISPENSER_LOOT_TABLE").rolls, (2, 2));
        // The library's book is `method_6347(random, 1, 5, 2)`; every other
        // one is `method_4610`'s single weight-1 book.
        assert_eq!(
            table("Library.POSSIBLE_CHEST_CONTENT").book,
            Some(super::Book {
                min: 1,
                max: 5,
                weight: 2
            })
        );
        assert_eq!(
            table("ChestCorridor.POSSIBLE_CHEST_CONTENT").book,
            Some(super::Book {
                min: 1,
                max: 1,
                weight: 1
            })
        );
        // A fortress chest is 2 + nextInt(4); the bonus chest fills 10 slots.
        assert_eq!(table("AbstractPiece.POSSIBLE_CHEST_CONTENTS").rolls, (2, 5));
        assert_eq!(table("ServerWorld.BONUS_CHEST_LOOT_TABLE").rolls, (10, 10));
    }

    /// The dump's numeric ids flatten to the current items, including the
    /// damage variants; a missing item is an error, not a silent drop.
    #[test]
    fn the_dump_flattens_1_8_ids_and_metadata() {
        let dump = concat!(
            "DungeonFeature.field_10161\t265\t0\t1\t4\t10\t\n",
            "DungeonFeature.field_10161\t351\t4\t4\t9\t5\t\n",
            "DungeonFeature.field_10161\t2256\t0\t1\t1\t4\t\n",
            "MineshaftPieces.LOOT_TABLES\t49\t0\t3\t7\t5\t\n",
        );
        let groups = parse_dump(dump).unwrap();
        assert_eq!(groups.len(), 2);
        let (category, rows) = &groups[0];
        assert_eq!(category, "DungeonFeature.field_10161");
        assert_eq!(rows[0].item, "minecraft:iron_ingot");
        assert_eq!(rows[1].item, "minecraft:lapis_lazuli");
        assert_eq!(rows[1].min, 4);
        assert_eq!(rows[1].max, 9);
        assert_eq!(rows[2].item, "minecraft:music_disc_13");
        assert_eq!(groups[1].1[0].item, "minecraft:obsidian");
    }

    /// A generated table keeps 1.8's weights, its uniform counts, and names
    /// the book last with 1.8's 25 enchantments at level 30.
    #[test]
    fn a_generated_table_keeps_1_8_weights_counts_and_book() {
        let table = TABLES
            .iter()
            .find(|(name, _)| *name == "DungeonFeature.field_10161")
            .map(|(_, table)| table)
            .unwrap();
        let rows = super::parse_dump(concat!(
            "DungeonFeature.field_10161\t329\t0\t1\t1\t10\t\n",
            "DungeonFeature.field_10161\t265\t0\t1\t4\t10\t\n",
        ))
        .unwrap()
        .remove(0)
        .1;
        let json = table_json(table, &rows);
        assert_eq!(json["type"], "minecraft:chest");
        assert_eq!(json["pools"][0]["rolls"], 8);
        let entries = json["pools"][0]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["name"], "minecraft:saddle");
        assert_eq!(entries[0]["weight"], 10);
        assert!(entries[0].get("modifier").is_none());
        assert_eq!(entries[1]["name"], "minecraft:iron_ingot");
        assert_eq!(
            entries[1]["modifier"]["count"],
            serde_json::json!({"type": "minecraft:uniform", "min": 1, "max": 4})
        );
        let book = &entries[2];
        assert_eq!(book["name"], "minecraft:book");
        assert_eq!(book["weight"], 1);
        let functions = book["modifier"]["functions"].as_array().unwrap();
        assert_eq!(functions[0]["type"], "minecraft:enchant_with_levels");
        assert_eq!(functions[0]["levels"], 30);
        assert_eq!(
            functions[0]["options"].as_array().unwrap().len(),
            crate::enchant::ENCHANTING_TABLE_1_8_9.len()
        );
        // No Mending, Frost Walker or other post-1.8 enchantment.
        let names: Vec<&str> = functions[0]["options"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|name| name.as_str())
            .collect();
        assert!(!names.contains(&"minecraft:mending"));
        assert!(!names.contains(&"minecraft:frost_walker"));
        assert!(names.contains(&"minecraft:protection"));
    }

    /// The library's book is a stack of one to five books, and the table's
    /// pool rolls one to four times.
    #[test]
    fn the_library_rolls_one_to_four_times_and_can_place_five_books() {
        let table = TABLES
            .iter()
            .find(|(name, _)| *name == "Library.POSSIBLE_CHEST_CONTENT")
            .map(|(_, table)| table)
            .unwrap();
        let rows = super::parse_dump("Library.POSSIBLE_CHEST_CONTENT\t340\t0\t1\t3\t20\t\n")
            .unwrap()
            .remove(0)
            .1;
        let json = table_json(table, &rows);
        assert_eq!(
            json["pools"][0]["rolls"],
            serde_json::json!({"type": "minecraft:uniform", "min": 1, "max": 4})
        );
        let book = &json["pools"][0]["entries"][1];
        assert_eq!(book["weight"], 2);
        assert_eq!(
            book["modifier"]["functions"][0]["count"],
            serde_json::json!({"type": "minecraft:uniform", "min": 1, "max": 5})
        );
    }

    /// The generated file is in the shape the loot engine reads: it loads
    /// from a resource root and fills a 27-slot chest with 1.8's items.
    #[test]
    fn a_generated_table_fills_a_chest_through_the_loot_engine() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("horizon-chest-1-8-9-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data/minecraft/loot_table/chests")).unwrap();
        fs::write(
            root.join("items.json"),
            r#"{"minecraft:saddle":{},"minecraft:iron_ingot":{},"minecraft:book":{},
                "minecraft:enchanted_book":{"minecraft:max_stack_size":1}}"#,
        )
        .unwrap();
        let table = TABLES
            .iter()
            .find(|(name, _)| *name == "DungeonFeature.field_10161")
            .map(|(_, table)| table)
            .unwrap();
        let rows = super::parse_dump(concat!(
            "DungeonFeature.field_10161\t329\t0\t1\t1\t10\t\n",
            "DungeonFeature.field_10161\t265\t0\t1\t4\t10\t\n",
        ))
        .unwrap()
        .remove(0)
        .1;
        fs::write(
            root.join("data/minecraft/loot_table/chests/simple_dungeon.json"),
            serde_json::to_vec(&table_json(table, &rows)).unwrap(),
        )
        .unwrap();
        // A book-only table proves the random-book entry on its own: every
        // roll becomes one enchanted book, split to the item's stack size 1.
        // Both tables are on disk before the load: tables are cached per root.
        fs::write(
            root.join("data/minecraft/loot_table/chests/book_only.json"),
            serde_json::to_vec(&table_json(table, &[])).unwrap(),
        )
        .unwrap();
        let data =
            horizon_world::loot::LootData::load(&root, &horizon_world::loot::PackOverlay::NONE)
                .unwrap();
        let stacks: Vec<_> = data
            .fill("minecraft:chests/simple_dungeon", 42, 27, None)
            .into_iter()
            .flatten()
            .collect();
        assert!(!stacks.is_empty());
        for stack in &stacks {
            assert!(
                matches!(
                    stack.item.as_str(),
                    "minecraft:saddle" | "minecraft:iron_ingot" | "minecraft:enchanted_book"
                ),
                "{}",
                stack.item
            );
        }
        // A book-only table proves the random-book entry on its own: every
        // roll becomes one enchanted book, split to the item's stack size 1.
        fs::write(
            root.join("data/minecraft/loot_table/chests/book_only.json"),
            serde_json::to_vec(&table_json(table, &[])).unwrap(),
        )
        .unwrap();
        // A book-only table proves the random-book entry on its own: every
        // roll becomes one enchanted book, split to the item's stack size 1.
        fs::write(
            root.join("data/minecraft/loot_table/chests/book_only.json"),
            serde_json::to_vec(&table_json(table, &[])).unwrap(),
        )
        .unwrap();
        let books: Vec<_> = data
            .fill("minecraft:chests/book_only", 42, 27, None)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(books.len(), 8);
        for book in &books {
            assert_eq!(book.item, "minecraft:enchanted_book");
            assert_eq!(book.count, 1);
        }
        let _ = fs::remove_dir_all(&root);
    }
}
