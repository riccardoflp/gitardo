//! The table directory of a font file, and the small tables of its sizes and metrics: `head`, `hhea`, `maxp` and
//! `OS/2`.

use std::ops::Range;

use super::reader::Reader;
use crate::error::Error;

/// The version of a font with TrueType outlines.
const TRUETYPE_VERSION: u32 = 0x0001_0000;
/// The version Apple gives some TrueType fonts: `true`.
const APPLE_TRUETYPE_VERSION: u32 = u32::from_be_bytes(*b"true");
/// The version of a font with CFF outlines: `OTTO`.
const CFF_VERSION: u32 = u32::from_be_bytes(*b"OTTO");
/// The tag of a collection of fonts: `ttcf`.
const COLLECTION_TAG: u32 = u32::from_be_bytes(*b"ttcf");
/// The bit of `fsSelection` in `OS/2` that says the typographic metrics are the ones to use for lines.
const USE_TYPOGRAPHIC_METRICS: u16 = 1 << 7;

/// How the offsets of `loca` are written, as `head` says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocationFormat {
    /// 16-bit offsets, each half the real offset.
    Short,
    /// 32-bit offsets.
    Long,
}

/// Where each table of a font is in its file.
#[derive(Clone, Debug)]
pub(crate) struct TableDirectory {
    /// The tag and the range of bytes of each table.
    tables: Vec<([u8; 4], Range<usize>)>,
}

impl TableDirectory {
    /// Reads the directory of the font at a position in a file, which is the font itself when the file is not a
    /// collection.
    pub(crate) fn read(file: Reader<'_>, index: u32) -> Result<TableDirectory, Error> {
        let start = font_offset(file, index)?;
        let font = file.from(start).ok_or(Error::NotAFont)?;
        match font.u32(0).ok_or(Error::NotAFont)? {
            TRUETYPE_VERSION | APPLE_TRUETYPE_VERSION => {}
            CFF_VERSION => return Err(Error::UnsupportedOutlines),
            _ => return Err(Error::NotAFont),
        }
        let count = font.u16(4).ok_or(Error::NotAFont)?;
        let mut tables = Vec::with_capacity(usize::from(count));
        for table in 0..usize::from(count) {
            let record = font.slice(12 + table * 16, 16).ok_or(Error::NotAFont)?;
            let tag: [u8; 4] = record
                .bytes()
                .get(..4)
                .and_then(|tag| tag.try_into().ok())
                .ok_or(Error::NotAFont)?;
            let offset = record.offset32(8).ok_or(Error::NotAFont)?;
            let length = record.offset32(12).ok_or(Error::NotAFont)?;
            tables.push((tag, offset..offset.saturating_add(length)));
        }
        Ok(TableDirectory { tables })
    }

    /// Where a table is in the file, when the font has it and it is inside the file.
    pub(crate) fn find(&self, file: Reader<'_>, tag: &'static str) -> Option<Range<usize>> {
        let (_, range) = self.tables.iter().find(|(name, _)| name == tag.as_bytes())?;
        (range.end <= file.len()).then(|| range.clone())
    }

    /// Where a table every font needs is in the file.
    pub(crate) fn range(&self, file: Reader<'_>, tag: &'static str) -> Result<Range<usize>, Error> {
        self.find(file, tag).ok_or(Error::MissingTable { table: tag })
    }

    /// A table every font needs.
    pub(crate) fn required<'data>(&self, file: Reader<'data>, tag: &'static str) -> Result<Reader<'data>, Error> {
        self.table(file, tag).ok_or(Error::MissingTable { table: tag })
    }

    /// A table the font may have.
    pub(crate) fn table<'data>(&self, file: Reader<'data>, tag: &'static str) -> Option<Reader<'data>> {
        let range = self.find(file, tag)?;
        file.slice(range.start, range.len())
    }
}

/// Where the font at a position starts in a file: 0 for a single font, its offset in the header of a collection.
fn font_offset(file: Reader<'_>, index: u32) -> Result<usize, Error> {
    if file.u32(0) != Some(COLLECTION_TAG) {
        return if index == 0 {
            Ok(0)
        } else {
            Err(Error::NoSuchFont { index, count: 1 })
        };
    }
    let count = file.u32(8).ok_or(Error::NotAFont)?;
    if index >= count {
        return Err(Error::NoSuchFont { index, count });
    }
    let position = usize::try_from(index).map_err(|_| Error::NotAFont)?;
    file.offset32(12 + position * 4).ok_or(Error::NotAFont)
}

/// What the font header says that drawing needs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Head {
    /// Font units in the side of the em square.
    pub(crate) units_per_em: u16,
    /// How the offsets of `loca` are written.
    pub(crate) location_format: LocationFormat,
}

impl Head {
    /// Reads the font header.
    pub(crate) fn read(head: Reader<'_>) -> Result<Head, Error> {
        let damaged = Error::Damaged { table: "head" };
        let units_per_em = head.u16(18).ok_or(damaged.clone())?;
        if !(16..=16384).contains(&units_per_em) {
            return Err(damaged);
        }
        let location_format = match head.i16(50).ok_or(damaged.clone())? {
            0 => LocationFormat::Short,
            1 => LocationFormat::Long,
            _ => return Err(damaged),
        };
        Ok(Head {
            units_per_em,
            location_format,
        })
    }
}

/// The heights of the font's lines: from `hhea`, or from the typographic metrics of `OS/2` when it says to use them.
pub(crate) fn vertical_metrics(hhea: Reader<'_>, os2: Option<Reader<'_>>) -> Result<super::VerticalMetrics, Error> {
    let damaged = Error::Damaged { table: "hhea" };
    let from_hhea = super::VerticalMetrics {
        ascender: hhea.i16(4).ok_or(damaged.clone())?,
        descender: hhea.i16(6).ok_or(damaged.clone())?,
        line_gap: hhea.i16(8).ok_or(damaged)?,
    };
    let typographic = os2.and_then(|os2| {
        let selection = os2.u16(62)?;
        (selection & USE_TYPOGRAPHIC_METRICS != 0).then_some(super::VerticalMetrics {
            ascender: os2.i16(68)?,
            descender: os2.i16(70)?,
            line_gap: os2.i16(72)?,
        })
    });
    Ok(typographic.unwrap_or(from_hhea))
}

/// The number of glyphs with an advance of their own in `hmtx`, as `hhea` says.
pub(crate) fn horizontal_metric_count(hhea: Reader<'_>) -> Result<u16, Error> {
    match hhea.u16(34) {
        Some(count) if count > 0 => Ok(count),
        _ => Err(Error::Damaged { table: "hhea" }),
    }
}

/// The number of glyphs in the font, as `maxp` says.
pub(crate) fn glyph_count(maxp: Reader<'_>) -> Result<u16, Error> {
    maxp.u16(4).ok_or(Error::Damaged { table: "maxp" })
}
