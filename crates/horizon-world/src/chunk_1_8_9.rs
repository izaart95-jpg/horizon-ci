//! The 1.8.9 chunk NBT format (`Level` + 16³ sections).
//!
//! This is the format an Anvil region stores. It is deliberately separate from
//! [`crate::chunk`], whose paletted sections belong to the 26.3 target: 1.8 has
//! numeric block ids, a metadata nibble, an optional `Add` nibble for ids above
//! 255, and separate block/sky light arrays.
//!
//! The **network** chunk payload is not this NBT: 1.8 sends each section as
//! 4096 little-endian `u16` states (`id << 4 | metadata`) followed by block
//! light and, when the dimension has skylight, sky light. [`from_network`] and
//! [`from_bulk`] decode those payloads into the same [`Chunk1_8`] model.
//!
//! Nothing selects this module yet. It is the reader/writer stage 3 of
//! `docs/mc-1.8.9.md` needs; until the game consumes it, the 26.3 chunk code
//! remains the only path.
use crate::{
    compression::invalid,
    nbt::{Compound, JavaString, NamedTag, Tag},
};
use std::io;

const SIDE: usize = 16;
const SECTION_VOLUME: usize = SIDE * SIDE * SIDE;
const NIBBLE_VOLUME: usize = SECTION_VOLUME / 2;
const BIOME_COUNT: usize = SIDE * SIDE;
const HEIGHTMAP_COUNT: usize = SIDE * SIDE;

fn key(value: &str) -> JavaString {
    JavaString::from(value)
}

fn index(x: usize, y: usize, z: usize) -> usize {
    assert!(
        x < SIDE && y < SIDE && z < SIDE,
        "block outside a 1.8 section"
    );
    (y << 8) | (z << 4) | x
}

fn nibble(values: &[u8], i: usize) -> u8 {
    if i & 1 == 0 {
        values[i >> 1] & 0x0F
    } else {
        values[i >> 1] >> 4
    }
}

fn set_nibble(values: &mut [u8], i: usize, value: u8) {
    let byte = &mut values[i >> 1];
    if i & 1 == 0 {
        *byte = (*byte & 0xF0) | (value & 0x0F);
    } else {
        *byte = (*byte & 0x0F) | ((value & 0x0F) << 4);
    }
}

fn field<'a>(map: &'a Compound, name: &str) -> Option<&'a Tag> {
    map.get(&key(name))
}

fn byte_array(map: &Compound, name: &str, len: usize) -> io::Result<Option<Vec<u8>>> {
    match field(map, name) {
        None => Ok(None),
        Some(Tag::ByteArray(values)) if values.len() == len => Ok(Some(values.clone())),
        Some(Tag::ByteArray(values)) => Err(invalid(format!(
            "{name} has {} bytes, expected {len}",
            values.len()
        ))),
        Some(_) => Err(invalid(format!("{name} is not a byte array"))),
    }
}

/// One 16×16×16 section of a 1.8 chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section1_8 {
    y: u8,
    blocks: Vec<u8>,
    data: Vec<u8>,
    add: Option<Vec<u8>>,
    block_light: Option<Vec<u8>>,
    sky_light: Option<Vec<u8>>,
    extra: Compound,
}
impl Section1_8 {
    /// A new section with zero blocks, zero metadata and zero light.
    pub fn new(y: u8) -> Self {
        assert!(y < 16, "1.8 sections are numbered 0..=15");
        Self {
            y,
            blocks: vec![0; SECTION_VOLUME],
            data: vec![0; NIBBLE_VOLUME],
            add: None,
            block_light: Some(vec![0; NIBBLE_VOLUME]),
            sky_light: Some(vec![0; NIBBLE_VOLUME]),
            extra: Compound::new(),
        }
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    /// The 1.8 block id (0..=4095), combining `Blocks` and the optional `Add`.
    pub fn block_id(&self, x: usize, y: usize, z: usize) -> u16 {
        let i = index(x, y, z);
        let high = self.add.as_ref().map_or(0, |add| u16::from(nibble(add, i)));
        u16::from(self.blocks[i]) | high << 8
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, id: u16, metadata: u8) {
        let i = index(x, y, z);
        let id = id & 0x0FFF;
        self.blocks[i] = id as u8;
        set_nibble(&mut self.data, i, metadata & 0x0F);
        if id > 0xFF && self.add.is_none() {
            self.add = Some(vec![0; NIBBLE_VOLUME]);
        }
        if let Some(add) = &mut self.add {
            set_nibble(add, i, (id >> 8) as u8);
        }
    }

    pub fn metadata(&self, x: usize, y: usize, z: usize) -> u8 {
        nibble(&self.data, index(x, y, z))
    }

    /// The 1.8 global block state: `id << 4 | metadata`.
    pub fn block_state(&self, x: usize, y: usize, z: usize) -> i32 {
        i32::from(self.block_id(x, y, z)) << 4 | i32::from(self.metadata(x, y, z))
    }

    pub fn block_light(&self, x: usize, y: usize, z: usize) -> u8 {
        self.block_light
            .as_ref()
            .map_or(0, |light| nibble(light, index(x, y, z)))
    }

    pub fn set_block_light(&mut self, x: usize, y: usize, z: usize, value: u8) {
        let i = index(x, y, z);
        set_nibble(
            self.block_light
                .get_or_insert_with(|| vec![0; NIBBLE_VOLUME]),
            i,
            value,
        );
    }

    pub fn sky_light(&self, x: usize, y: usize, z: usize) -> u8 {
        self.sky_light
            .as_ref()
            .map_or(0, |light| nibble(light, index(x, y, z)))
    }

    pub fn set_sky_light(&mut self, x: usize, y: usize, z: usize, value: u8) {
        let i = index(x, y, z);
        set_nibble(
            self.sky_light.get_or_insert_with(|| vec![0; NIBBLE_VOLUME]),
            i,
            value,
        );
    }

    /// Replace both light arrays as the 1.8 file stores them (2,048 nibble
    /// bytes, low nibble first). `None` leaves that array out, which Nether
    /// and End columns need for `SkyLight`.
    pub fn set_lights(&mut self, block: Option<Vec<u8>>, sky: Option<Vec<u8>>) {
        self.block_light = block;
        self.sky_light = sky;
    }

    /// Whether the section holds nothing worth saving: no block but air and
    /// no light.
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|&b| b == 0)
            && self
                .add
                .as_ref()
                .is_none_or(|add| add.iter().all(|&b| b == 0))
            && self
                .block_light
                .as_ref()
                .is_none_or(|light| light.iter().all(|&b| b == 0))
            && self
                .sky_light
                .as_ref()
                .is_none_or(|light| light.iter().all(|&b| b == 0))
    }

    pub fn from_nbt(tag: &Tag) -> io::Result<Self> {
        let Tag::Compound(map) = tag else {
            return Err(invalid("section is not a compound"));
        };
        let y = match field(map, "Y") {
            Some(Tag::Byte(y)) if (0..16).contains(y) => *y as u8,
            Some(Tag::Byte(y)) => return Err(invalid(format!("section Y {y} is outside 0..=15"))),
            Some(_) => return Err(invalid("section Y is not a byte")),
            None => return Err(invalid("section is missing Y")),
        };
        let blocks = byte_array(map, "Blocks", SECTION_VOLUME)?
            .ok_or_else(|| invalid("section is missing Blocks"))?;
        let data = byte_array(map, "Data", NIBBLE_VOLUME)?
            .ok_or_else(|| invalid("section is missing Data"))?;
        let mut extra = Compound::new();
        for (name, value) in map {
            let known = ["Y", "Blocks", "Data", "Add", "BlockLight", "SkyLight"]
                .iter()
                .any(|known| name == &key(known));
            if !known {
                extra.insert(name.clone(), value.clone());
            }
        }
        Ok(Self {
            y,
            blocks,
            data,
            add: byte_array(map, "Add", NIBBLE_VOLUME)?,
            block_light: byte_array(map, "BlockLight", NIBBLE_VOLUME)?,
            sky_light: byte_array(map, "SkyLight", NIBBLE_VOLUME)?,
            extra,
        })
    }

    pub fn to_nbt(&self) -> Tag {
        let mut map = self.extra.clone();
        map.insert(key("Y"), Tag::Byte(self.y as i8));
        map.insert(key("Blocks"), Tag::ByteArray(self.blocks.clone()));
        map.insert(key("Data"), Tag::ByteArray(self.data.clone()));
        if let Some(add) = &self.add {
            map.insert(key("Add"), Tag::ByteArray(add.clone()));
        }
        // 1.8's chunk loader always makes a nibble array from `BlockLight`;
        // a section without it must still write the 2,048 zero bytes.
        // `SkyLight` stays absent for Nether and End sections, which are the
        // only ones a 1.8 game writes without it.
        map.insert(
            key("BlockLight"),
            Tag::ByteArray(
                self.block_light
                    .clone()
                    .unwrap_or_else(|| vec![0; NIBBLE_VOLUME]),
            ),
        );
        if let Some(light) = &self.sky_light {
            map.insert(key("SkyLight"), Tag::ByteArray(light.clone()));
        }
        Tag::Compound(map)
    }
}

/// A 1.8 chunk column, as stored under `Level` in Anvil NBT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk1_8 {
    pub x: i32,
    pub z: i32,
    pub last_update: i64,
    pub terrain_populated: bool,
    pub sections: Vec<Section1_8>,
    pub biomes: Option<Vec<u8>>,
    pub height_map: Option<Vec<i32>>,
    pub tile_entities: Vec<Tag>,
    pub entities: Vec<Tag>,
    pub tile_ticks: Vec<Tag>,
    /// Fields this module does not model (`LightPopulated`, `InhabitedTime`, ...).
    pub extra: Compound,
}
impl Chunk1_8 {
    pub fn new(x: i32, z: i32) -> Self {
        Self {
            x,
            z,
            last_update: 0,
            terrain_populated: false,
            sections: Vec::new(),
            biomes: Some(vec![0; BIOME_COUNT]),
            height_map: Some(vec![0; HEIGHTMAP_COUNT]),
            tile_entities: Vec::new(),
            entities: Vec::new(),
            tile_ticks: Vec::new(),
            extra: Compound::new(),
        }
    }

    pub fn section(&self, y: u8) -> Option<&Section1_8> {
        self.sections.iter().find(|section| section.y() == y)
    }

    pub fn section_mut(&mut self, y: u8) -> Option<&mut Section1_8> {
        self.sections.iter_mut().find(|section| section.y() == y)
    }

    pub fn from_nbt(root: &NamedTag) -> io::Result<Self> {
        let Tag::Compound(root_map) = &root.tag else {
            return Err(invalid("chunk root is not a compound"));
        };
        let Some(Tag::Compound(level)) = field(root_map, "Level") else {
            return Err(invalid("chunk root has no Level compound"));
        };
        let required_int = |name: &str| match field(level, name) {
            Some(Tag::Int(v)) => Ok(*v),
            Some(_) => Err(invalid(format!("{name} is not an int"))),
            None => Err(invalid(format!("chunk is missing {name}"))),
        };
        let x = required_int("xPos")?;
        let z = required_int("zPos")?;
        let last_update = match field(level, "LastUpdate") {
            Some(Tag::Long(v)) => *v,
            Some(_) => return Err(invalid("LastUpdate is not a long")),
            None => 0,
        };
        let terrain_populated = match field(level, "TerrainPopulated") {
            Some(Tag::Byte(v)) => *v != 0,
            Some(_) => return Err(invalid("TerrainPopulated is not a byte")),
            None => false,
        };

        let sections = match field(level, "Sections") {
            Some(Tag::List(values)) => values
                .iter()
                .map(Section1_8::from_nbt)
                .collect::<io::Result<Vec<_>>>()?,
            Some(_) => return Err(invalid("Sections is not a list")),
            None => Vec::new(),
        };
        let mut seen = [false; 16];
        for section in &sections {
            if seen[section.y() as usize] {
                return Err(invalid(format!("duplicate section Y {}", section.y())));
            }
            seen[section.y() as usize] = true;
        }
        let mut sections = sections;
        sections.sort_by_key(Section1_8::y);

        let biomes = byte_array(level, "Biomes", BIOME_COUNT)?;
        let height_map = match field(level, "HeightMap") {
            Some(Tag::IntArray(values)) if values.len() == HEIGHTMAP_COUNT => Some(values.clone()),
            Some(Tag::IntArray(values)) => {
                return Err(invalid(format!(
                    "HeightMap has {} ints, expected {HEIGHTMAP_COUNT}",
                    values.len()
                )));
            }
            Some(_) => return Err(invalid("HeightMap is not an int array")),
            None => None,
        };
        let tag_list = |name: &str| -> io::Result<Vec<Tag>> {
            match field(level, name) {
                Some(Tag::List(values)) => {
                    if values
                        .iter()
                        .any(|value| !matches!(value, Tag::Compound(_)))
                    {
                        return Err(invalid(format!("{name} contains a non-compound")));
                    }
                    Ok(values.clone())
                }
                Some(_) => Err(invalid(format!("{name} is not a list"))),
                None => Ok(Vec::new()),
            }
        };
        let tile_entities = tag_list("TileEntities")?;
        let entities = tag_list("Entities")?;
        let tile_ticks = tag_list("TileTicks")?;

        let mut extra = Compound::new();
        for (name, value) in level {
            let known = [
                "xPos",
                "zPos",
                "LastUpdate",
                "TerrainPopulated",
                "V",
                "Sections",
                "Biomes",
                "HeightMap",
                "TileEntities",
                "Entities",
                "TileTicks",
            ]
            .iter()
            .any(|known| name == &key(known));
            if !known {
                extra.insert(name.clone(), value.clone());
            }
        }

        Ok(Self {
            x,
            z,
            last_update,
            terrain_populated,
            sections,
            biomes,
            height_map,
            tile_entities,
            entities,
            tile_ticks,
            extra,
        })
    }

    /// Write the chunk in the 1.8 layout. `V` is always written as 1.
    pub fn to_nbt(&self) -> NamedTag {
        let mut level = self.extra.clone();
        level.insert(key("xPos"), Tag::Int(self.x));
        level.insert(key("zPos"), Tag::Int(self.z));
        level.insert(key("LastUpdate"), Tag::Long(self.last_update));
        level.insert(
            key("TerrainPopulated"),
            Tag::Byte(if self.terrain_populated { 1 } else { 0 }),
        );
        level.insert(key("V"), Tag::Byte(1));
        level.insert(
            key("Sections"),
            Tag::List(self.sections.iter().map(Section1_8::to_nbt).collect()),
        );
        if let Some(biomes) = &self.biomes {
            level.insert(key("Biomes"), Tag::ByteArray(biomes.clone()));
        }
        if let Some(height_map) = &self.height_map {
            level.insert(key("HeightMap"), Tag::IntArray(height_map.clone()));
        }
        level.insert(key("TileEntities"), Tag::List(self.tile_entities.clone()));
        level.insert(key("Entities"), Tag::List(self.entities.clone()));
        level.insert(key("TileTicks"), Tag::List(self.tile_ticks.clone()));
        NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(Compound::from([(key("Level"), Tag::Compound(level))])),
        }
    }

    /// Write the chunk in the current save layout by running the same datafixer
    /// vanilla runs when it opens a 1.8 world.
    ///
    /// The fixes are chunk-level, not state-level: snowy grass, door halves,
    /// pumpkins and flower pots depend on the blocks or tile entities around
    /// them, so converting a 1.8 chunk means converting the whole column.
    pub fn to_current_nbt(&self, dimension: &str, generator: Option<&str>) -> NamedTag {
        let mut named = self.to_nbt();
        crate::datafix::update_chunk(&mut named.tag, dimension, generator);
        named
    }
}

/// The bytes one section occupies in a 1.8 network chunk.
fn network_section_len(sky_light: bool) -> usize {
    SECTION_VOLUME * 2 + NIBBLE_VOLUME + if sky_light { NIBBLE_VOLUME } else { 0 }
}

/// Decode a 1.8 `Map Chunk` payload (the bytes after the packet's data length).
///
/// `primary_bit_mask` selects the sections present; `sky_light` says whether
/// the dimension sends the sky-light array (overworld yes, Nether/End no);
/// `full_chunk` requires and consumes the 256-byte biome array. A non-full
/// chunk contains only the selected sections and must be merged into the
/// existing column, not replace it.
pub fn from_network(
    x: i32,
    z: i32,
    primary_bit_mask: u16,
    sky_light: bool,
    full_chunk: bool,
    data: &[u8],
) -> io::Result<Chunk1_8> {
    let count = primary_bit_mask.count_ones() as usize;
    let section_len = network_section_len(sky_light);
    let sections_len = count * section_len;
    let biomes_len = if full_chunk { BIOME_COUNT } else { 0 };
    let expected = sections_len + biomes_len;
    if data.len() != expected {
        return Err(invalid(format!(
            "network chunk has {} bytes, expected {expected} for {count} sections",
            data.len()
        )));
    }
    let states_len = count * SECTION_VOLUME * 2;
    let block_light_start = states_len;
    let sky_light_start = states_len + count * NIBBLE_VOLUME;
    let mut sections = Vec::with_capacity(count);
    let mut slot = 0usize;
    for y in 0..16u8 {
        if primary_bit_mask & (1 << y) == 0 {
            continue;
        }
        let mut section = Section1_8::new(y);
        let states = &data[slot * SECTION_VOLUME * 2..(slot + 1) * SECTION_VOLUME * 2];
        for i in 0..SECTION_VOLUME {
            let state = u16::from_le_bytes([states[i * 2], states[i * 2 + 1]]);
            let block_x = i & 15;
            let block_y = i >> 8;
            let block_z = (i >> 4) & 15;
            section.set_block(block_x, block_y, block_z, state >> 4, (state & 0x0F) as u8);
        }
        let light = block_light_start + slot * NIBBLE_VOLUME;
        section.block_light = Some(data[light..light + NIBBLE_VOLUME].to_vec());
        section.sky_light = if sky_light {
            let light = sky_light_start + slot * NIBBLE_VOLUME;
            Some(data[light..light + NIBBLE_VOLUME].to_vec())
        } else {
            None
        };
        sections.push(section);
        slot += 1;
    }
    Ok(Chunk1_8 {
        x,
        z,
        last_update: 0,
        terrain_populated: full_chunk,
        sections,
        biomes: full_chunk.then(|| data[sections_len..].to_vec()),
        height_map: None,
        tile_entities: Vec::new(),
        entities: Vec::new(),
        tile_ticks: Vec::new(),
        extra: Compound::new(),
    })
}

/// Decode a 1.8 `Map Chunk Bulk` payload.
///
/// `ChunkMapS2CPacket` builds every column as a full chunk and writes those
/// byte arrays back to back: each column's sections are followed by its own
/// 256-byte biome array (`ChunkMapS2CPacket.createExtraData` appends
/// `getBiomeArray` to the column it belongs to). `chunks` is
/// `(x, z, primary_bit_mask)` in the packet's order.
pub fn from_bulk(
    data: &[u8],
    chunks: &[(i32, i32, u16)],
    sky_light: bool,
) -> io::Result<Vec<Chunk1_8>> {
    let section_len = network_section_len(sky_light);
    let mut columns = Vec::with_capacity(chunks.len());
    let mut offset = 0usize;
    for &(x, z, mask) in chunks {
        let count = mask.count_ones() as usize;
        let end = offset + count * section_len + BIOME_COUNT;
        if end > data.len() {
            return Err(invalid("bulk chunk data ends before its column"));
        }
        columns.push(from_network(
            x,
            z,
            mask,
            sky_light,
            true,
            &data[offset..end],
        )?);
        offset = end;
    }
    if offset != data.len() {
        return Err(invalid(format!(
            "bulk chunk data has {} bytes, expected {offset}",
            data.len()
        )));
    }
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt::{self, Limits};
    use horizon_core::position::ChunkPos;

    fn bytes(tag: NamedTag) -> Vec<u8> {
        nbt::write_named(&tag, Limits::default()).unwrap()
    }

    fn read(bytes: &[u8]) -> NamedTag {
        let (root, used) = nbt::read_named(bytes, Limits::default()).unwrap();
        assert_eq!(used, bytes.len());
        root
    }

    #[test]
    fn block_ids_use_blocks_and_the_optional_add_nibble() {
        let mut section = Section1_8::new(0);
        section.set_block(1, 2, 3, 0x0AB, 5);
        assert_eq!(section.block_id(1, 2, 3), 0x0AB);
        assert_eq!(section.metadata(1, 2, 3), 5);
        assert_eq!(section.block_state(1, 2, 3), 0x0AB5);
        section.set_block(1, 2, 3, 0x1AB, 6);
        assert_eq!(section.block_id(1, 2, 3), 0x1AB);
        assert_eq!(section.block_state(1, 2, 3), 0x1AB6);
        assert_eq!(section.block_id(0, 0, 0), 0);
    }

    #[test]
    fn light_nibbles_keep_even_low_and_odd_high() {
        let mut section = Section1_8::new(0);
        section.set_block_light(0, 0, 0, 15);
        section.set_block_light(1, 0, 0, 3);
        section.set_sky_light(0, 0, 0, 12);
        section.set_sky_light(15, 15, 15, 9);
        assert_eq!(section.block_light(0, 0, 0), 15);
        assert_eq!(section.block_light(1, 0, 0), 3);
        assert_eq!(section.sky_light(0, 0, 0), 12);
        assert_eq!(section.sky_light(15, 15, 15), 9);
        assert_eq!(section.block_light(2, 0, 0), 0);
    }

    #[test]
    fn section_index_is_y_then_z_then_x() {
        let mut section = Section1_8::new(0);
        section.set_block(15, 15, 15, 7, 0);
        assert_eq!(section.blocks[4095], 7);
        assert_eq!(section.block_id(15, 15, 15), 7);
        section.set_block(0, 1, 0, 9, 0);
        assert_eq!(section.blocks[256], 9);
    }

    #[test]
    fn chunk_nbt_round_trips() {
        let mut section = Section1_8::new(0);
        section.set_block(1, 2, 3, 0x1AB, 5);
        section.set_block_light(1, 2, 3, 12);
        section.set_sky_light(1, 2, 3, 7);
        let mut extra = Compound::new();
        extra.insert(key("InhabitedTime"), Tag::Long(42));
        let chunk = Chunk1_8 {
            x: 4,
            z: -7,
            last_update: 99,
            terrain_populated: true,
            sections: vec![section],
            biomes: Some(vec![1; BIOME_COUNT]),
            height_map: Some(vec![64; HEIGHTMAP_COUNT]),
            tile_entities: vec![Tag::Compound(Compound::new())],
            entities: Vec::new(),
            tile_ticks: Vec::new(),
            extra,
        };
        let root = chunk.to_nbt();
        let back = Chunk1_8::from_nbt(&read(&bytes(root))).unwrap();
        assert_eq!(back, chunk);
        assert_eq!(back.sections[0].block_state(1, 2, 3), 0x1AB5);
    }

    #[test]
    fn absent_sky_light_stays_absent() {
        let mut map = Compound::new();
        map.insert(key("Y"), Tag::Byte(0));
        map.insert(key("Blocks"), Tag::ByteArray(vec![0; SECTION_VOLUME]));
        map.insert(key("Data"), Tag::ByteArray(vec![0; NIBBLE_VOLUME]));
        map.insert(key("BlockLight"), Tag::ByteArray(vec![0; NIBBLE_VOLUME]));
        let section = Section1_8::from_nbt(&Tag::Compound(map)).unwrap();
        assert_eq!(section.sky_light(0, 0, 0), 0);
        let Tag::Compound(back) = section.to_nbt() else {
            panic!("section is not a compound");
        };
        assert!(!back.contains_key(&key("SkyLight")));
        assert!(back.contains_key(&key("BlockLight")));
    }

    #[test]
    fn rejects_bad_shapes() {
        let mut map = Compound::new();
        map.insert(key("Y"), Tag::Byte(0));
        map.insert(key("Blocks"), Tag::ByteArray(vec![0; SECTION_VOLUME - 1]));
        map.insert(key("Data"), Tag::ByteArray(vec![0; NIBBLE_VOLUME]));
        assert!(Section1_8::from_nbt(&Tag::Compound(map)).is_err());

        let mut level = Compound::new();
        level.insert(key("Sections"), Tag::List(Vec::new()));
        let root = NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(Compound::from([(key("Level"), Tag::Compound(level))])),
        };
        assert!(
            Chunk1_8::from_nbt(&root).is_err(),
            "missing xPos is rejected"
        );
        let root = NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(Compound::new()),
        };
        assert!(Chunk1_8::from_nbt(&root).is_err());

        let mut first = Compound::new();
        first.insert(key("Y"), Tag::Byte(0));
        first.insert(key("Blocks"), Tag::ByteArray(vec![0; SECTION_VOLUME]));
        first.insert(key("Data"), Tag::ByteArray(vec![0; NIBBLE_VOLUME]));
        let mut second = first.clone();
        second.insert(key("Y"), Tag::Byte(0));
        let mut level = Compound::new();
        level.insert(
            key("Sections"),
            Tag::List(vec![Tag::Compound(first), Tag::Compound(second)]),
        );
        let root = NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(Compound::from([(key("Level"), Tag::Compound(level))])),
        };
        assert!(Chunk1_8::from_nbt(&root).is_err());
    }

    /// Build the section part of a network column: all state arrays, then all
    /// block-light arrays, then all sky-light arrays (the 1.8 order).
    fn network_column(
        sections: &[(Vec<u16>, Vec<u8>, Option<Vec<u8>>)],
        biomes: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for (states, _, _) in sections {
            assert_eq!(states.len(), SECTION_VOLUME);
            for state in states {
                data.extend_from_slice(&state.to_le_bytes());
            }
        }
        for (_, block_light, _) in sections {
            assert_eq!(block_light.len(), NIBBLE_VOLUME);
            data.extend_from_slice(block_light);
        }
        for (_, _, sky_light) in sections {
            if let Some(sky_light) = sky_light {
                assert_eq!(sky_light.len(), NIBBLE_VOLUME);
                data.extend_from_slice(sky_light);
            }
        }
        if let Some(biomes) = biomes {
            assert_eq!(biomes.len(), BIOME_COUNT);
            data.extend_from_slice(biomes);
        }
        data
    }

    #[test]
    fn network_chunk_is_packed_u16_states() {
        let mask = 1 << 3;
        let mut states = vec![0u16; SECTION_VOLUME];
        // Index 561 is x=1, y=2, z=3: y << 8 | z << 4 | x.
        states[561] = 17 << 4 | 4;
        states[0] = 300 << 4 | 5;
        let mut block_light = vec![0u8; NIBBLE_VOLUME];
        block_light[561 / 2] = 0xF0;
        let mut sky_light = vec![0u8; NIBBLE_VOLUME];
        sky_light[0] = 0x0A;
        let biomes: Vec<u8> = (0..BIOME_COUNT).map(|i| i as u8).collect();
        let data = network_column(&[(states, block_light, Some(sky_light))], Some(&biomes));

        let chunk = from_network(1, -2, mask, true, true, &data).unwrap();
        assert_eq!((chunk.x, chunk.z), (1, -2));
        assert_eq!(chunk.sections.len(), 1);
        assert!(chunk.height_map.is_none());
        let section = chunk.section(3).unwrap();
        assert_eq!(section.block_id(1, 2, 3), 17);
        assert_eq!(section.metadata(1, 2, 3), 4);
        assert_eq!(section.block_state(1, 2, 3), 276);
        // A 12-bit id uses the optional Add array.
        assert_eq!(section.block_id(0, 0, 0), 300);
        assert_eq!(section.metadata(0, 0, 0), 5);
        assert_eq!(section.block_light(1, 2, 3), 15);
        assert_eq!(section.sky_light(0, 0, 0), 10);
        assert_eq!(chunk.biomes.as_deref(), Some(biomes.as_slice()));
    }

    #[test]
    fn network_sections_are_columnar_not_interleaved() {
        let mut first = vec![0u16; SECTION_VOLUME];
        first.fill(1 << 4 | 2);
        let mut third = vec![0u16; SECTION_VOLUME];
        third.fill(35 << 4 | 14);
        let data = network_column(
            &[
                (
                    first,
                    vec![0x11; NIBBLE_VOLUME],
                    Some(vec![0x33; NIBBLE_VOLUME]),
                ),
                (
                    third,
                    vec![0x22; NIBBLE_VOLUME],
                    Some(vec![0x44; NIBBLE_VOLUME]),
                ),
            ],
            Some(&[7; BIOME_COUNT]),
        );
        let chunk = from_network(0, 0, 1 | 1 << 2, true, true, &data).unwrap();
        let first = chunk.section(0).unwrap();
        let third = chunk.section(2).unwrap();
        assert_eq!((first.block_id(0, 0, 0), first.metadata(0, 0, 0)), (1, 2));
        assert_eq!((third.block_id(0, 0, 0), third.metadata(0, 0, 0)), (35, 14));
        assert_eq!(first.block_light(0, 0, 0), 1);
        assert_eq!(third.block_light(0, 0, 0), 2);
        assert_eq!(first.sky_light(0, 0, 0), 3);
        assert_eq!(third.sky_light(0, 0, 0), 4);
    }

    #[test]
    fn network_nether_chunk_has_no_sky_light() {
        let data = network_column(
            &[(vec![0; SECTION_VOLUME], vec![0; NIBBLE_VOLUME], None)],
            Some(&[0; BIOME_COUNT]),
        );
        assert_eq!(data.len(), network_section_len(false) + BIOME_COUNT);
        let chunk = from_network(0, 0, 1, false, true, &data).unwrap();
        let section = chunk.section(0).unwrap();
        assert!(section.sky_light.is_none());
        let Tag::Compound(map) = section.to_nbt() else {
            panic!("section is not a compound");
        };
        assert!(!map.contains_key(&key("SkyLight")));
    }

    #[test]
    fn network_non_full_chunk_has_no_biomes() {
        let data = network_column(
            &[(
                vec![0; SECTION_VOLUME],
                vec![0; NIBBLE_VOLUME],
                Some(vec![0; NIBBLE_VOLUME]),
            )],
            None,
        );
        assert_eq!(data.len(), network_section_len(true));
        let chunk = from_network(0, 0, 1, true, false, &data).unwrap();
        assert!(chunk.biomes.is_none());
        assert!(!chunk.terrain_populated);
    }

    #[test]
    fn bulk_chunks_carry_their_own_biomes() {
        let one = network_column(
            &[(
                vec![0; SECTION_VOLUME],
                vec![0; NIBBLE_VOLUME],
                Some(vec![0; NIBBLE_VOLUME]),
            )],
            Some(&[3; BIOME_COUNT]),
        );
        let two = network_column(
            &[
                (
                    vec![0; SECTION_VOLUME],
                    vec![0; NIBBLE_VOLUME],
                    Some(vec![0; NIBBLE_VOLUME]),
                ),
                (
                    vec![0; SECTION_VOLUME],
                    vec![0; NIBBLE_VOLUME],
                    Some(vec![0; NIBBLE_VOLUME]),
                ),
            ],
            Some(&[9; BIOME_COUNT]),
        );
        // ChunkMapS2CPacket.write: each column's bytes, biome array included.
        let mut data = Vec::new();
        data.extend_from_slice(&one);
        data.extend_from_slice(&two);
        let chunks = from_bulk(&data, &[(1, 2, 1), (3, 4, 1 | 1 << 2)], true).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!((chunks[0].x, chunks[0].z), (1, 2));
        assert_eq!((chunks[1].x, chunks[1].z), (3, 4));
        assert_eq!(chunks[0].sections.len(), 1);
        assert_eq!(chunks[1].sections.len(), 2);
        assert_eq!(chunks[0].biomes.as_deref(), Some(&[3; BIOME_COUNT][..]));
        assert_eq!(chunks[1].biomes.as_deref(), Some(&[9; BIOME_COUNT][..]));
        assert!(chunks.iter().all(|chunk| chunk.terrain_populated));
    }

    #[test]
    fn network_chunk_rejects_wrong_lengths() {
        let exact = network_column(
            &[(
                vec![0; SECTION_VOLUME],
                vec![0; NIBBLE_VOLUME],
                Some(vec![0; NIBBLE_VOLUME]),
            )],
            Some(&[0; BIOME_COUNT]),
        );
        assert!(from_network(0, 0, 1, true, true, &exact[..exact.len() - 1]).is_err());
        let mut too_long = exact.clone();
        too_long.push(0);
        assert!(from_network(0, 0, 1, true, true, &too_long).is_err());
        assert!(from_bulk(&exact, &[(0, 0, 1), (0, 1, 1)], true).is_err());
    }

    #[test]
    fn saved_1_8_chunk_upgrades_to_the_current_data_version() {
        let mut chunk = Chunk1_8::new(0, 0);
        let mut section = Section1_8::new(0);
        section.set_block(0, 0, 0, 1, 0); // stone
        section.set_block(1, 0, 0, 3, 0); // dirt
        section.set_block(2, 0, 0, 2, 0); // grass
        chunk.sections.push(section);
        let named = chunk.to_current_nbt("minecraft:overworld", Some("minecraft:noise"));
        let tag = named.tag.clone();
        assert_eq!(tag.get("DataVersion"), Some(&Tag::Int(crate::DATA_VERSION)));
        let data = crate::chunk::ChunkData::new(named).expect("upgraded chunk loads as current");
        assert_eq!(data.position(), ChunkPos::new(0, 0));
        let Tag::List(sections) = tag.get("sections").expect("sections") else {
            panic!("sections is not a list");
        };
        let state = sections
            .iter()
            .find(|section| section.get("Y") == Some(&Tag::Byte(0)))
            .expect("section 0");
        let Tag::List(palette) = state
            .get("block_states")
            .and_then(|states| states.get("palette"))
            .expect("palette")
        else {
            panic!("palette is not a list");
        };
        let names: Vec<_> = palette
            .iter()
            .map(|entry| crate::datafix::dynamic::as_str(entry.get("id")))
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.as_deref() == Some("minecraft:stone"))
        );
        assert!(
            names
                .iter()
                .any(|name| name.as_deref() == Some("minecraft:grass_block"))
        );
    }
}
