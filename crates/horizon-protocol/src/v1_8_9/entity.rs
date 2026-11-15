//! 1.8.9 entity metadata and small entity wire types.
//!
//! 1.8 metadata is a terminated list of `(type, index, value)` triples: one
//! header byte (`type << 5 | index`) per entry, then the value, ending with
//! `0x7F`. Later versions changed both the type ids and the position encoding.
use crate::{
    Result,
    buf::{Codec, Reader, Writer},
    v1_8_9::codec::Slot,
};
use std::collections::BTreeMap;

/// One metadata value. The index is the map key.
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Slot(Slot),
    /// Three plain ints in 1.8, not a packed position.
    Position([i32; 3]),
    /// Pitch, yaw, roll in degrees.
    Rotation([f32; 3]),
}
/// Entity index to metadata value.
pub type Metadata = BTreeMap<u8, MetadataValue>;

/// The 1.8 metadata list, terminated by `0x7F`.
pub struct EntityMetadata;
impl Codec<Metadata> for EntityMetadata {
    fn read(r: &mut Reader) -> Result<Metadata> {
        let mut metadata = Metadata::new();
        loop {
            let header = r.u8()?;
            if header == 0x7F {
                return Ok(metadata);
            }
            let kind = header >> 5;
            let index = header & 0x1F;
            let value = match kind {
                0 => MetadataValue::Byte(r.i8()?),
                1 => MetadataValue::Short(r.i16()?),
                2 => MetadataValue::Int(r.i32()?),
                3 => MetadataValue::Float(r.f32()?),
                4 => MetadataValue::String(r.string(32767)?),
                5 => MetadataValue::Slot(<Slot as Codec<Slot>>::read(r)?),
                6 => MetadataValue::Position([r.i32()?, r.i32()?, r.i32()?]),
                7 => MetadataValue::Rotation([r.f32()?, r.f32()?, r.f32()?]),
                _ => unreachable!("three bits of type"),
            };
            metadata.insert(index, value);
        }
    }
    fn write(v: &Metadata, w: &mut Writer) {
        for (&index, value) in v {
            let kind = match value {
                MetadataValue::Byte(_) => 0,
                MetadataValue::Short(_) => 1,
                MetadataValue::Int(_) => 2,
                MetadataValue::Float(_) => 3,
                MetadataValue::String(_) => 4,
                MetadataValue::Slot(_) => 5,
                MetadataValue::Position(_) => 6,
                MetadataValue::Rotation(_) => 7,
            };
            w.u8(kind << 5 | (index & 0x1F));
            match value {
                MetadataValue::Byte(v) => w.i8(*v),
                MetadataValue::Short(v) => w.i16(*v),
                MetadataValue::Int(v) => w.i32(*v),
                MetadataValue::Float(v) => w.f32(*v),
                MetadataValue::String(v) => w.string(v),
                MetadataValue::Slot(v) => <Slot as Codec<Slot>>::write(v, w),
                MetadataValue::Position(v) => {
                    for c in v {
                        w.i32(*c);
                    }
                }
                MetadataValue::Rotation(v) => {
                    for c in v {
                        w.f32(*c);
                    }
                }
            }
        }
        w.u8(0x7F);
    }
}

/// Three 1/8000-block-per-tick velocities (1.8 `vec3i16`).
pub struct Velocity;
impl Codec<[i16; 3]> for Velocity {
    fn read(r: &mut Reader) -> Result<[i16; 3]> {
        Ok([r.i16()?, r.i16()?, r.i16()?])
    }
    fn write(v: &[i16; 3], w: &mut Writer) {
        for c in v {
            w.i16(*c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(metadata: &Metadata) -> Vec<u8> {
        let mut w = Writer::new();
        <EntityMetadata as Codec<Metadata>>::write(metadata, &mut w);
        w.data
    }
    fn decode(bytes: &[u8]) -> Result<Metadata> {
        let mut r = Reader::new(bytes);
        let value = <EntityMetadata as Codec<Metadata>>::read(&mut r)?;
        assert_eq!(r.remaining(), 0);
        Ok(value)
    }

    #[test]
    fn header_packs_type_and_index() {
        let mut metadata = Metadata::new();
        metadata.insert(0, MetadataValue::Byte(0x7F));
        metadata.insert(1, MetadataValue::Short(300));
        metadata.insert(4, MetadataValue::String("hi".into()));
        metadata.insert(6, MetadataValue::Position([1, 2, 3]));
        assert_eq!(
            encode(&metadata),
            [
                0x00, 0x7F, 0x21, 0x01, 0x2C, 0x84, 0x02, b'h', b'i', 0xC6, 0, 0, 0, 1, 0, 0, 0, 2,
                0, 0, 0, 3, 0x7F
            ]
        );
        assert_eq!(decode(&encode(&metadata)).unwrap(), metadata);
    }

    #[test]
    fn rotation_and_slot_values_round_trip() {
        let mut metadata = Metadata::new();
        metadata.insert(2, MetadataValue::Float(1.5));
        metadata.insert(
            3,
            MetadataValue::Slot(Slot {
                id: 276,
                count: 1,
                damage: 0,
                nbt: horizon_world::nbt::Tag::End,
            }),
        );
        metadata.insert(7, MetadataValue::Rotation([10.0, 20.0, 30.0]));
        assert_eq!(decode(&encode(&metadata)).unwrap(), metadata);
    }

    #[test]
    fn empty_metadata_is_only_the_terminator() {
        assert_eq!(encode(&Metadata::new()), [0x7F]);
        assert!(decode(&[0x7F]).unwrap().is_empty());
    }

    #[test]
    fn truncated_metadata_is_rejected() {
        assert!(decode(&[0x00]).is_err());
        assert!(decode(&[0x84, 0x05, b'h']).is_err());
        assert!(decode(&[]).is_err());
    }

    #[test]
    fn velocity_is_three_shorts() {
        let mut w = Writer::new();
        <Velocity as Codec<[i16; 3]>>::write(&[1, -2, 300], &mut w);
        assert_eq!(w.data, [0x00, 0x01, 0xFF, 0xFE, 0x01, 0x2C]);
        let mut r = Reader::new(&w.data);
        assert_eq!(
            <Velocity as Codec<[i16; 3]>>::read(&mut r).unwrap(),
            [1, -2, 300]
        );
    }
}
