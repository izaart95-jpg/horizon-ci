//! Build the 1.8.9 crafting recipe data from a `RecipeDispatcher` dump.
//!
//! 1.8 has no recipe JSON: its recipes are code, registered in
//! `RecipeDispatcher` and the special `RecipeType`s. The dump this reads is
//! produced by `tools/java/RecipeProbe18.java`, which prints every recipe in
//! registration order with numeric item ids:
//!
//! ```text
//! SHAPED <w> <h> <result> <w*h cells...>
//! SHAPELESS <result> <ingredients...>
//! OTHER <RecipeType class> <result>
//! ```
//!
//! A stack is `id:count@damage` or `-`; damage `32767` is 1.8's "any
//! metadata" wildcard. This tool decodes every stack through the same
//! datafixer the game uses (`legacy_1_8_9::stack`) so the output speaks the
//! engine's flattened names (`minecraft:oak_planks`, not `minecraft:planks`
//! with damage 0), expands each wildcard to its variants, and writes modern
//! `data/minecraft/recipe/*.json`.
//!
//! The 1.8 special recipes are written as their current equivalents where one
//! exists: armour dyeing, firework rocket/star/fade, banner duplication, book
//! cloning, map cloning, map extending and item repair. 1.8's banner-pattern
//! recipe has no current equivalent and is recorded as debt in
//! `docs/mc-1.8.9.md`.
//!
//! ```sh
//! java -cp <1.8.9 named jar + libraries> RecipeProbe18 > recipes18.txt
//! cargo run -p horizon-client --example prepare_1_8_9_recipes -- \
//!     --recipes recipes18.txt --output .local/horizon-client/1.8.9
//! ```

use horizon_items::legacy_1_8_9;
use horizon_world::nbt::Tag;
use serde_json::{Map, Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut recipes = PathBuf::new();
    let mut output = PathBuf::from(".local/horizon-client/1.8.9");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--recipes" => recipes = args.next().ok_or("--recipes needs a path")?.into(),
            "--output" => output = args.next().ok_or("--output needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    if recipes.as_os_str().is_empty() {
        return Err("--recipes needs the RecipeProbe18 dump".into());
    }
    let text = fs::read_to_string(&recipes)?;
    let dir = output.join("data/minecraft/recipe");
    // The root's `data` tree is still the 26.3 one (recorded as debt); replace
    // the whole recipe directory (26.3 nests some recipes in subdirectories),
    // or both sets would load and overlap.
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;

    let mut decoder = Decoder::default();
    let mut written = 0usize;
    let mut skipped = Vec::new();
    let mut specials: Vec<(String, Value)> = Vec::new();
    let mut index = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields[0] {
            "SHAPED" => {
                let width: usize = fields[1].parse()?;
                let height: usize = fields[2].parse()?;
                let result = Stack::parse(fields[3])?;
                let cells: Vec<Stack> = fields[4..]
                    .iter()
                    .map(|s| Stack::parse(s))
                    .collect::<Result<_, _>>()?;
                if cells.len() != width * height {
                    skipped.push(format!(
                        "{line}: {width}x{height} has {} cells",
                        cells.len()
                    ));
                    continue;
                }
                // 1.8's map upscale is a `ShapedRecipeType` subclass with a
                // count-0 placeholder result; its `getResult` copies the map
                // and sets `map_is_scaling`. The current `MapExtending`
                // special is the same recipe.
                if result.count == 0 {
                    specials.push((
                        "map_extending".to_owned(),
                        json!({
                            "type": "minecraft:crafting_special_mapextending",
                            "map": "minecraft:filled_map",
                            "material": "minecraft:paper",
                            "result": {"id": "minecraft:filled_map"},
                        }),
                    ));
                    continue;
                }
                let Some((result, count)) = decoder.result(&result) else {
                    skipped.push(format!("{line}: result does not decode"));
                    continue;
                };
                let mut by_ingredient: HashMap<Vec<String>, char> = HashMap::new();
                let mut key = Map::new();
                let mut pattern = Vec::with_capacity(height);
                let mut next = b'A';
                let mut ok = true;
                for row in 0..height {
                    let mut text = String::with_capacity(width);
                    for column in 0..width {
                        let cell = &cells[row * width + column];
                        if cell.empty() {
                            text.push(' ');
                            continue;
                        }
                        let Some(items) = decoder.ingredient(cell) else {
                            skipped.push(format!("{line}: ingredient {cell:?} does not decode"));
                            ok = false;
                            break;
                        };
                        let c = *by_ingredient.entry(items.clone()).or_insert_with(|| {
                            let c = next as char;
                            next += 1;
                            key.insert(c.to_string(), ingredient_json(&items));
                            c
                        });
                        text.push(c);
                    }
                    if !ok {
                        break;
                    }
                    pattern.push(text);
                }
                if !ok {
                    continue;
                }
                let name = format!("1_8_9_{index:04}_{}", slug(&result));
                index += 1;
                written += 1;
                write_json(
                    &dir.join(format!("{name}.json")),
                    &json!({
                        "type": "minecraft:crafting_shaped",
                        "pattern": pattern,
                        "key": Value::Object(key),
                        "result": {"id": result, "count": count},
                    }),
                )?;
            }
            "SHAPELESS" => {
                let result = Stack::parse(fields[1])?;
                let Some((result, count)) = decoder.result(&result) else {
                    skipped.push(format!("{line}: result does not decode"));
                    continue;
                };
                let mut ingredients = Vec::new();
                let mut ok = true;
                for token in &fields[2..] {
                    let cell = Stack::parse(token)?;
                    match decoder.ingredient(&cell) {
                        Some(items) => ingredients.push(ingredient_json(&items)),
                        None => {
                            skipped.push(format!("{line}: ingredient {cell:?} does not decode"));
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    continue;
                }
                let name = format!("1_8_9_{index:04}_{}", slug(&result));
                index += 1;
                written += 1;
                write_json(
                    &dir.join(format!("{name}.json")),
                    &json!({
                        "type": "minecraft:crafting_shapeless",
                        "ingredients": ingredients,
                        "result": {"id": result, "count": count},
                    }),
                )?;
            }
            "OTHER" => specials_for(fields[1], &mut specials),
            other => skipped.push(format!("{line}: unknown kind {other}")),
        }
    }
    for (name, value) in &specials {
        write_json(&dir.join(format!("1_8_9_{name}.json")), value)?;
    }
    println!(
        "{} shaped/shapeless + {} special recipes -> {}",
        written,
        specials.len(),
        dir.display()
    );
    for line in &skipped {
        println!("skipped: {line}");
    }
    Ok(())
}

/// One 1.8 stack token. `-` is an empty cell; a count of 0 on an ingredient
/// still names an item (1.8 ignores ingredient counts).
#[derive(Clone, Copy, Debug)]
struct Stack {
    id: i16,
    count: i32,
    damage: i16,
}
impl Stack {
    fn parse(token: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if token == "-" {
            return Ok(Self {
                id: -1,
                count: 0,
                damage: 0,
            });
        }
        let (id, rest) = token.split_once(':').ok_or("stack needs id:count@damage")?;
        let (count, damage) = rest.split_once('@').ok_or("stack needs count@damage")?;
        Ok(Self {
            id: id.parse()?,
            count: count.parse()?,
            damage: damage.parse()?,
        })
    }
    fn empty(&self) -> bool {
        self.id < 0
    }
}

#[derive(Default)]
struct Decoder {
    /// The flattened item name of one id/damage, cached: the datafixer walks
    /// several schema versions per call.
    names: HashMap<(i16, i16), Option<String>>,
}
impl Decoder {
    fn name(&mut self, id: i16, damage: i16) -> Option<String> {
        if let Some(name) = self.names.get(&(id, damage)) {
            return name.clone();
        }
        // A recipe ingredient's count is irrelevant; a result's count is read
        // from the dump token. Decode every stack as one item.
        let name = legacy_1_8_9::stack(id, 1, damage, &Tag::End).map(|s| s.item.to_string());
        self.names.insert((id, damage), name.clone());
        name
    }
    fn result(&mut self, stack: &Stack) -> Option<(String, i32)> {
        self.name(stack.id, stack.damage)
            .map(|name| (name, stack.count.max(1)))
    }
    /// A recipe ingredient: a fixed damage is one item; damage 32767 is every
    /// metadata value the id registers, in damage order.
    fn ingredient(&mut self, stack: &Stack) -> Option<Vec<String>> {
        if stack.damage == 32767 {
            let mut names = Vec::new();
            for damage in 0..=15 {
                if let Some(name) = self.name(stack.id, damage)
                    && !names.contains(&name)
                {
                    names.push(name);
                }
            }
            return (!names.is_empty()).then_some(names);
        }
        self.name(stack.id, stack.damage).map(|name| vec![name])
    }
}

fn ingredient_json(items: &[String]) -> Value {
    if items.len() == 1 {
        json!(items[0])
    } else {
        json!(items)
    }
}
fn slug(name: &str) -> String {
    name.strip_prefix("minecraft:")
        .unwrap_or(name)
        .replace(['/', ':'], "_")
}
fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    fs::write(path, text)?;
    Ok(())
}

/// The 1.8 special `RecipeType`s. Their ingredients are fixed in code, so the
/// dump only names the class.
fn specials_for(kind: &str, out: &mut Vec<(String, Value)>) {
    let dyes = [
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
    ];
    // 1.8's `Items.DYE` damage order: four of them flattened to items that
    // are not named `*_dye` (ink sac, cocoa beans, lapis lazuli, bone meal).
    let dye_items: Vec<String> = [
        "ink_sac",
        "red_dye",
        "green_dye",
        "cocoa_beans",
        "lapis_lazuli",
        "purple_dye",
        "cyan_dye",
        "light_gray_dye",
        "gray_dye",
        "pink_dye",
        "lime_dye",
        "yellow_dye",
        "light_blue_dye",
        "magenta_dye",
        "orange_dye",
        "bone_meal",
    ]
    .iter()
    .map(|d| format!("minecraft:{d}"))
    .collect();
    match kind {
        // `ArmorDyeRecipeType`: any leather armour plus one or more dyes.
        "ArmorDyeRecipeType" => {
            for piece in ["helmet", "chestplate", "leggings", "boots"] {
                let item = format!("minecraft:leather_{piece}");
                out.push((
                    format!("armor_dye_{piece}"),
                    json!({
                        "type": "minecraft:crafting_dye",
                        "target": item,
                        "dye": dye_items,
                        "result": {"id": item},
                    }),
                ));
            }
        }
        // `FireworkRecipeType`: rocket, star and fade (the class has all three
        // branches; 1.8's star uses any skull as the creeper shape).
        "FireworkRecipeType" => {
            out.push((
                "firework_rocket".to_owned(),
                json!({
                    "type": "minecraft:crafting_special_firework_rocket",
                    "shell": "minecraft:paper",
                    "fuel": "minecraft:gunpowder",
                    "star": "minecraft:firework_star",
                    "result": {"id": "minecraft:firework_rocket", "count": 3},
                }),
            ));
            out.push((
                "firework_star".to_owned(),
                json!({
                    "type": "minecraft:crafting_special_firework_star",
                    "dye": dye_items,
                    "fuel": "minecraft:gunpowder",
                    "trail": "minecraft:diamond",
                    "twinkle": "minecraft:glowstone_dust",
                    "shapes": {
                        "burst": "minecraft:feather",
                        "creeper": [
                            "minecraft:skeleton_skull",
                            "minecraft:wither_skeleton_skull",
                            "minecraft:zombie_head",
                            "minecraft:player_head",
                            "minecraft:creeper_head",
                        ],
                        "large_ball": "minecraft:fire_charge",
                        "star": "minecraft:gold_nugget",
                    },
                    "result": {"id": "minecraft:firework_star"},
                }),
            ));
            out.push((
                "firework_star_fade".to_owned(),
                json!({
                    "type": "minecraft:crafting_special_firework_star_fade",
                    "dye": dye_items,
                    "target": "minecraft:firework_star",
                    "result": {"id": "minecraft:firework_star"},
                }),
            ));
        }
        // `PatternRecipeType` is banner duplication: a patterned banner plus a
        // blank one of the same colour.
        "PatternRecipeType" => {
            for color in dyes {
                let item = format!("minecraft:{color}_banner");
                out.push((
                    format!("banner_duplicate_{color}"),
                    json!({
                        "type": "minecraft:crafting_special_bannerduplicate",
                        "banner": item,
                        "result": {"id": item},
                    }),
                ));
            }
        }
        // `BookCloningRecipeType`: one written book plus writable books; the
        // generation limit is 0..1 in 1.8 (the current default).
        "BookCloningRecipeType" => out.push((
            "book_cloning".to_owned(),
            json!({
                "type": "minecraft:crafting_special_bookcloning",
                "source": "minecraft:written_book",
                "material": "minecraft:writable_book",
                "result": {"id": "minecraft:written_book"},
            }),
        )),
        // `MapCloningRecipeType`: one filled map plus empty maps.
        "MapCloningRecipeType" => out.push((
            "map_cloning".to_owned(),
            json!({
                "type": "minecraft:crafting_transmute",
                "input": "minecraft:filled_map",
                "material": "minecraft:map",
                "material_count": {"min": 1, "max": 8},
                "add_material_count_to_result": true,
                "result": {},
            }),
        )),
        // `RepairingRecipeType`: two damaged copies of one item.
        "RepairingRecipeType" => out.push((
            "repair_item".to_owned(),
            json!({"type": "minecraft:crafting_special_repairitem"}),
        )),
        // `CopyingRecipeType`: a banner plus a pattern item, or a 3x3 dye
        // layout, adds the first matching pattern. The order is the 1.8
        // `BannerPattern` enum's (base is not craftable).
        "CopyingRecipeType" => {
            let banners: Vec<String> = dyes
                .iter()
                .map(|d| format!("minecraft:{d}_banner"))
                .collect();
            let layout = |rows: [&str; 3]| json!(rows);
            let mut patterns = Vec::new();
            let mut push = |pattern: &str, ingredient: Option<&str>, rows: Option<Value>| {
                let mut p = json!({"pattern": format!("minecraft:{pattern}")});
                if let Some(ingredient) = ingredient {
                    p["ingredient"] = json!(ingredient);
                }
                if let Some(rows) = rows {
                    p["layout"] = rows;
                }
                patterns.push(p);
            };
            push(
                "square_bottom_left",
                None,
                Some(layout(["   ", "   ", "#  "])),
            );
            push(
                "square_bottom_right",
                None,
                Some(layout(["   ", "   ", "  #"])),
            );
            push("square_top_left", None, Some(layout(["#  ", "   ", "   "])));
            push(
                "square_top_right",
                None,
                Some(layout(["  #", "   ", "   "])),
            );
            push("stripe_bottom", None, Some(layout(["   ", "   ", "###"])));
            push("stripe_top", None, Some(layout(["###", "   ", "   "])));
            push("stripe_left", None, Some(layout(["#  ", "#  ", "#  "])));
            push("stripe_right", None, Some(layout(["  #", "  #", "  #"])));
            push("stripe_center", None, Some(layout([" # ", " # ", " # "])));
            push("stripe_middle", None, Some(layout(["   ", "###", "   "])));
            push(
                "stripe_downright",
                None,
                Some(layout(["#  ", " # ", "  #"])),
            );
            push("stripe_downleft", None, Some(layout(["  #", " # ", "#  "])));
            push("small_stripes", None, Some(layout(["# #", "# #", "   "])));
            push("cross", None, Some(layout(["# #", " # ", "# #"])));
            push("straight_cross", None, Some(layout([" # ", "###", " # "])));
            push("triangle_bottom", None, Some(layout(["   ", " # ", "# #"])));
            push("triangle_top", None, Some(layout(["# #", " # ", "   "])));
            push(
                "triangles_bottom",
                None,
                Some(layout(["   ", "# #", " # "])),
            );
            push("triangles_top", None, Some(layout([" # ", "# #", "   "])));
            push("diagonal_left", None, Some(layout(["## ", "#  ", "   "])));
            push(
                "diagonal_up_right",
                None,
                Some(layout(["   ", "  #", " ##"])),
            );
            push(
                "diagonal_up_left",
                None,
                Some(layout(["   ", "#  ", "## "])),
            );
            push("diagonal_right", None, Some(layout([" ##", "  #", "   "])));
            push("circle", None, Some(layout(["   ", " # ", "   "])));
            push("rhombus", None, Some(layout([" # ", "# #", " # "])));
            push("half_vertical", None, Some(layout(["## ", "## ", "## "])));
            push("half_horizontal", None, Some(layout(["###", "###", "   "])));
            push(
                "half_vertical_right",
                None,
                Some(layout([" ##", " ##", " ##"])),
            );
            push(
                "half_horizontal_bottom",
                None,
                Some(layout(["   ", "###", "###"])),
            );
            push("border", None, Some(layout(["###", "# #", "###"])));
            push("curly_border", Some("minecraft:vine"), None);
            push("creeper", Some("minecraft:creeper_head"), None);
            push("gradient", None, Some(layout(["# #", " # ", " # "])));
            push("gradient_up", None, Some(layout([" # ", " # ", "# #"])));
            push("bricks", Some("minecraft:bricks"), None);
            push("skull", Some("minecraft:wither_skeleton_skull"), None);
            push("flower", Some("minecraft:oxeye_daisy"), None);
            push("mojang", Some("minecraft:enchanted_golden_apple"), None);
            out.push((
                "banner_pattern".to_owned(),
                json!({
                    "type": "minecraft:crafting_special_bannerpattern",
                    "banner": banners,
                    "patterns": patterns,
                }),
            ));
        }
        other => println!("unhandled special recipe {other}"),
    }
}
