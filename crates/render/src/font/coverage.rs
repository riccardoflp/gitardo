//! The coverage tables and class definitions of OpenType's layout tables, read once into ranges of glyphs.

use super::reader::Reader;

/// A range of glyphs, both ends included, and the number it gives them: a coverage index or a class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GlyphRange {
    /// The first glyph.
    first: u16,
    /// The last glyph.
    last: u16,
    /// For a coverage, the index of the first glyph; for a class definition, the class of every glyph.
    value: u16,
}

/// Which glyphs a subtable applies to, and the index of each among them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Coverage {
    /// The ranges, sorted by glyph.
    ranges: Vec<GlyphRange>,
}

impl Coverage {
    /// Reads a coverage table: format 1, a sorted list of glyphs, or format 2, sorted ranges of glyphs with the index
    /// of their first one.
    pub(crate) fn read(table: Reader<'_>) -> Option<Coverage> {
        let count = table.offset16(2)?;
        let mut ranges = Vec::with_capacity(count);
        match table.u16(0)? {
            1 => {
                for index in 0..count {
                    let glyph = table.u16(4 + index * 2)?;
                    let value = u16::try_from(index).ok()?;
                    ranges.push(GlyphRange {
                        first: glyph,
                        last: glyph,
                        value,
                    });
                }
            }
            2 => {
                for index in 0..count {
                    let at = 4 + index * 6;
                    let (first, last, value) = (table.u16(at)?, table.u16(at + 2)?, table.u16(at + 4)?);
                    ranges.push(GlyphRange { first, last, value });
                }
            }
            _ => return None,
        }
        ranges.sort_by_key(|range| range.first);
        Some(Coverage { ranges })
    }

    /// The index of a glyph in the coverage, or nothing when the coverage does not hold it.
    pub(crate) fn index(&self, glyph: u16) -> Option<u16> {
        let range = find(&self.ranges, glyph)?;
        range.value.checked_add(glyph - range.first)
    }

    /// Every glyph of the coverage with its index, in the order of the glyphs.
    pub(crate) fn glyphs(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.ranges.iter().flat_map(|range| {
            (range.first..=range.last).map(move |glyph| (glyph, range.value.wrapping_add(glyph - range.first)))
        })
    }
}

/// The class of each glyph, for subtables that give values to pairs of classes rather than of glyphs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClassDefinition {
    /// The ranges, sorted by glyph; a glyph in none is in class 0.
    ranges: Vec<GlyphRange>,
}

impl ClassDefinition {
    /// Reads a class definition: format 1, the classes of consecutive glyphs from a first one, or format 2, sorted
    /// ranges of glyphs with their class.
    pub(crate) fn read(table: Reader<'_>) -> Option<ClassDefinition> {
        let mut ranges = Vec::new();
        match table.u16(0)? {
            1 => {
                let first = table.u16(2)?;
                let count = table.offset16(4)?;
                ranges.reserve(count);
                for index in 0..count {
                    let glyph = first.checked_add(u16::try_from(index).ok()?)?;
                    let value = table.u16(6 + index * 2)?;
                    ranges.push(GlyphRange {
                        first: glyph,
                        last: glyph,
                        value,
                    });
                }
            }
            2 => {
                let count = table.offset16(2)?;
                ranges.reserve(count);
                for index in 0..count {
                    let at = 4 + index * 6;
                    let (first, last, value) = (table.u16(at)?, table.u16(at + 2)?, table.u16(at + 4)?);
                    ranges.push(GlyphRange { first, last, value });
                }
            }
            _ => return None,
        }
        ranges.sort_by_key(|range| range.first);
        Some(ClassDefinition { ranges })
    }

    /// The class of a glyph; 0 for a glyph the definition does not name.
    pub(crate) fn class(&self, glyph: u16) -> u16 {
        find(&self.ranges, glyph).map_or(0, |range| range.value)
    }
}

/// The range that holds a glyph, among ranges sorted by their first glyph that do not overlap.
fn find(ranges: &[GlyphRange], glyph: u16) -> Option<&GlyphRange> {
    let after = ranges.partition_point(|range| range.first <= glyph);
    let range = ranges.get(after.checked_sub(1)?)?;
    (glyph <= range.last).then_some(range)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    #[test]
    fn coverages_give_the_index_of_their_glyphs() {
        let list = bytes(&[1, 3, 5, 9, 12]);
        let coverage = Coverage::read(Reader::new(&list)).unwrap();
        assert_eq!(
            (coverage.index(9), coverage.index(10), coverage.index(12)),
            (Some(1), None, Some(2))
        );
        let ranges = bytes(&[2, 2, 10, 14, 0, 20, 21, 5]);
        let coverage = Coverage::read(Reader::new(&ranges)).unwrap();
        assert_eq!(
            (coverage.index(13), coverage.index(21), coverage.index(15)),
            (Some(3), Some(6), None)
        );
        assert_eq!(coverage.glyphs().count(), 7);
    }

    #[test]
    fn class_definitions_put_unnamed_glyphs_in_class_0() {
        let consecutive = bytes(&[1, 40, 3, 1, 2, 1]);
        let classes = ClassDefinition::read(Reader::new(&consecutive)).unwrap();
        assert_eq!(
            (
                classes.class(40),
                classes.class(41),
                classes.class(43),
                classes.class(3)
            ),
            (1, 2, 0, 0)
        );
        let ranges = bytes(&[2, 1, 100, 110, 7]);
        let classes = ClassDefinition::read(Reader::new(&ranges)).unwrap();
        assert_eq!((classes.class(105), classes.class(111)), (7, 0));
    }
}
