//! Write the 1.8.9 banner patterns, their tags and their textures.
//!
//! 1.8 has no `banner_pattern` registry: `BannerBlockEntity.BannerPattern` is
//! an enum. `tools/BannerPatternProbe18.java` dumps that enum from the real
//! 1.8.9 jar, and this tool checks the dump against
//! `horizon_items::banner_patterns_1_8_9::PATTERNS` before writing the current
//! `data/minecraft/banner_pattern/*.json` files and their tags. The target's
//! data root must not keep 26.3's four post-1.8 patterns (flow, guster, globe,
//! piglin): a 1.8 save cannot name them and the jar has no texture for them.
//!
//! The textures need converting too. 1.8 paints a pattern where its texture
//! is opaque white and leaves the banner underneath where it is opaque black;
//! the current renderer wants the 1.9+ alpha mask. The 1.8 jar also names the
//! pole-and-cloth texture `entity/banner_base.png`, while the current banner
//! models ask for `entity/banner/banner_base`, so this copies it to that name.
//! Without both steps placed banners do not draw their cloth or come out
//! black.
//!
//! ```sh
//! javac -cp <1.8.9 named jar> -d /tmp/opencode tools/BannerPatternProbe18.java
//! java -cp <1.8.9 named jar>:/tmp/opencode BannerPatternProbe18 > banners-1.8.9.txt
//! cargo run --profile fast -p horizon-client --example prepare_1_8_9_banners -- \
//!     --dump /tmp/opencode/banners-1.8.9.txt --root .local/horizon-client/1.8.9
//! ```
use horizon_items::banner_patterns_1_8_9::{self, Image, PATTERNS};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut dump = PathBuf::from("/tmp/opencode/banners-1.8.9.txt");
    let mut root = PathBuf::from(".local/horizon-client/1.8.9");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dump" => dump = args.next().ok_or("--dump needs a path")?.into(),
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let parsed = banner_patterns_1_8_9::parse_dump(&fs::read_to_string(&dump)?)?;
    if parsed.len() != PATTERNS.len() {
        return Err(format!(
            "dump has {} patterns, want {}",
            parsed.len(),
            PATTERNS.len()
        )
        .into());
    }
    for (probe, known) in parsed.iter().zip(PATTERNS) {
        let ingredient_matches = match (&probe.ingredient, known.ingredient) {
            (None, None) => true,
            (Some((item, damage)), Some(ingredient)) => {
                item == ingredient.legacy_item && *damage == ingredient.legacy_damage
            }
            _ => false,
        };
        if probe.id != known.id || probe.save_code != known.save_code || !ingredient_matches {
            return Err(format!(
                "dump {} {} {:?} does not match {} {} {:?}",
                probe.id,
                probe.save_code,
                probe.ingredient,
                known.id,
                known.save_code,
                known.ingredient
            )
            .into());
        }
    }
    let data = root.join("data/minecraft");
    let patterns = data.join("banner_pattern");
    fs::create_dir_all(&patterns)?;
    // Drop whatever set the root had before (26.3's 43 entries included).
    for entry in fs::read_dir(&patterns)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json") {
            fs::remove_file(path)?;
        }
    }
    for pattern in &PATTERNS {
        fs::write(
            patterns.join(format!("{}.json", pattern.id)),
            serde_json::to_vec_pretty(&banner_patterns_1_8_9::pattern_json(pattern))?,
        )?;
    }
    write_tags(&data.join("tags/banner_pattern"))?;
    let textures = root.join("assets/minecraft/textures/entity");
    let banner_textures = textures.join("banner");
    // 1.8's own `base.png` is already the current white flag; only the 38
    // pattern masks change form.
    for pattern in PATTERNS.iter().filter(|p| p.id != "base") {
        let path = banner_textures.join(format!("{}.png", pattern.id));
        let image = read_rgba(&path)?;
        write_png(&path, &banner_patterns_1_8_9::alpha_mask(&image)?)?;
    }
    // 26.3's post-1.8 patterns have no 1.8 art.
    for stale in ["flow.png", "guster.png", "globe.png", "piglin.png"] {
        let path = banner_textures.join(stale);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    // The current banner models sample `entity/banner/banner_base`; 1.8 names
    // the same pole-and-cloth art one folder up.
    let source = textures.join("banner_base.png");
    let destination = banner_textures.join("banner_base.png");
    if source.is_file() {
        fs::copy(&source, &destination)?;
    } else if !destination.is_file() {
        return Err(format!(
            "{} is missing; prepare the 1.8 jar's textures first",
            source.display()
        )
        .into());
    }
    println!(
        "{} banner patterns, {} cropped textures and the 1.8 tags under {}",
        PATTERNS.len(),
        PATTERNS.len(),
        banner_textures.display()
    );
    Ok(())
}

/// The 1.8 pattern tags: the 32 layout patterns need no item, and the six
/// ingredient patterns map to the current item tag names.
fn write_tags(root: &Path) -> Result<(), Box<dyn Error>> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root.join("pattern_item"))?;
    fs::write(
        root.join("no_item_required.json"),
        serde_json::to_vec_pretty(&banner_patterns_1_8_9::no_item_required_tag())?,
    )?;
    // 26.3's item tag names for 1.8's six ingredient items (vine and bricks
    // are the Bordure Indented and Field Masoned pattern items today).
    let item_tags = [
        ("bordure_indented", "curly_border"),
        ("field_masoned", "bricks"),
        ("creeper", "creeper"),
        ("skull", "skull"),
        ("flower", "flower"),
        ("mojang", "mojang"),
    ];
    for (tag, pattern) in item_tags {
        let entry = PATTERNS
            .iter()
            .find(|p| p.id == pattern)
            .ok_or("ingredient pattern missing")?;
        fs::write(
            root.join(format!("pattern_item/{tag}.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "values": [format!("minecraft:{}", entry.id)],
            }))?,
        )?;
    }
    Ok(())
}

/// Decode a PNG to RGBA8 (the `png` crate expands palettes and strips 16-bit).
fn read_rgba(path: &Path) -> Result<Image, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut bytes)?;
    let channels = info.color_type.samples();
    let mut pixels = Vec::with_capacity((info.width * info.height * 4) as usize);
    for pixel in bytes[..info.buffer_size()].chunks_exact(channels) {
        match info.color_type {
            png::ColorType::Rgba => pixels.extend_from_slice(pixel),
            png::ColorType::Rgb => {
                pixels.extend_from_slice(&pixel[..3]);
                pixels.push(255);
            }
            png::ColorType::GrayscaleAlpha => {
                pixels.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
            png::ColorType::Grayscale => {
                pixels.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]);
            }
            png::ColorType::Indexed => return Err("palette did not expand".into()),
        }
    }
    Ok(Image {
        width: info.width,
        height: info.height,
        pixels,
    })
}

fn write_png(path: &Path, image: &Image) -> Result<(), Box<dyn Error>> {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        image.width,
        image.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&image.pixels)?;
    Ok(())
}
