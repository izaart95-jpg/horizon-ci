//! Write the 1.8.9 painting variants, their textures and the `#placeable` tag.
//!
//! 1.8 has no `painting_variant` registry: `PaintingEntity.PaintingMotive` is
//! an enum. `tools/PaintingProbe18.java` dumps that enum from the real 1.8.9
//! jar, and this tool checks the dump against
//! `horizon_items::paintings_1_8_9::VARIANTS` before writing the current
//! `data/minecraft/painting_variant/*.json` files and the placeable tag. The
//! target's data root must not keep 26.3's larger painting set: a 1.8 save can
//! only name the 26 enum motives, and 26.3 art has no 1.8 texture.
//!
//! 1.8 also has no per-motive textures. Its 26 motives and the wooden back
//! patch live in one 256x256 `paintings_kristoffer_zetterstrand.png` atlas,
//! which the 1.8 renderer crops at run time. The current renderer wants
//! `textures/painting/<id>.png` and `painting/back`, so this tool crops them
//! out of the atlas the prepared root already carries from the 1.8 jar.
//!
//! ```sh
//! javac -cp <1.8.9 named jar> -d /tmp/opencode tools/PaintingProbe18.java
//! java -cp <1.8.9 named jar>:/tmp/opencode PaintingProbe18 > paintings-1.8.9.txt
//! cargo run --profile fast -p horizon-client --example prepare_1_8_9_paintings -- \
//!     --dump /tmp/opencode/paintings-1.8.9.txt --root .local/horizon-client/1.8.9
//! ```
use horizon_items::paintings_1_8_9::{self, ATLAS, ATLAS_SIZE, BACK_REGION, Image, VARIANTS};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut dump = PathBuf::from("/tmp/opencode/paintings-1.8.9.txt");
    let mut root = PathBuf::from(".local/horizon-client/1.8.9");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dump" => dump = args.next().ok_or("--dump needs a path")?.into(),
            "--root" => root = args.next().ok_or("--root needs a path")?.into(),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let parsed = paintings_1_8_9::parse_dump(&fs::read_to_string(&dump)?)?;
    if parsed.len() != VARIANTS.len() {
        return Err(format!("dump has {} motives, want {}", parsed.len(), VARIANTS.len()).into());
    }
    for (probe, known) in parsed.iter().zip(VARIANTS) {
        if probe.id != known.id
            || probe.save_name != known.save_name
            || (probe.width, probe.height) != (known.width, known.height)
            || probe.atlas != known.atlas
        {
            return Err(format!(
                "dump {} {} {}x{} at {:?} does not match {} {} {}x{} at {:?}",
                probe.id,
                probe.save_name,
                probe.width,
                probe.height,
                probe.atlas,
                known.id,
                known.save_name,
                known.width,
                known.height,
                known.atlas
            )
            .into());
        }
    }
    let data = root.join("data/minecraft");
    let variants = data.join("painting_variant");
    fs::create_dir_all(&variants)?;
    // Drop whatever set the root had before (26.3's 51 entries included).
    for entry in fs::read_dir(&variants)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json") {
            fs::remove_file(path)?;
        }
    }
    for variant in &VARIANTS {
        fs::write(
            variants.join(format!("{}.json", variant.id)),
            serde_json::to_vec_pretty(&paintings_1_8_9::variant_json(variant))?,
        )?;
    }
    let tag = data.join("tags/painting_variant/placeable.json");
    fs::create_dir_all(tag.parent().ok_or("tag has no parent")?)?;
    fs::write(
        &tag,
        serde_json::to_vec_pretty(&paintings_1_8_9::placeable_tag())?,
    )?;
    let textures = root.join("assets/minecraft/textures/painting");
    let atlas_path = textures.join(ATLAS);
    let atlas = read_rgba(&atlas_path)?;
    if (atlas.width, atlas.height) != (ATLAS_SIZE, ATLAS_SIZE) {
        return Err(format!(
            "{} is {}x{}, want {ATLAS_SIZE}x{ATLAS_SIZE}",
            atlas_path.display(),
            atlas.width,
            atlas.height
        )
        .into());
    }
    // Every current texture is one of these crops; anything else (26.3's 26
    // extra motives) has no 1.8 art and must not stay.
    for entry in fs::read_dir(&textures)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "png")
            && path.file_name().is_some_and(|n| n != ATLAS)
        {
            fs::remove_file(path)?;
        }
    }
    for variant in &VARIANTS {
        let image = paintings_1_8_9::crop(
            &atlas,
            variant.atlas.0,
            variant.atlas.1,
            variant.width as u32 * 16,
            variant.height as u32 * 16,
        )?;
        write_png(&textures.join(format!("{}.png", variant.id)), &image)?;
    }
    let (x, y, width, height) = BACK_REGION;
    let back = paintings_1_8_9::crop(&atlas, x, y, width, height)?;
    write_png(&textures.join("back.png"), &back)?;
    println!(
        "{} painting variants, {} textures and {} under {}",
        VARIANTS.len(),
        VARIANTS.len() + 1,
        tag.display(),
        variants.display()
    );
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
