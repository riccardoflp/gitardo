//! Text: the fonts loaded, the typefaces chosen from them, and lines of text measured, cut with an ellipsis and
//! wrapped by words. Drawing them is [`Canvas::draw_text`](crate::Canvas::draw_text).
//!
//! `cosmic-text` shapes the text, finds a font for each character and rasterizes the glyphs; none of its types leaves
//! this module. Sizes and widths are in logical pixels: the scale of the screen is applied only when drawing, so that a
//! line measures the same at every scale.

mod bundled;
mod draw;
mod engine;

use std::cell::RefCell;
use std::fmt;
use std::ops::Range;

use crate::error::Error;
pub use draw::TextRun;
pub(crate) use draw::{GlyphImage, GlyphRun};
use engine::{Engine, Face};

pub(crate) use bundled::{INTERFACE_FAMILY, database as bundled_fonts};

/// A family of fonts text can be drawn in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// Inter, the font of the interface, which comes with the program.
    Interface,
    /// `JetBrains Mono`, the font of hashes and code, which comes with the program.
    Code,
    /// A family of fonts installed on the system, by its name, such as `Segoe UI`, once
    /// [`Fonts::load_system_fonts`] has read them.
    Named(String),
}

/// How heavy the strokes of a typeface are, from 100 (thin) to 900 (black).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// The weight of running text.
    pub const REGULAR: FontWeight = FontWeight(400);
    /// A little heavier than running text.
    pub const MEDIUM: FontWeight = FontWeight(500);
    /// Between medium and bold.
    pub const SEMIBOLD: FontWeight = FontWeight(600);
    /// Bold.
    pub const BOLD: FontWeight = FontWeight(700);
}

/// A typeface made by [`Fonts::add_typeface`]: a family at a weight. A character its family lacks is drawn from
/// another font loaded that has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypefaceId(u16);

/// How text is drawn: its typeface and its size, the height of the em square in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// The typeface.
    pub typeface: TypefaceId,
    /// The size, in logical pixels.
    pub size: f32,
}

/// The heights of a line of text, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMetrics {
    /// From the top of the line to the baseline.
    pub ascent: f32,
    /// From the baseline to the bottom of the line.
    pub descent: f32,
    /// From the top of a line to the top of the next one.
    pub height: f32,
}

/// Where a line of text is cut to fit a width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Truncation {
    /// The bytes of the text that are shown, from the start.
    pub shown: usize,
    /// Whether an ellipsis follows them, because the text was cut.
    pub is_cut: bool,
    /// The width of what is drawn, the ellipsis included, in logical pixels.
    pub width: f32,
}

/// The character drawn after text that was cut.
pub(crate) const ELLIPSIS: &str = "\u{2026}";

/// A typeface: the face looked for, and the heights of its lines as fractions of the size.
#[derive(Clone, Debug)]
struct Typeface {
    /// The family and weight.
    face: Face,
    /// From the top of the line to the baseline.
    ascent: f32,
    /// From the baseline to the bottom of the line.
    descent: f32,
    /// The space between the bottom of a line and the top of the next.
    gap: f32,
}

/// The fonts loaded, the typefaces made of them, and the lines shaped and glyphs rasterized with them so far.
///
/// Measuring takes `&self`, so that a widget can measure while it lays out; the lines shaped while measuring are kept
/// inside, behind a cell that only one call at a time uses.
pub struct Fonts {
    /// The font system and what it keeps.
    engine: RefCell<Engine>,
    /// The typefaces, by their [`TypefaceId`].
    typefaces: Vec<Typeface>,
}

impl Default for Fonts {
    fn default() -> Fonts {
        Fonts::new()
    }
}

impl fmt::Debug for Fonts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Fonts")
            .field("typefaces", &self.typefaces)
            .finish_non_exhaustive()
    }
}

impl Fonts {
    /// No fonts at all: text measures nothing and draws nothing until fonts are loaded.
    #[must_use]
    pub fn new() -> Fonts {
        Fonts::over(cosmic_text::fontdb::Database::new())
    }

    /// The fonts that come with the program, Inter and `JetBrains Mono`, and no font of the system: what the tests draw
    /// with, so that their pictures are the same on every machine. Nothing is read from the disk.
    #[must_use]
    pub fn bundled() -> Fonts {
        Fonts::over(bundled::database())
    }

    /// Adds the fonts installed on the system to those loaded, so that a character the fonts of the program lack, such
    /// as an emoji or a Chinese character, is drawn from a font of the system that has it, and so that a
    /// [`FontFamily::Named`] can name one of them.
    ///
    /// This reads the folders of fonts of the system (on Windows the fonts folder of Windows and of the user, on macOS
    /// the `Library/Fonts` folders, on Linux the folders of `/usr/share/fonts` and of the user) and the header of every
    /// font file in them, which takes a moment: call it once, when the program starts, never while drawing.
    pub fn load_system_fonts(&mut self) {
        self.engine.get_mut().load_system_fonts();
    }

    /// The names of the families of the fonts loaded, the program's and, once they are loaded, the system's, in the
    /// order of the alphabet: what the menu of the fonts of the settings page lists.
    #[must_use]
    pub fn families(&self) -> Vec<String> {
        self.engine.borrow_mut().families()
    }

    /// Makes a typeface of a family at a weight; the face nearest that weight is used when the family has none of it.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownFamily`] when no font loaded belongs to the family, and [`Error::TooManyTypefaces`] when more
    /// typefaces are made than a [`TypefaceId`] can count.
    pub fn add_typeface(&mut self, family: &FontFamily, weight: FontWeight) -> Result<TypefaceId, Error> {
        let id = TypefaceId(u16::try_from(self.typefaces.len()).map_err(|_| Error::TooManyTypefaces)?);
        let name = match family {
            FontFamily::Interface => bundled::INTERFACE_FAMILY,
            FontFamily::Code => bundled::CODE_FAMILY,
            FontFamily::Named(name) => name,
        };
        let face = Face {
            family: name.to_owned(),
            weight: weight.0,
        };
        let (ascent, descent, gap) = self
            .engine
            .get_mut()
            .face_heights(&face)
            .ok_or_else(|| Error::UnknownFamily { name: name.to_owned() })?;
        self.typefaces.push(Typeface {
            face,
            ascent,
            descent,
            gap,
        });
        Ok(id)
    }

    /// The heights of a line of text in a style, from the first font of its typeface.
    #[must_use]
    pub fn line_metrics(&self, style: TextStyle) -> LineMetrics {
        let Some(typeface) = self.typeface(style.typeface) else {
            return LineMetrics {
                ascent: style.size * 0.8,
                descent: style.size * 0.2,
                height: style.size,
            };
        };
        let ascent = typeface.ascent * style.size;
        let descent = typeface.descent * style.size;
        LineMetrics {
            ascent,
            descent,
            height: ascent + descent + typeface.gap * style.size,
        }
    }

    /// The width of a line of text in a style, in logical pixels: the advances of its glyphs as shaped, kerning
    /// included.
    #[must_use]
    pub fn measure(&self, text: &str, style: TextStyle) -> f32 {
        self.with_shaped(text, style, 0.0, |shaped| shaped.width)
    }

    /// Where a line of text is cut so that it and an ellipsis after it fit a width; the whole line when it fits. The
    /// spaces before the cut are left out, and nothing but the ellipsis is shown when not even one character fits.
    #[must_use]
    pub fn truncate(&self, text: &str, style: TextStyle, width: f32) -> Truncation {
        let (whole, ends) = self.with_shaped(text, style, (0.0, Vec::new()), |shaped| {
            (shaped.width, cluster_ends(&shaped.glyphs))
        });
        if whole <= width {
            return Truncation {
                shown: text.len(),
                is_cut: false,
                width: whole,
            };
        }
        let ellipsis = self.measure(ELLIPSIS, style);
        let fitting = ends.iter().rev().find(|(_, end_x)| end_x + ellipsis <= width);
        let shown = fitting.map_or(0, |(end, _)| text.get(..*end).map_or(0, |kept| kept.trim_end().len()));
        Truncation {
            shown,
            is_cut: true,
            width: self.measure(text.get(..shown).unwrap_or_default(), style) + ellipsis,
        }
    }

    /// The lines of a text wrapped to a width by words, as ranges of its bytes: a line ends before the spaces where it
    /// breaks, a line break in the text always ends one, and a word longer than the width is cut where it overflows.
    #[must_use]
    pub fn wrap(&self, text: &str, style: TextStyle, width: f32) -> Vec<Range<usize>> {
        let mut lines = Vec::new();
        let mut paragraph_start = 0;
        for paragraph in text.split('\n') {
            let wrapped = self.with_shaped(paragraph, style, Vec::new(), |shaped| {
                shaped.wrapped(style.size, width.max(0.0))
            });
            let before = lines.len();
            for glyphs in wrapped {
                if let Some(range) = line_range(paragraph, &glyphs) {
                    lines.push(paragraph_start + range.start..paragraph_start + range.end);
                }
            }
            if lines.len() == before {
                lines.push(paragraph_start..paragraph_start);
            }
            paragraph_start += paragraph.len() + 1;
        }
        lines
    }

    /// How many glyphs were rasterized since these fonts were made: once a frame has been drawn, drawing it again
    /// rasterizes none.
    #[must_use]
    pub fn rasterized_glyph_count(&self) -> u64 {
        self.engine.try_borrow().map_or(0, |engine| engine.rasterized())
    }

    /// Calls `visit` with the image of each glyph of a line of text and where it goes in the frame, and returns the
    /// width of the line in logical pixels.
    pub(crate) fn glyph_images(&mut self, run: &GlyphRun<'_>, visit: impl FnMut(GlyphImage<'_>)) -> f32 {
        let Some(typeface) = self.typefaces.get(usize::from(run.style.typeface.0)) else {
            return 0.0;
        };
        draw::glyph_images(
            self.engine.get_mut(),
            &typeface.face,
            (run.style.typeface.0, run),
            visit,
        )
    }

    /// Fonts over a database of fonts.
    fn over(database: cosmic_text::fontdb::Database) -> Fonts {
        Fonts {
            engine: RefCell::new(Engine::new(database)),
            typefaces: Vec::new(),
        }
    }

    /// A typeface, by its identifier.
    fn typeface(&self, id: TypefaceId) -> Option<&Typeface> {
        self.typefaces.get(usize::from(id.0))
    }

    /// What `read` makes of a line of text shaped in a style, or `fallback` when the style has no typeface.
    fn with_shaped<R>(&self, text: &str, style: TextStyle, fallback: R, read: impl FnOnce(&engine::Shaped) -> R) -> R {
        let Some(typeface) = self.typeface(style.typeface) else {
            return fallback;
        };
        // Nothing keeps the engine borrowed across a call of these fonts, so it is always free here; were it not, the
        // text would measure as nothing for this one call rather than stop the program.
        let Ok(mut engine) = self.engine.try_borrow_mut() else {
            return fallback;
        };
        let key = (style.typeface.0, style.size.to_bits());
        engine
            .shaped(key, &typeface.face, text, style.size)
            .map_or(fallback, read)
    }
}

/// The byte where each cluster of glyphs ends, from the first of the text, with the width of the text up to there.
fn cluster_ends(glyphs: &[cosmic_text::LayoutGlyph]) -> Vec<(usize, f32)> {
    let mut by_start: Vec<(usize, usize, f32)> = glyphs.iter().map(|glyph| (glyph.start, glyph.end, glyph.w)).collect();
    by_start.sort_by_key(|(start, end, _)| (*start, *end));
    let mut ends: Vec<(usize, f32)> = Vec::with_capacity(by_start.len());
    let mut width = 0.0;
    for (_, end, advance) in by_start {
        width += advance;
        match ends.last_mut() {
            // Several glyphs of one cluster, such as a letter and its accent, end at the same byte.
            Some(last) if last.0 == end => last.1 = width,
            _ => ends.push((end, width)),
        }
    }
    ends
}

/// The bytes of a paragraph on one of its wrapped lines, the spaces at its end left out; nothing for a line without
/// glyphs.
fn line_range(paragraph: &str, glyphs: &[cosmic_text::LayoutGlyph]) -> Option<Range<usize>> {
    let start = glyphs.iter().map(|glyph| glyph.start).min()?;
    let end = glyphs.iter().map(|glyph| glyph.end).max()?;
    let kept = paragraph.get(start..end).map_or(0, |line| line.trim_end().len());
    Some(start..start + kept)
}

#[cfg(test)]
mod tests;
