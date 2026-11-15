//! Wire types that differ between 1.8.9 and 26.3: the packed block position,
//! the numeric item stack and the multi-block-change record array.
use crate::{
    Result,
    buf::{Codec, Reader, Writer},
};
use horizon_world::nbt::Tag;

/// 1.8's `Position`: x (26 bits) in the high bits, then y (12), then z (26).
/// 26.3 uses `x | z | y` (`crate::buf::Pos`), so the two are not interchangeable.
pub struct Pos1_8;
impl Codec<[i32; 3]> for Pos1_8 {
    fn read(r: &mut Reader) -> Result<[i32; 3]> {
        Ok(unpack(r.i64()?))
    }
    fn write(v: &[i32; 3], w: &mut Writer) {
        w.i64(pack(*v));
    }
}
pub fn pack(p: [i32; 3]) -> i64 {
    ((i64::from(p[0]) & 0x3FF_FFFF) << 38)
        | ((i64::from(p[1]) & 0xFFF) << 26)
        | (i64::from(p[2]) & 0x3FF_FFFF)
}
pub fn unpack(v: i64) -> [i32; 3] {
    [
        (v >> 38) as i32,
        (v << 26 >> 52) as i32,
        (v << 38 >> 38) as i32,
    ]
}

/// A 1.8 item stack: numeric id, count, damage and optional NBT. Id `-1` is
/// the empty stack and carries no further fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub id: i16,
    pub count: i8,
    pub damage: i16,
    pub nbt: Tag,
}
impl Slot {
    pub fn empty() -> Self {
        Self {
            id: -1,
            count: 0,
            damage: 0,
            nbt: Tag::End,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.id == -1
    }
    pub fn new(id: i16, count: i8, damage: i16) -> Self {
        Self {
            id,
            count,
            damage,
            nbt: Tag::End,
        }
    }
}
impl Codec<Slot> for Slot {
    fn read(r: &mut Reader) -> Result<Slot> {
        let id = r.i16()?;
        if id == -1 {
            return Ok(Slot::empty());
        }
        Ok(Slot {
            id,
            count: r.i8()?,
            damage: r.i16()?,
            nbt: r.nbt_classic()?,
        })
    }
    fn write(v: &Slot, w: &mut Writer) {
        w.i16(v.id);
        if v.id == -1 {
            return;
        }
        w.i8(v.count);
        w.i16(v.damage);
        w.nbt_classic(&v.nbt);
    }
}

/// A `Window Items` slot array: an `i16` count, then that many slots.
pub struct Slots;
impl Codec<Vec<Slot>> for Slots {
    fn read(r: &mut Reader) -> Result<Vec<Slot>> {
        let n = r.i16()?;
        if n < 0 || n as usize > r.remaining() / 2 {
            return Err(crate::Error::new("invalid window slot count"));
        }
        (0..n).map(|_| <Slot as Codec<Slot>>::read(r)).collect()
    }
    fn write(v: &Vec<Slot>, w: &mut Writer) {
        w.i16(v.len() as i16);
        for slot in v {
            <Slot as Codec<Slot>>::write(slot, w);
        }
    }
}

/// One `MultiBlockChange` record: the horizontal position packs x in the high
/// nibble and z in the low nibble, both chunk-relative.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockRecord {
    pub x: u8,
    pub z: u8,
    pub y: u8,
    /// The 1.8 global block state: `id << 4 | metadata`.
    pub state: i32,
}
impl BlockRecord {
    pub fn horizontal(&self) -> u8 {
        (self.x & 0x0F) << 4 | (self.z & 0x0F)
    }
    pub fn from_horizontal(h: u8, y: u8, state: i32) -> Self {
        Self {
            x: h >> 4,
            z: h & 0x0F,
            y,
            state,
        }
    }
}
/// VarInt-counted `BlockRecord` array.
pub struct BlockRecords;
impl Codec<Vec<BlockRecord>> for BlockRecords {
    fn read(r: &mut Reader) -> Result<Vec<BlockRecord>> {
        let n = r.varint()?;
        if n < 0 {
            return Err(crate::Error::new("negative multi-block-change count"));
        }
        (0..n)
            .map(|_| {
                let h = r.u8()?;
                let y = r.u8()?;
                Ok(BlockRecord::from_horizontal(h, y, r.varint()?))
            })
            .collect()
    }
    fn write(v: &Vec<BlockRecord>, w: &mut Writer) {
        w.varint(v.len() as i32);
        for record in v {
            w.u8(record.horizontal());
            w.u8(record.y);
            w.varint(record.state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_position_puts_y_above_z() {
        // x = 1, y = 2, z = 3: 1 << 38 | 2 << 26 | 3.
        assert_eq!(pack([1, 2, 3]), (1 << 38) | (2 << 26) | 3);
        assert_eq!(unpack((1 << 38) | (2 << 26) | 3), [1, 2, 3]);
        for p in [
            [0, 0, 0],
            [-1, 0, 0],
            [0, -1, 0],
            [0, 0, -1],
            [33_554_431, 2047, -33_554_432],
        ] {
            assert_eq!(unpack(pack(p)), p);
        }
    }

    #[test]
    fn empty_slot_is_only_the_id() {
        let mut w = Writer::new();
        <Slot as Codec<Slot>>::write(&Slot::empty(), &mut w);
        assert_eq!(w.data, [0xFF, 0xFF]);
        let mut r = Reader::new(&w.data);
        assert_eq!(<Slot as Codec<Slot>>::read(&mut r).unwrap(), Slot::empty());
    }

    #[test]
    fn slot_layout_and_optional_nbt() {
        let slot = Slot {
            id: 276,
            count: 1,
            damage: 0,
            nbt: Tag::End,
        };
        let mut w = Writer::new();
        <Slot as Codec<Slot>>::write(&slot, &mut w);
        assert_eq!(w.data, [0x01, 0x14, 0x01, 0x00, 0x00, 0x00]);
        let mut r = Reader::new(&w.data);
        assert_eq!(<Slot as Codec<Slot>>::read(&mut r).unwrap(), slot);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn slot_nbt_is_written_with_a_root_name() {
        // 1.8 writes the file form (`CompressedStreamTools.write`): the
        // compound type, an empty root name, the body, then TAG_End.
        let slot = Slot {
            id: 276,
            count: 1,
            damage: 0,
            nbt: Tag::compound([("Damage", Tag::Int(3))]),
        };
        let mut w = Writer::new();
        <Slot as Codec<Slot>>::write(&slot, &mut w);
        assert_eq!(
            w.data,
            [
                0x01, 0x14, 0x01, 0x00, 0x00, // id, count, damage
                0x0A, 0x00, 0x00, // TAG_Compound, empty root name
                0x03, 0x00, 0x06, b'D', b'a', b'm', b'a', b'g', b'e', 0, 0, 0, 3,    //
                0x00, // TAG_End
            ]
        );
        let mut r = Reader::new(&w.data);
        assert_eq!(<Slot as Codec<Slot>>::read(&mut r).unwrap(), slot);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn a_zero_byte_means_no_nbt() {
        // An absent tag is a single `0`, not a compound with an empty body.
        let mut r = Reader::new(&[0x01, 0x14, 0x01, 0x00, 0x00, 0x00]);
        assert_eq!(
            <Slot as Codec<Slot>>::read(&mut r).unwrap(),
            Slot::new(276, 1, 0)
        );
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn multi_block_records_pack_x_high_z_low() {
        let records = vec![
            BlockRecord {
                x: 5,
                z: 9,
                y: 64,
                state: 1 << 4 | 3,
            },
            BlockRecord {
                x: 15,
                z: 0,
                y: 0,
                state: 0,
            },
        ];
        assert_eq!(records[0].horizontal(), 0x59);
        let mut w = Writer::new();
        <BlockRecords as Codec<Vec<BlockRecord>>>::write(&records, &mut w);
        assert_eq!(w.data, [0x02, 0x59, 64, 0x13, 0xF0, 0, 0]);
        let mut r = Reader::new(&w.data);
        assert_eq!(
            <BlockRecords as Codec<Vec<BlockRecord>>>::read(&mut r).unwrap(),
            records
        );
    }
}
