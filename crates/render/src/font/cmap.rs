//! The `cmap` table: which glyph draws each character. Two formats of subtable are read, which between them cover
//! the fonts of every system: format 4, segments of the Basic Multilingual Plane, and format 12, groups of the whole
//! of Unicode.

use super::reader::Reader;
use crate::error::Error;

/// The subtable a font maps characters with, and where it is in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharacterMap {
    /// Format 4, segments of 16-bit characters, at an offset in the file.
    Segments(usize),
    /// Format 12, groups of 32-bit characters, at an offset in the file.
    Groups(usize),
}

/// The encodings of Unicode a subtable may have, as pairs of a platform and an encoding, best first: the whole of
/// Unicode before its first plane only.
const WHOLE_UNICODE: [(u16, u16); 2] = [(3, 10), (0, 4)];
/// The encodings of the first plane of Unicode.
const FIRST_PLANE: [(u16, u16); 4] = [(3, 1), (0, 3), (0, 2), (0, 1)];

impl CharacterMap {
    /// Chooses the subtable of a `cmap` table that starts at an offset in the file: one of format 12 for the whole of
    /// Unicode when the font has one, else one of format 4.
    pub(crate) fn read(cmap: Reader<'_>, start: usize) -> Result<CharacterMap, Error> {
        let count = cmap.u16(2).ok_or(Error::Damaged { table: "cmap" })?;
        let records: Vec<(u16, u16, usize)> = (0..usize::from(count))
            .filter_map(|record| {
                let at = 4 + record * 8;
                Some((cmap.u16(at)?, cmap.u16(at + 2)?, cmap.offset32(at + 4)?))
            })
            .collect();
        let find = |encodings: &[(u16, u16)], format: u16| {
            encodings.iter().find_map(|(platform, encoding)| {
                records.iter().find_map(|(record_platform, record_encoding, offset)| {
                    let matches = record_platform == platform && record_encoding == encoding;
                    (matches && cmap.u16(*offset) == Some(format)).then_some(start + offset)
                })
            })
        };
        if let Some(offset) = find(&WHOLE_UNICODE, 12).or_else(|| find(&FIRST_PLANE, 12)) {
            return Ok(CharacterMap::Groups(offset));
        }
        find(&FIRST_PLANE, 4)
            .or_else(|| find(&WHOLE_UNICODE, 4))
            .map(CharacterMap::Segments)
            .ok_or(Error::MissingTable { table: "cmap" })
    }

    /// The glyph of a character, or nothing when the subtable has none; glyph 0 may also mean none.
    pub(crate) fn glyph(self, file: Reader<'_>, character: u32) -> Option<u16> {
        match self {
            CharacterMap::Segments(offset) => segment_glyph(file.from(offset)?, character),
            CharacterMap::Groups(offset) => group_glyph(file.from(offset)?, character),
        }
    }
}

/// The glyph of a character in a subtable of format 4: the first segment whose end is at or after the character,
/// when it starts at or before it, gives the glyph by adding its delta to the character, or by reading it from the
/// array of glyphs its range offset points into.
fn segment_glyph(table: Reader<'_>, character: u32) -> Option<u16> {
    let character = u16::try_from(character).ok()?;
    let segment_count = usize::from(table.u16(6)? / 2);
    let ends = 14;
    let starts = ends + segment_count * 2 + 2;
    let deltas = starts + segment_count * 2;
    let range_offsets = deltas + segment_count * 2;
    let (mut low, mut high) = (0, segment_count);
    while low < high {
        let middle = usize::midpoint(low, high);
        if table.u16(ends + middle * 2)? < character {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    let segment = low;
    if segment >= segment_count {
        return None;
    }
    let start = table.u16(starts + segment * 2)?;
    if start > character {
        return None;
    }
    let delta = table.u16(deltas + segment * 2)?;
    let range_offset = table.offset16(range_offsets + segment * 2)?;
    if range_offset == 0 {
        return Some(character.wrapping_add(delta));
    }
    let at = range_offsets + segment * 2 + range_offset + usize::from(character - start) * 2;
    let glyph = table.u16(at)?;
    (glyph != 0).then(|| glyph.wrapping_add(delta))
}

/// The glyph of a character in a subtable of format 12: the group whose range holds the character gives its first
/// glyph, and the glyphs after it follow the characters.
fn group_glyph(table: Reader<'_>, character: u32) -> Option<u16> {
    let group_count = table.offset32(12)?;
    let (mut low, mut high) = (0, group_count);
    while low < high {
        let middle = usize::midpoint(low, high);
        let group = 16 + middle * 12;
        let (first, last) = (table.u32(group)?, table.u32(group + 4)?);
        if character < first {
            high = middle;
        } else if character > last {
            low = middle + 1;
        } else {
            let first_glyph = table.u32(group + 8)?;
            return u16::try_from(first_glyph.checked_add(character - first)?).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A subtable of format 4 with two segments, `A` to `C` by delta and `a` to `b` through the array of glyphs, and
    /// the last segment that every such subtable ends with.
    fn segments() -> Vec<u8> {
        let mut table = Vec::new();
        let words: [u16; 24] = [
            4,
            48,
            0,
            6,
            0,
            0,
            0, // format, length, language, segment count × 2, search fields
            0x43,
            0x62,
            0xFFFF, // ends
            0,      // padding
            0x41,
            0x61,
            0xFFFF, // starts
            (10u16).wrapping_sub(0x41),
            0,
            1, // deltas
            0,
            4,
            0, // range offsets: the second points 4 bytes on, at the array
            20,
            21,
            0,
            0, // the array of glyphs, and room
        ];
        for word in words {
            table.extend_from_slice(&word.to_be_bytes());
        }
        table
    }

    #[test]
    fn format_4_maps_by_delta_and_by_array() {
        let table = segments();
        let reader = Reader::new(&table);
        assert_eq!(segment_glyph(reader, 0x41), Some(10));
        assert_eq!(segment_glyph(reader, 0x43), Some(12));
        assert_eq!(segment_glyph(reader, 0x44), None);
        assert_eq!(segment_glyph(reader, 0x61), Some(20));
        assert_eq!(segment_glyph(reader, 0x62), Some(21));
        assert_eq!(segment_glyph(reader, 0x1_0000), None);
    }

    #[test]
    fn format_12_maps_groups_of_the_whole_of_unicode() {
        let mut table = Vec::new();
        table.extend_from_slice(&12u16.to_be_bytes());
        table.extend_from_slice(&[0, 0]);
        for word in [40u32, 0, 2, 0x41, 0x5A, 3, 0x1F600, 0x1F64F, 100] {
            table.extend_from_slice(&word.to_be_bytes());
        }
        let reader = Reader::new(&table);
        assert_eq!(group_glyph(reader, 0x41), Some(3));
        assert_eq!(group_glyph(reader, 0x5A), Some(28));
        assert_eq!(group_glyph(reader, 0x1F601), Some(101));
        assert_eq!(group_glyph(reader, 0x60), None);
    }
}
