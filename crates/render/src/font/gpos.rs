//! Kerning: the pair adjustments of the `kern` feature of `GPOS` (lookups of type 2, in formats 1 and 2, also behind
//! lookups of type 9 that extend them), or the pairs of the `kern` table for fonts without them. Only the change of
//! advance of the first glyph is kept, which is what spacing text along a line needs.

use super::GlyphId;
use super::coverage::{ClassDefinition, Coverage};
use super::kern;
use super::reader::Reader;

/// The lookup type of pair adjustments.
const PAIR_ADJUSTMENT: u16 = 2;
/// The lookup type that holds a subtable of another type further in the table.
const EXTENSION: u16 = 9;
/// The bit of a value format that says the record holds a change of advance along the line.
const X_ADVANCE: u16 = 0x0004;
/// The most class pairs a subtable may hold, so that a damaged font cannot take all the memory.
const MOST_CLASS_PAIRS: usize = 1 << 22;

/// The kerning of a font: lookups applied one after the other, their changes added up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Kerning {
    /// The lookups, each a list of subtables of which the first that applies to a pair gives its value.
    lookups: Vec<Vec<PairSubtable>>,
}

/// One subtable of pair adjustments.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PairSubtable {
    /// Pairs of glyphs, as a key with the first glyph in the high half, sorted, with the change of advance.
    Pairs(Vec<(u32, i16)>),
    /// Pairs of classes of glyphs.
    Classes(ClassPairs),
}

/// A subtable that gives values to pairs of classes: the first glyph must be in its coverage.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ClassPairs {
    /// The glyphs that may come first.
    coverage: Coverage,
    /// The classes of the first glyph.
    first: ClassDefinition,
    /// The classes of the second glyph.
    second: ClassDefinition,
    /// The number of classes of the second glyph.
    second_count: usize,
    /// The change of advance of each pair of classes, row by row of the first class.
    values: Vec<i16>,
}

impl Kerning {
    /// The kerning of a font from its `GPOS` table, or from its `kern` table when `GPOS` has no kerning.
    pub(crate) fn read(gpos: Option<Reader<'_>>, kern: Option<Reader<'_>>) -> Kerning {
        let lookups = gpos.map(read_lookups).unwrap_or_default();
        if !lookups.is_empty() {
            return Kerning { lookups };
        }
        let pairs = kern.map(kern::pairs).unwrap_or_default();
        if pairs.is_empty() {
            return Kerning::default();
        }
        Kerning {
            lookups: vec![vec![PairSubtable::Pairs(pairs)]],
        }
    }

    /// The change of the advance of the first glyph of a pair, in font units.
    pub(crate) fn between(&self, first: GlyphId, second: GlyphId) -> i16 {
        self.lookups
            .iter()
            .filter_map(|lookup| lookup.iter().find_map(|subtable| subtable.value(first.0, second.0)))
            .fold(0, i16::saturating_add)
    }
}

impl PairSubtable {
    /// The change the subtable gives a pair, or nothing when it does not apply to it.
    fn value(&self, first: u16, second: u16) -> Option<i16> {
        match self {
            PairSubtable::Pairs(pairs) => {
                let key = (u32::from(first) << 16) | u32::from(second);
                let found = pairs.binary_search_by_key(&key, |(pair, _)| *pair).ok()?;
                pairs.get(found).map(|(_, value)| *value)
            }
            PairSubtable::Classes(classes) => {
                let _ = classes.coverage.index(first)?;
                let row = usize::from(classes.first.class(first));
                let column = usize::from(classes.second.class(second));
                if column >= classes.second_count {
                    return None;
                }
                classes.values.get(row * classes.second_count + column).copied()
            }
        }
    }
}

/// The subtables of pair adjustments of every lookup of the `kern` feature, lookup by lookup in the order of the
/// lookup list. The lookups of every script and language are taken together, which is what Latin, Greek and Cyrillic
/// text needs.
fn read_lookups(gpos: Reader<'_>) -> Vec<Vec<PairSubtable>> {
    let (Some(features), Some(lookups)) = (gpos.offset16(6), gpos.offset16(8)) else {
        return Vec::new();
    };
    let mut indices = kern_lookup_indices(gpos, features).unwrap_or_default();
    indices.sort_unstable();
    indices.dedup();
    indices
        .into_iter()
        .filter_map(|index| {
            let lookup = gpos.from(lookups + gpos.offset16(lookups + 2 + usize::from(index) * 2)?)?;
            let subtables = read_lookup(lookup)?;
            (!subtables.is_empty()).then_some(subtables)
        })
        .collect()
}

/// The indices of the lookups of every `kern` feature in the feature list.
fn kern_lookup_indices(gpos: Reader<'_>, features: usize) -> Option<Vec<u16>> {
    let count = gpos.offset16(features)?;
    let mut indices = Vec::new();
    for feature in 0..count {
        let record = features + 2 + feature * 6;
        if gpos.slice(record, 4)?.bytes() != b"kern" {
            continue;
        }
        let table = features + gpos.offset16(record + 4)?;
        let lookup_count = gpos.offset16(table + 2)?;
        for lookup in 0..lookup_count {
            indices.push(gpos.u16(table + 4 + lookup * 2)?);
        }
    }
    Some(indices)
}

/// The subtables of a lookup of pair adjustments, following extensions; nothing for a lookup of another type.
fn read_lookup(lookup: Reader<'_>) -> Option<Vec<PairSubtable>> {
    let kind = lookup.u16(0)?;
    let count = lookup.offset16(4)?;
    let mut subtables = Vec::with_capacity(count);
    for index in 0..count {
        let mut subtable = lookup.from(lookup.offset16(6 + index * 2)?)?;
        let mut subtable_kind = kind;
        if kind == EXTENSION {
            subtable_kind = subtable.u16(2)?;
            subtable = subtable.from(subtable.offset32(4)?)?;
        }
        if subtable_kind != PAIR_ADJUSTMENT {
            return None;
        }
        if let Some(read) = read_pair_subtable(subtable) {
            subtables.push(read);
        }
    }
    Some(subtables)
}

/// The two value formats of a subtable, as the size of each record and where the change of advance of the first
/// glyph is in the first record.
#[derive(Clone, Copy, Debug)]
struct Values {
    /// The bytes of the two records together.
    size: usize,
    /// Where the change of advance of the first glyph is, or nothing when the records do not hold one.
    advance: Option<usize>,
}

impl Values {
    /// The layout of the records from their two formats: each field present takes two bytes, in the order of the bits.
    fn of(first_format: u16, second_format: u16) -> Values {
        let field_bytes = |format: u16| usize::try_from(format.count_ones()).unwrap_or(0) * 2;
        let advance = (first_format & X_ADVANCE != 0).then(|| field_bytes(first_format & (X_ADVANCE - 1)));
        Values {
            size: field_bytes(first_format) + field_bytes(second_format),
            advance,
        }
    }

    /// The change of advance in the records at an offset.
    fn advance_at(self, table: Reader<'_>, at: usize) -> Option<i16> {
        self.advance.map_or(Some(0), |offset| table.i16(at + offset))
    }
}

/// A subtable of pair adjustments, of format 1 (pairs of glyphs) or 2 (pairs of classes).
fn read_pair_subtable(table: Reader<'_>) -> Option<PairSubtable> {
    let coverage = Coverage::read(table.from(table.offset16(2)?)?)?;
    let values = Values::of(table.u16(4)?, table.u16(6)?);
    match table.u16(0)? {
        1 => {
            let mut pairs = Vec::new();
            for (glyph, index) in coverage.glyphs() {
                let set = table.offset16(10 + usize::from(index) * 2)?;
                let count = table.offset16(set)?;
                for pair in 0..count {
                    let record = set + 2 + pair * (2 + values.size);
                    let second = table.u16(record)?;
                    let value = values.advance_at(table, record + 2)?;
                    pairs.push(((u32::from(glyph) << 16) | u32::from(second), value));
                }
            }
            pairs.sort_by_key(|(key, _)| *key);
            Some(PairSubtable::Pairs(pairs))
        }
        2 => read_class_pairs(table, coverage, values).map(PairSubtable::Classes),
        _ => None,
    }
}

/// A subtable of pairs of classes: the two class definitions, then a record for each pair of classes.
fn read_class_pairs(table: Reader<'_>, coverage: Coverage, values: Values) -> Option<ClassPairs> {
    let first = ClassDefinition::read(table.from(table.offset16(8)?)?)?;
    let second = ClassDefinition::read(table.from(table.offset16(10)?)?)?;
    let first_count = table.offset16(12)?;
    let second_count = table.offset16(14)?;
    let pair_count = first_count
        .checked_mul(second_count)
        .filter(|count| *count <= MOST_CLASS_PAIRS)?;
    let mut changes = Vec::with_capacity(pair_count);
    for pair in 0..pair_count {
        changes.push(values.advance_at(table, 16 + pair * values.size)?);
    }
    Some(ClassPairs {
        coverage,
        first,
        second,
        second_count,
        values: changes,
    })
}
