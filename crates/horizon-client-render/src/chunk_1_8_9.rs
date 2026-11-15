//! Turn a decoded 1.8.9 chunk column into the renderer's resident chunk.
//!
//! 1.8 sends one state integer (`id << 4 | metadata`) per block and separate
//! 4-bit block-light and sky-light arrays, while the renderer stores registry
//! ids in a paletted container and one `(sky << 4) | block` nibble per block.
//! Biomes are one id per column in 1.8 and 4x4x4 cells per section here.
use crate::chunk::{BIOME_STRATEGY, Chunk, ColumnLayout, Light, Section};
use horizon_core::paletted::PalettedContainer;
use horizon_world::{chunk::BlockState, chunk_1_8_9::Chunk1_8};
use std::sync::Arc;

/// Decode a 1.8 chunk into `layout`. `resolve` maps a current block state to
/// the renderer's registry id; `biomes` is the 1.8 per-column biome id array
/// already resolved to the client's biome ids. A state the registry cannot
/// place becomes air, as vanilla reads an unknown block state: dropping the
/// whole column would leave a hole in the world.
///
/// Sections 1.8 did not send are air. In a skylight dimension they get full
/// sky light, which is how the 1.8 client filled the holes in its bit mask.
pub fn chunk(
    source: &Chunk1_8,
    layout: ColumnLayout,
    sky_light: bool,
    biomes: &[u16; 256],
    mut resolve: impl FnMut(&BlockState) -> Option<u32>,
) -> Chunk {
    let mut out = Chunk::with_layout(source.x, source.z, layout);
    let default_light = if sky_light { 0xF0 } else { 0x00 };
    for index in 0..layout.sections() {
        let section_y = layout.min_y() / 16 + index as i32;
        let mut values = [0u32; 4096];
        let mut light = [default_light; 4096];
        if (0..16).contains(&section_y)
            && let Some(section) = source.section(section_y as u8)
        {
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let i = (y << 8) | (z << 4) | x;
                        let state = horizon_world::legacy_1_8_9::block_state(
                            section.block_id(x, y, z),
                            section.metadata(x, y, z),
                        );
                        values[i] = resolve(&state).unwrap_or_else(|| {
                            log::debug!(
                                "1.8.9: chunk {} {} has no registry state for {}, drawing air",
                                source.x,
                                source.z,
                                state.name
                            );
                            0
                        });
                        light[i] = (section.sky_light(x, y, z) << 4) | section.block_light(x, y, z);
                    }
                }
            }
        }
        let mut cells = [0u32; 64];
        for y4 in 0..4 {
            for z4 in 0..4 {
                for x4 in 0..4 {
                    cells[(y4 << 2 | z4) << 2 | x4] = u32::from(biomes[((z4 * 4) * 16) + x4 * 4]);
                }
            }
        }
        let biomes = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
        let light = uniform(&light);
        out.sections[index] = Arc::new(Section {
            blocks: Section::from_blocks(&values),
            biomes,
            light,
        });
    }
    out.lit = true;
    out.light_loaded = true;
    out
}

fn uniform(values: &[u8; 4096]) -> Light {
    let first = values[0];
    if values.iter().all(|&v| v == first) {
        Light::Uniform(first)
    } else {
        Light::Full(Box::new(*values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::chunk_1_8_9::Section1_8;

    fn stone() -> u32 {
        1
    }
    fn grass() -> u32 {
        2
    }
    fn resolve(state: &BlockState) -> Option<u32> {
        match state.name.as_str() {
            "minecraft:air" => Some(0),
            "minecraft:stone" => Some(stone()),
            "minecraft:grass_block" => Some(grass()),
            _ => None,
        }
    }

    #[test]
    fn blocks_light_and_biomes_cross_over() {
        let mut source = Chunk1_8::new(2, 3);
        let mut section = Section1_8::new(0);
        section.set_block(0, 0, 0, 1, 0);
        section.set_block(1, 0, 0, 2, 0);
        section.set_sky_light(0, 0, 0, 15);
        set_light(&mut section);
        source.sections.push(section);

        let layout = ColumnLayout::new(0, 256).unwrap();
        let mut plain = [7u16; 256];
        for (i, value) in plain.iter_mut().enumerate() {
            *value = 7 + (i % 3) as u16;
        }
        let chunk = chunk(&source, layout, true, &plain, resolve);
        assert_eq!((chunk.x, chunk.z), (2, 3));
        assert_eq!(chunk.sections.len(), 16);
        assert_eq!(chunk.sections[0].get(0), stone());
        assert_eq!(chunk.sections[0].get(1), grass());
        assert_eq!(chunk.sections[0].light(0), 0xF3);
        assert_eq!(chunk.sections[1].get(0), 0);
        assert_eq!(chunk.sections[1].light(0), 0xF0);
        assert_eq!(chunk.sections[0].biomes.get(0), 7);
        assert!(chunk.lit && chunk.light_loaded);
    }

    fn set_light(section: &mut Section1_8) {
        section.set_block_light(0, 0, 0, 3);
    }

    #[test]
    fn nether_sections_have_no_sky_light() {
        let source = Chunk1_8::new(0, 0);
        let layout = ColumnLayout::new(0, 256).unwrap();
        let chunk = chunk(&source, layout, false, &[0; 256], resolve);
        assert_eq!(chunk.sections[0].light(0), 0);
    }

    #[test]
    fn unknown_states_draw_as_air_instead_of_dropping_the_column() {
        let mut source = Chunk1_8::new(0, 0);
        let mut section = Section1_8::new(0);
        section.set_block(0, 0, 0, 35, 14); // red wool, not in the test map
        source.sections.push(section);
        let layout = ColumnLayout::new(0, 256).unwrap();
        let decoded = chunk(&source, layout, true, &[0; 256], resolve);
        assert_eq!(decoded.sections[0].get(0), 0);
        assert!(decoded.lit && decoded.light_loaded);
    }

    /// Every state in the real 1.8.9 save must reach a registry id: one the
    /// registry cannot place used to drop its whole column from a server's
    /// terrain, which is how a multiplayer world grew holes.
    #[test]
    fn a_real_1_8_chunk_resolves_every_state() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let facts = root.join(".local/horizon-client/1.8.9/facts.json");
        let region = root.join(".local/minecraft/1.8.9/server-run/world/region/r.0.0.mca");
        let (Ok(facts), Ok(region)) = (
            std::fs::File::open(facts),
            horizon_world::region::RegionFile::open(region),
        ) else {
            eprintln!("skipped: no prepared 1.8 resources or saved world");
            return;
        };
        let registry = horizon_world::registry::BlockRegistry::from_facts_json(facts).unwrap();
        let mut region = region;
        let positions: Vec<_> = region.chunks().map(|(pos, _)| pos).collect();
        assert!(!positions.is_empty(), "the saved region holds no chunks");
        let layout = ColumnLayout::VANILLA;
        let biomes = [7u16; 256];
        let mut seen = std::collections::BTreeSet::new();
        let mut exact_misses = std::collections::BTreeSet::new();
        let mut decoded = false;
        for pos in positions {
            let record = region.read(pos).unwrap().expect("listed chunk reads back");
            let nbt = record.nbt(horizon_world::nbt::Limits::default()).unwrap();
            let source = Chunk1_8::from_nbt(&nbt).unwrap();
            // Collect distinct legacy ids; mapping every one of the save's 22
            // million blocks makes this a five-minute test.
            for section in &source.sections {
                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            seen.insert((section.block_id(x, y, z), section.metadata(x, y, z)));
                        }
                    }
                }
            }
            // One full decode keeps the column path covered.
            if !decoded {
                let column = chunk(&source, layout, true, &biomes, |state| {
                    registry.id_filled(state).map(|id| id.0)
                });
                assert_eq!(column.sections.len(), 24);
                decoded = true;
            }
        }
        let states: Vec<_> = seen
            .into_iter()
            .map(|(id, meta)| horizon_world::legacy_1_8_9::block_state(id, meta))
            .filter(|state| state.name != "minecraft:air")
            .collect();
        assert!(!states.is_empty());
        for state in &states {
            if registry.id(state).is_none() {
                exact_misses.insert(state);
            }
        }
        let unresolved: Vec<_> = states
            .iter()
            .filter(|state| registry.id_filled(state).is_none())
            .collect();
        assert!(unresolved.is_empty(), "unresolved states: {unresolved:?}");
        assert!(
            exact_misses.is_empty(),
            "states only the fallback resolves: {exact_misses:?}"
        );
    }
}
