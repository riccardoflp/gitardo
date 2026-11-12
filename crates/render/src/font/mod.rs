//! TrueType fonts: the tables of a font file read from the OpenType specification (Microsoft, "OpenType
//! Specification", version 1.9), enough to find the glyph of a character, its outline, its advance, and the kerning
//! between two glyphs.

mod cmap;
mod coverage;
mod glyf;
mod gpos;
mod kern;
mod reader;
mod tables;

use std::ops::Range;

use crate::error::Error;
use crate::path::Path;
use cmap::CharacterMap;
use gpos::Kerning;
use reader::Reader;
use tables::{LocationFormat, TableDirectory};

/// The number of a glyph in its font; glyph 0 is the one a font draws for a character it lacks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(pub u16);

/// The heights of a font's lines, in font units, `y` growing upwards from the baseline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VerticalMetrics {
    /// How far the font reaches above the baseline.
    pub ascender: i16,
    /// How far the font reaches below the baseline, as a negative number.
    pub descender: i16,
    /// The space the font asks for between one line's descender and the next one's ascender.
    pub line_gap: i16,
}

/// A TrueType font read from its file: one face, such as the bold face of a family.
#[derive(Clone, Debug)]
pub struct Font {
    /// The whole file.
    data: Vec<u8>,
    /// Font units in the side of the em square, which the size of the font scales to.
    units_per_em: u16,
    /// The heights of its lines.
    vertical: VerticalMetrics,
    /// The number of glyphs.
    glyph_count: u16,
    /// The number of glyphs with an advance of their own in `hmtx`; those after share the last one's.
    horizontal_metric_count: u16,
    /// Where `hmtx` is in the file.
    horizontal_metrics: Range<usize>,
    /// Where `loca` is in the file, and the size of its offsets.
    locations: (Range<usize>, LocationFormat),
    /// Where `glyf` is in the file.
    glyphs: Range<usize>,
    /// The subtable of `cmap` used to find the glyph of a character.
    characters: CharacterMap,
    /// The kerning between pairs of glyphs, from `GPOS`, or from `kern` when the font has no `GPOS` kerning.
    kerning: Kerning,
}

impl Font {
    /// Reads a font from the bytes of a TrueType or OpenType file with TrueType outlines (`.ttf`, `.otf`), or the
    /// first font of a collection (`.ttc`).
    ///
    /// # Errors
    ///
    /// When the bytes are not such a font, its outlines are CFF ones, or a table it needs is missing or damaged.
    pub fn from_bytes(data: Vec<u8>) -> Result<Font, Error> {
        Font::from_collection(data, 0)
    }

    /// Reads the font at a position, from 0, of a collection of fonts (`.ttc`); for a single font, position 0 is the
    /// font.
    ///
    /// # Errors
    ///
    /// As [`Font::from_bytes`], and when the collection has no font at that position.
    pub fn from_collection(data: Vec<u8>, index: u32) -> Result<Font, Error> {
        let file = Reader::new(&data);
        let directory = TableDirectory::read(file, index)?;
        let head = tables::Head::read(directory.required(file, "head")?)?;
        let vertical = tables::vertical_metrics(directory.required(file, "hhea")?, directory.table(file, "OS/2"))?;
        let horizontal_metric_count = tables::horizontal_metric_count(directory.required(file, "hhea")?)?;
        let glyph_count = tables::glyph_count(directory.required(file, "maxp")?)?;
        let characters = CharacterMap::read(directory.required(file, "cmap")?, directory.range(file, "cmap")?.start)?;
        let kerning = Kerning::read(directory.table(file, "GPOS"), directory.table(file, "kern"));
        let font = Font {
            units_per_em: head.units_per_em,
            vertical,
            glyph_count,
            horizontal_metric_count,
            horizontal_metrics: directory.range(file, "hmtx")?,
            locations: (directory.range(file, "loca")?, head.location_format),
            glyphs: directory.range(file, "glyf")?,
            characters,
            kerning,
            data,
        };
        Ok(font)
    }

    /// Font units in the side of the em square: a font drawn at 16 pixels draws this many units in 16 pixels.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// The heights of the font's lines.
    #[must_use]
    pub fn vertical_metrics(&self) -> VerticalMetrics {
        self.vertical
    }

    /// The number of glyphs in the font.
    #[must_use]
    pub fn glyph_count(&self) -> u16 {
        self.glyph_count
    }

    /// The glyph the font draws for a character, or nothing when it has none.
    #[must_use]
    pub fn glyph_id(&self, character: char) -> Option<GlyphId> {
        let glyph = self.characters.glyph(Reader::new(&self.data), u32::from(character))?;
        (glyph != 0 && glyph < self.glyph_count).then_some(GlyphId(glyph))
    }

    /// How far the pen moves after a glyph, in font units.
    #[must_use]
    pub fn advance(&self, glyph: GlyphId) -> u16 {
        let metrics = self.table(&self.horizontal_metrics);
        let last = self.horizontal_metric_count.saturating_sub(1);
        let index = usize::from(glyph.0.min(last));
        metrics.u16(index * 4).unwrap_or(0)
    }

    /// The change of the advance of the first glyph of a pair when the second follows it, in font units: negative to
    /// bring the two closer, as in `AV`.
    #[must_use]
    pub fn kerning(&self, first: GlyphId, second: GlyphId) -> i16 {
        self.kerning.between(first, second)
    }

    /// The outline of a glyph in font units, `y` growing upwards from the baseline; nothing when the glyph draws
    /// nothing, as a space does, or its data is damaged.
    #[must_use]
    pub fn outline(&self, glyph: GlyphId) -> Option<Path> {
        let source = glyf::Source {
            locations: self.table(&self.locations.0),
            format: self.locations.1,
            glyphs: self.table(&self.glyphs),
            glyph_count: self.glyph_count,
        };
        let path = source.outline(glyph)?;
        (!path.is_empty()).then_some(path)
    }

    /// A table of the font, read from where it is in the file.
    fn table(&self, range: &Range<usize>) -> Reader<'_> {
        Reader::new(self.data.get(range.clone()).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
