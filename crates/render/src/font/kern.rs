//! The `kern` table of older fonts: format 0 subtables, sorted pairs of glyphs with the change of advance between them.

use super::reader::Reader;

/// The bit of a subtable's coverage that says it is horizontal kerning.
const HORIZONTAL: u16 = 0x0001;
/// The bit that says its values are minimums rather than kerning.
const MINIMUM: u16 = 0x0002;
/// The bit that says it moves glyphs across the line rather than along it.
const CROSS_STREAM: u16 = 0x0004;

/// The pairs of every horizontal kerning subtable of format 0 in a `kern` table of version 0, as a key of the two
/// glyphs, first in the high half, and the change of advance; their values add up when several subtables name the
/// same pair.
pub(crate) fn pairs(kern: Reader<'_>) -> Vec<(u32, i16)> {
    let mut pairs = Vec::new();
    if kern.u16(0) != Some(0) {
        return pairs;
    }
    let table_count = kern.u16(2).unwrap_or(0);
    let mut at = 4;
    for _ in 0..table_count {
        let (Some(length), Some(coverage)) = (kern.offset16(at + 2), kern.u16(at + 4)) else {
            break;
        };
        let format = coverage >> 8;
        let is_kerning = coverage & (HORIZONTAL | MINIMUM | CROSS_STREAM) == HORIZONTAL;
        if format == 0 && is_kerning {
            read_pairs(kern, at + 6, &mut pairs);
        }
        at += length.max(6);
    }
    pairs.sort_by_key(|(key, _)| *key);
    pairs.dedup_by(|later, earlier| {
        let is_same = later.0 == earlier.0;
        if is_same {
            earlier.1 = earlier.1.saturating_add(later.1);
        }
        is_same
    });
    pairs
}

/// Reads the pairs of a subtable of format 0, from its count of pairs.
fn read_pairs(kern: Reader<'_>, at: usize, pairs: &mut Vec<(u32, i16)>) {
    let count = kern.offset16(at).unwrap_or(0);
    for pair in 0..count {
        let record = at + 8 + pair * 6;
        if let (Some(key), Some(value)) = (kern.u32(record), kern.i16(record + 4)) {
            pairs.push((key, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_pairs_are_read_and_others_left_out() {
        let mut table: Vec<u8> = Vec::new();
        for word in [0u16, 2] {
            table.extend_from_slice(&word.to_be_bytes());
        }
        // A horizontal subtable of two pairs, then a cross-stream one of one pair.
        for (coverage, pairs) in [
            (0x0001u16, vec![(3u16, 4u16, -50i16), (1, 2, 20)]),
            (0x0005, vec![(1, 2, 99)]),
        ] {
            let length = u16::try_from(14 + pairs.len() * 6).unwrap();
            for word in [0, length, coverage, u16::try_from(pairs.len()).unwrap(), 0, 0, 0] {
                table.extend_from_slice(&word.to_be_bytes());
            }
            for (first, second, value) in pairs {
                table.extend_from_slice(&first.to_be_bytes());
                table.extend_from_slice(&second.to_be_bytes());
                table.extend_from_slice(&value.to_be_bytes());
            }
        }
        assert_eq!(
            pairs(Reader::new(&table)),
            vec![((1 << 16) | 2, 20), ((3 << 16) | 4, -50)]
        );
    }
}
