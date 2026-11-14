//! The engine of text: the font system of `cosmic-text` with the fonts loaded, the lines it has shaped and the glyphs
//! it has rasterized, kept so that a line is shaped once and a glyph rasterized once while they are in use.

use std::borrow::Cow;
use std::collections::HashMap;

use cosmic_text::{
    Attrs, AttrsList, CacheKey, Family, FontSystem, Hinting, LayoutGlyph, ShapeLine, Shaping, SwashCache, SwashImage,
    Weight, Wrap, fontdb,
};

/// How many spaces a tab is as wide as.
const TAB_WIDTH: u16 = 4;

/// The most lines kept shaped: past this they are all forgotten, which only costs shaping the lines in sight again.
const MOST_LINES: usize = 8_192;

/// The most glyphs kept rasterized: past this they are all forgotten, which only costs rasterizing the glyphs in sight
/// again.
const MOST_GLYPHS: usize = 16_384;

/// The language the fonts are chosen for when a character has different forms in different languages, as some Chinese
/// characters do. Fixed, so that the program draws the same text the same way whatever the system says.
const LOCALE: &str = "en-US";

/// A face of a family, as the font system looks for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Face {
    /// The name of the family.
    pub(crate) family: String,
    /// The weight, from 100 (thin) to 900 (black).
    pub(crate) weight: u16,
}

impl Face {
    /// The attributes of text in this face.
    fn attributes(&self) -> Attrs<'_> {
        Attrs::new()
            .family(Family::Name(&self.family))
            .weight(Weight(self.weight))
    }
}

/// A line of text shaped in a face at a size: its glyphs on one line, from the left, and what wrapping it needs.
#[derive(Clone, Debug)]
pub(crate) struct Shaped {
    /// The line shaped, which can be laid out again at a width.
    pub(crate) shape: ShapeLine,
    /// The glyphs on one line, in the order they are drawn, their positions in pixels at the size.
    pub(crate) glyphs: Vec<LayoutGlyph>,
    /// The width of the line, in pixels at the size.
    pub(crate) width: f32,
}

impl Shaped {
    /// The lines of the text laid out to a width, each as the glyphs on it; a line breaks between words, or inside a
    /// word that is wider than the width.
    pub(crate) fn wrapped(&self, size: f32, width: f32) -> Vec<Vec<LayoutGlyph>> {
        let lines = self
            .shape
            .layout(size, Some(width), Wrap::WordOrGlyph, None, None, Hinting::Disabled);
        lines.into_iter().map(|line| line.glyphs).collect()
    }
}

/// Which lines are shaped alike: those of one typeface at one size, its bits.
type LineKey = (u16, u32);

/// The font system, the lines shaped and the glyphs rasterized.
pub(crate) struct Engine {
    /// The fonts and how one is found for each character.
    pub(crate) system: FontSystem,
    /// The lines shaped, by typeface and size, then by text.
    lines: HashMap<LineKey, HashMap<String, Shaped>>,
    /// How many lines are kept shaped.
    line_count: usize,
    /// The glyphs rasterized.
    glyphs: SwashCache,
    /// How many glyphs were rasterized since the engine was made.
    rasterized: u64,
}

impl Engine {
    /// An engine over a database of fonts.
    pub(crate) fn new(database: fontdb::Database) -> Engine {
        Engine {
            system: FontSystem::new_with_locale_and_db(LOCALE.to_owned(), database),
            lines: HashMap::new(),
            line_count: 0,
            glyphs: SwashCache::new(),
            rasterized: 0,
        }
    }

    /// Adds the fonts installed on the system to those loaded, reading the folders of fonts of the system. What was
    /// shaped and rasterized is forgotten, since a character may now be drawn from another font.
    pub(crate) fn load_system_fonts(&mut self) {
        let system = std::mem::replace(
            &mut self.system,
            FontSystem::new_with_locale_and_db(String::new(), fontdb::Database::new()),
        );
        let (locale, mut database) = system.into_locale_and_db();
        database.load_system_fonts();
        self.system = FontSystem::new_with_locale_and_db(locale, database);
        self.lines.clear();
        self.line_count = 0;
        self.glyphs = SwashCache::new();
    }

    /// The heights of the first font of a face, as fractions of its em: above the baseline, below it, and the gap
    /// between lines; nothing when no font loaded belongs to its family.
    pub(crate) fn face_heights(&mut self, face: &Face) -> Option<(f32, f32, f32)> {
        let families = [Family::Name(&face.family)];
        let query = fontdb::Query {
            families: &families,
            weight: Weight(face.weight),
            ..fontdb::Query::default()
        };
        let id = self.system.db().query(&query)?;
        let font = self.system.get_font(id, Weight(face.weight))?;
        let metrics = font.metrics();
        let em = f32::from(metrics.units_per_em.max(1));
        Some((
            metrics.ascent / em,
            -metrics.descent / em,
            metrics.leading.max(0.0) / em,
        ))
    }

    /// A line of text shaped in a face at a size, shaped the first time it is asked for.
    pub(crate) fn shaped(&mut self, key: LineKey, face: &Face, text: &str, size: f32) -> Option<&Shaped> {
        if self.line_count >= MOST_LINES {
            self.lines.clear();
            self.line_count = 0;
        }
        let lines = self.lines.entry(key).or_default();
        if !lines.contains_key(text) {
            let shaped = shape(&mut self.system, face, text, size);
            let _ = lines.insert(text.to_owned(), shaped);
            self.line_count += 1;
        }
        lines.get(text)
    }

    /// The image of a glyph at its size and position inside a pixel, rasterized the first time it is asked for; nothing
    /// for a glyph that draws nothing, such as a space.
    pub(crate) fn glyph(&mut self, key: CacheKey) -> Option<&SwashImage> {
        if !self.glyphs.image_cache.contains_key(&key) {
            if self.glyphs.image_cache.len() >= MOST_GLYPHS {
                self.glyphs.image_cache.clear();
            }
            self.rasterized += 1;
        }
        self.glyphs.get_image(&mut self.system, key).as_ref()
    }

    /// How many glyphs were rasterized since the engine was made.
    pub(crate) fn rasterized(&self) -> u64 {
        self.rasterized
    }
}

/// Shapes a line of text in a face at a size: finds a font for each character, the face's first, joins and positions
/// the glyphs as the script needs, and lays them out on one line.
fn shape(system: &mut FontSystem, face: &Face, text: &str, size: f32) -> Shaped {
    let attributes = AttrsList::new(&face.attributes());
    let shape = ShapeLine::new(system, &one_paragraph(text), &attributes, Shaping::Advanced, TAB_WIDTH);
    let line = shape
        .layout(size, None, Wrap::None, None, None, Hinting::Disabled)
        .into_iter()
        .next();
    let (glyphs, width) = line.map_or_else(|| (Vec::new(), 0.0), |line| (line.glyphs, line.w));
    Shaped { shape, glyphs, width }
}

/// The text with every character that ends a paragraph, such as a line break, made a character of the same length in
/// bytes that does not, so that the line is shaped as one paragraph and the bytes of its glyphs are those of the text.
fn one_paragraph(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_paragraph_end) {
        return Cow::Borrowed(text);
    }
    let mut line = String::with_capacity(text.len());
    for character in text.chars() {
        match character.len_utf8() {
            _ if !is_paragraph_end(character) => line.push(character),
            1 => line.push(' '),
            // A soft hyphen and a space of no width are not drawn, and are as long as the separators they replace.
            2 => line.push('\u{AD}'),
            _ => line.push('\u{200B}'),
        }
    }
    Cow::Owned(line)
}

/// Whether a character ends a paragraph, by the Unicode Bidirectional Algorithm (class B).
fn is_paragraph_end(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{1C}'..='\u{1E}' | '\u{85}' | '\u{2029}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_become_characters_of_the_same_length() {
        assert_eq!(one_paragraph("plain"), Cow::Borrowed("plain"));
        let replaced = one_paragraph("a\nb\r\u{85}c\u{2029}d");
        assert_eq!(replaced, "a b \u{AD}c\u{200B}d");
        assert_eq!(replaced.len(), "a\nb\r\u{85}c\u{2029}d".len());
    }
}
