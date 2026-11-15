//! 1.8.9 maps: the palette, its shading, canvas patches and icons.
//!
//! A 1.8 map pixel is one byte, `colour * 4 + shade` (`ItemMap.updateMapData`
//! writes `mapColor.colorIndex * 4 + shade`). The upper six bits index
//! `MapColor.mapColorArray`; 1.8.9 defines only ids 0..=35 there (0 is
//! `airColor`, drawn as the transparent checkerboard) and the low two bits are
//! the height/water shade 0..=3. `MapColor.getMapColor` scales each channel by
//! `[180, 220, 255, 135][shade] / 255` in integer arithmetic and returns opaque
//! ARGB; [`color`] returns the same result as `0xRRGGBB` (`0` for the
//! transparent colour 0), which is what the current map renderer takes.
//!
//! Later versions appended colours to `MapColor` (26.3 has 62 ids), but ids
//! 0..=35 kept their 1.8.9 values; [`PALETTE`] pins the 1.8.9 table here.
//!
//! `MapIcon.direction_and_type` packs the icon type in the high nibble and the
//! rotation in the low one (`S34PacketMaps.readPacketData`: `(byte >> 4) & 15`
//! and `byte & 15`). 1.8.9's `assets/minecraft/textures/map/map_icons.png` is a
//! 32x32 atlas of 8x8 cells: types 0..=6 have art and 7..=15 sample the opaque
//! `0x0094FF` filler. The mansion, monument and banner icons are 1.9/1.11
//! additions that post-date this format.
//!
//! Sources: MCP 1.8.9 `MapColor`, `ItemMap`, `MapItemRenderer`, `MapData`,
//! `S34PacketMaps` and `Vec4b`, checked against the official 1.8.9 client jar
//! (`3870888a…`, texture `map_icons.png`).
use horizon_protocol::v1_8_9::play_extra::{MapIcon, MapPatch};

/// The map canvas is 128x128 pixels at every 1.8.9 scale.
pub const CANVAS_SIZE: u8 = 128;

/// `MapColor.mapColorArray` ids 0..=35 as `0xRRGGBB`.
///
/// Id 0 is `airColor`/transparent; the rest are `MapColor.getMapColor`'s
/// values at shade 2 (the unshaded ones). 1.8.9 stops at `netherrackColor`
/// (35); ids 36..=63 exist in the array but were never constructed.
pub const PALETTE: [u32; 36] = [
    0x000000, 0x7FB238, 0xF7E9A3, 0xC7C7C7, 0xFF0000, 0xA0A0FF, 0xA7A7A7, 0x007C00, 0xFFFFFF,
    0xA4A8B8, 0x976D4D, 0x707070, 0x4040FF, 0x8F7748, 0xFFFCF5, 0xD87F33, 0xB24CD8, 0x6699D8,
    0xE5E533, 0x7FCC19, 0xF27FA5, 0x4C4C4C, 0x999999, 0x4C7F99, 0x7F3FB2, 0x334CB2, 0x664C33,
    0x667F33, 0x993333, 0x191919, 0xFAEE4D, 0x5CDBD5, 0x4A80FF, 0x00D93A, 0x815631, 0x700200,
];

/// `MapColor.getMapColor`'s shade multipliers, indexed by a pixel's low bits:
/// 180/255, 220/255, 255/255 and 135/255.
pub const SHADE_MULTIPLIERS: [u8; 4] = [180, 220, 255, 135];

/// The unshaded `0xRRGGBB` value of a 1.8.9 `MapColor` id (0..=35).
///
/// `0` is `airColor`/transparent and ids 36..=63 do not exist in 1.8.9; both
/// return `0`.
pub fn base_color(color_index: u8) -> u32 {
    PALETTE.get(usize::from(color_index)).copied().unwrap_or(0)
}

/// Scale an `0xRRGGBB` colour by a 1.8.9 shade, as `MapColor.getMapColor`
/// does. `shade` is masked to its two bits.
pub fn shade_rgb(rgb: u32, shade: u8) -> u32 {
    let factor = u32::from(SHADE_MULTIPLIERS[usize::from(shade & 3)]);
    let channel = |shift: u32| ((rgb >> shift) & 0xFF) * factor / 255;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// The colour 1.8.9 draws for one raw map pixel (`MapData.colors` byte).
///
/// The high six bits are the `MapColor` id and the low two are the shade;
/// `0` is returned for the transparent id 0 and for ids 1.8.9 does not define
/// (vanilla indexes a null entry there).
pub fn color(argb_index: u8) -> u32 {
    match PALETTE.get(usize::from(argb_index >> 2)) {
        Some(&rgb) => shade_rgb(rgb, argb_index & 3),
        None => 0,
    }
}

/// One rectangular patch from `PacketPlayOutMap` (`S34PacketMaps`).
///
/// The protocol stores the packet's unsigned width/height/x/y bytes as `i8`s;
/// this widens them back, so a width of 200 arrives here as 200 and is clipped
/// by [`patch_colors`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    /// Row-major pixels, `width * height` bytes when the packet is well formed.
    pub colors: Vec<u8>,
}

impl Patch {
    /// Convert the protocol patch, restoring the wire's unsigned fields.
    pub fn from_protocol(patch: &MapPatch) -> Self {
        Self {
            start_x: patch.x as u8,
            start_y: patch.y as u8,
            width: patch.columns as u8,
            height: patch.rows as u8,
            colors: patch.data.clone(),
        }
    }
}

impl From<&MapPatch> for Patch {
    fn from(patch: &MapPatch) -> Self {
        Self::from_protocol(patch)
    }
}

/// Yield `(x, y, colour)` for every pixel a patch contributes to the 128x128
/// canvas, in row-major order.
///
/// Pixels outside the canvas (a patch may start at or overlap an edge) and
/// bytes missing from a truncated `colors` are skipped; extra bytes are
/// ignored.
pub fn patch_colors(patch: &Patch) -> impl Iterator<Item = (u8, u8, u32)> + '_ {
    let width = usize::from(patch.width);
    let height = usize::from(patch.height);
    let start_x = u16::from(patch.start_x);
    let start_y = u16::from(patch.start_y);
    let canvas = u16::from(CANVAS_SIZE);
    let colors = &patch.colors;
    (0..height).flat_map(move |row| {
        (0..width).filter_map(move |column| {
            let pixel = *colors.get(row * width + column)?;
            let x = start_x + column as u16;
            let y = start_y + row as u16;
            if x < canvas && y < canvas {
                Some((x as u8, y as u8, color(pixel)))
            } else {
                None
            }
        })
    })
}

/// A map decoration type 1.8.9 can draw.
///
/// The names describe 1.8.9's sprites rather than later decoration enums:
/// `MapData.updateDecorations` creates types 0 (player), 1 (item frame) and 6
/// (player off the map); types 2..=5 come from a map's `Decorations` NBT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconKind {
    /// Type 0: the player marker, a white arrow.
    WhiteArrow,
    /// Type 1: an item frame, a green arrow.
    GreenArrow,
    /// Type 2: a red marker.
    RedArrow,
    /// Type 3: a blue marker.
    BlueArrow,
    /// Type 4: a target X.
    WhiteCross,
    /// Type 5: a target point, a red pointer.
    RedPointer,
    /// Type 6: a player outside the mapped area, a white circle.
    WhiteCircle,
    /// Types 7..=15: slots whose art 1.8.9 never shipped. The client samples
    /// the atlas's opaque `0x0094FF` filler there. Later versions gave these
    /// numbers meanings (1.9: `player_off_limits` at 7; 1.11: mansion at 8,
    /// monument at 9, banners at 10..=25), but none of those exist in a 1.8.9
    /// map.
    Unassigned,
    /// A type outside the four-bit field.
    Unknown,
}

/// The sixteen four-bit icon slots in atlas order, as 1.8.9 indexes them.
pub const ICON_KINDS: [IconKind; 16] = [
    IconKind::WhiteArrow,
    IconKind::GreenArrow,
    IconKind::RedArrow,
    IconKind::BlueArrow,
    IconKind::WhiteCross,
    IconKind::RedPointer,
    IconKind::WhiteCircle,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
    IconKind::Unassigned,
];

/// The kind of an already-unpacked four-bit icon type.
pub fn icon_kind(icon_type: u8) -> IconKind {
    ICON_KINDS
        .get(usize::from(icon_type))
        .copied()
        .unwrap_or(IconKind::Unknown)
}

/// A 1.8.9 map decoration, independent of any renderer's marker type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Icon {
    pub kind: IconKind,
    /// `-128..=127`; vanilla draws `x / 2 + 64`.
    pub x: i8,
    /// `-128..=127`; the protocol calls this field `z`.
    pub y: i8,
    /// Rotation, `rotation * 360 / 16` degrees.
    pub rotation: u8,
}

impl Icon {
    /// Split `direction_and_type` into its type and rotation nibbles.
    pub fn from_protocol(icon: &MapIcon) -> Self {
        let packed = icon.direction_and_type as u8;
        Self {
            kind: icon_kind(packed >> 4),
            x: icon.x,
            y: icon.z,
            rotation: packed & 0x0F,
        }
    }
}

impl From<&MapIcon> for Icon {
    fn from(icon: &MapIcon) -> Self {
        Self::from_protocol(icon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocol_patch(x: i8, y: i8, columns: i8, rows: i8, data: Vec<u8>) -> Patch {
        Patch::from_protocol(&MapPatch {
            columns,
            rows,
            x,
            y,
            data,
        })
    }

    #[test]
    fn palette_holds_the_1_8_9_colours() {
        assert_eq!(PALETTE.len(), 36);
        // MapColor ids 1, 2, 8, 11, 12, 14, 29 and 35.
        assert_eq!(base_color(1), 0x7FB238); // grassColor
        assert_eq!(base_color(2), 0xF7E9A3); // sandColor
        assert_eq!(base_color(8), 0xFFFFFF); // snowColor
        assert_eq!(base_color(11), 0x707070); // stoneColor
        assert_eq!(base_color(12), 0x4040FF); // waterColor
        assert_eq!(base_color(14), 0xFFFCF5); // quartzColor
        assert_eq!(base_color(29), 0x191919); // blackColor
        assert_eq!(base_color(35), 0x700200); // netherrackColor
        assert_eq!(base_color(0), 0, "airColor is transparent");
        assert_eq!(base_color(36), 0, "first id 1.8.9 does not define");
        assert_eq!(base_color(u8::MAX), 0);
    }

    #[test]
    fn colour_applies_the_vanilla_shade_multipliers() {
        // MapColor.getMapColor: channel * [180, 220, 255, 135] / 255, with
        // integer division. Expected values are hand-computed from that.
        assert_eq!(color(1 << 2), 0x597D27);
        assert_eq!(color((1 << 2) | 1), 0x6D9930);
        assert_eq!(color((1 << 2) | 2), 0x7FB238);
        assert_eq!(color((1 << 2) | 3), 0x435E1D);

        assert_eq!(color(12 << 2), 0x2D2DB4);
        assert_eq!(color((12 << 2) | 2), 0x4040FF);
        assert_eq!(color((11 << 2) | 3), 0x3B3B3B);

        // Shade 2 is the unshaded palette for every entry.
        for index in 1..PALETTE.len() as u8 {
            assert_eq!(color(index << 2 | 2), base_color(index), "id {index}");
        }
    }

    #[test]
    fn transparent_colour_and_unknown_ids_return_zero() {
        for shade in 0..=3 {
            assert_eq!(color(shade), 0, "colour id 0 is airColor");
        }
        assert_eq!(color(36 << 2), 0);
        assert_eq!(color((63 << 2) | 3), 0);
        assert_eq!(color(u8::MAX), 0);
    }

    #[test]
    fn patch_restores_unsigned_fields_and_clips_to_the_canvas() {
        // The packet's width/height/x/y are unsigned bytes; the protocol
        // decoder stores them as i8, so 200 arrives as -56.
        let patch = Patch::from_protocol(&MapPatch {
            columns: -56,
            rows: 2,
            x: 126,
            y: 127,
            data: vec![6; 200 * 2],
        });
        assert_eq!((patch.width, patch.height), (200, 2));
        assert_eq!((patch.start_x, patch.start_y), (126, 127));
        assert_eq!(
            patch_colors(&patch).collect::<Vec<_>>(),
            [(126, 127, 0x7FB238), (127, 127, 0x7FB238)]
        );
    }

    #[test]
    fn patch_is_row_major_and_tolerates_short_or_long_data() {
        // 2x2 patch with only two of its four pixels on the wire.
        let short = protocol_patch(10, 20, 2, 2, vec![6, 10]);
        assert_eq!(
            patch_colors(&short).collect::<Vec<_>>(),
            [(10, 20, 0x7FB238), (11, 20, 0xF7E9A3)]
        );

        // Extra bytes past width * height are not pixels.
        let long = protocol_patch(10, 20, 1, 1, vec![6, 0, 0]);
        assert_eq!(
            patch_colors(&long).collect::<Vec<_>>(),
            [(10, 20, 0x7FB238)]
        );
    }

    #[test]
    fn patch_covers_the_whole_canvas_and_drops_offscreen_starts() {
        // 128 arrives as -128 in the protocol's i8 fields.
        let full = protocol_patch(0, 0, -128, -128, vec![6; 128 * 128]);
        assert_eq!((full.width, full.height), (128, 128));
        let pixels: Vec<_> = patch_colors(&full).collect();
        assert_eq!(pixels.len(), 128 * 128);
        assert_eq!(pixels[0], (0, 0, 0x7FB238));
        assert_eq!(pixels[128 * 128 - 1], (127, 127, 0x7FB238));

        // -1 is the wire byte 255: the whole patch starts past the edge.
        let off = protocol_patch(-1, -1, 2, 2, vec![6; 4]);
        assert_eq!(patch_colors(&off).count(), 0);

        let zero = protocol_patch(0, 0, 0, 4, vec![6; 8]);
        assert_eq!(
            patch_colors(&zero).count(),
            0,
            "a zero-column patch has no pixels"
        );
    }

    #[test]
    fn icon_unpacks_the_type_and_rotation_nibbles() {
        let icon = Icon::from_protocol(&MapIcon {
            direction_and_type: 0x63,
            x: -12,
            z: 100,
        });
        assert_eq!(
            icon,
            Icon {
                kind: IconKind::WhiteCircle,
                x: -12,
                y: 100,
                rotation: 3,
            }
        );

        let negative = Icon::from_protocol(&MapIcon {
            direction_and_type: -1,
            x: 0,
            z: 0,
        });
        assert_eq!(negative.kind, IconKind::Unassigned, "type 15 has no art");
        assert_eq!(negative.rotation, 15, "rotation is the low nibble");
    }

    #[test]
    fn icon_kind_covers_every_four_bit_type() {
        assert_eq!(ICON_KINDS.len(), 16);
        let expected = [
            IconKind::WhiteArrow,
            IconKind::GreenArrow,
            IconKind::RedArrow,
            IconKind::BlueArrow,
            IconKind::WhiteCross,
            IconKind::RedPointer,
            IconKind::WhiteCircle,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
            IconKind::Unassigned,
        ];
        for (value, kind) in expected.into_iter().enumerate() {
            assert_eq!(icon_kind(value as u8), kind, "type {value}");
        }
        assert_eq!(icon_kind(16), IconKind::Unknown);
        assert_eq!(icon_kind(u8::MAX), IconKind::Unknown);
    }
}
