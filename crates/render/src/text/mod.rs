//! Text: the fonts loaded, typefaces of a font and its fallbacks, and lines of text measured, cut with an ellipsis
//! and wrapped by words. Drawing them is [`Canvas::draw_text`](crate::Canvas::draw_text).
//!
//! Sizes and widths are in logical pixels: the scale of the screen is applied only when drawing, so that a line
//! measures the same at every scale (D4).

mod cache;
mod draw;
mod layout;
mod wrap;

use crate::error::Error;
use crate::font::Font;
use cache::GlyphCache;
pub use draw::TextRun;

/// A font loaded into [`Fonts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontId(u16);

/// A typeface: a font and the fonts to look in, in order, for a character it lacks.
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

/// The fonts loaded, the typefaces made of them, and the glyphs drawn with them so far.
#[derive(Clone, Debug, Default)]
pub struct Fonts {
    /// The fonts, by their [`FontId`].
    faces: Vec<Font>,
    /// The fonts of each typeface, by its [`TypefaceId`].
    typefaces: Vec<Vec<FontId>>,
    /// The glyphs already rasterized.
    glyphs: GlyphCache,
}

impl Fonts {
    /// No fonts yet.
    #[must_use]
    pub fn new() -> Fonts {
        Fonts::default()
    }

    /// Loads a font.
    ///
    /// # Errors
    ///
    /// When more fonts are loaded than a [`FontId`] can count.
    pub fn add_font(&mut self, font: Font) -> Result<FontId, Error> {
        let id = FontId(u16::try_from(self.faces.len()).map_err(|_| Error::TooManyFonts)?);
        self.faces.push(font);
        Ok(id)
    }

    /// Makes a typeface of fonts already loaded: the first draws what it has, each of the others what the ones before
    /// lack. A character none of them has is drawn as a box.
    ///
    /// # Errors
    ///
    /// When more typefaces are made than a [`TypefaceId`] can count.
    pub fn add_typeface(&mut self, fonts: &[FontId]) -> Result<TypefaceId, Error> {
        let id = TypefaceId(u16::try_from(self.typefaces.len()).map_err(|_| Error::TooManyFonts)?);
        let known = fonts
            .iter()
            .copied()
            .filter(|font| usize::from(font.0) < self.faces.len())
            .collect();
        self.typefaces.push(known);
        Ok(id)
    }

    /// A font loaded, by its identifier.
    #[must_use]
    pub fn font(&self, id: FontId) -> Option<&Font> {
        self.faces.get(usize::from(id.0))
    }

    /// The heights of a line of text in a style, from the first font of its typeface.
    #[must_use]
    pub fn line_metrics(&self, style: TextStyle) -> LineMetrics {
        let Some(font) = self.primary_font(style.typeface) else {
            return LineMetrics {
                ascent: style.size * 0.8,
                descent: style.size * 0.2,
                height: style.size,
            };
        };
        let metrics = font.vertical_metrics();
        let unit = style.size / f32::from(font.units_per_em());
        let ascent = f32::from(metrics.ascender) * unit;
        let descent = -f32::from(metrics.descender) * unit;
        LineMetrics {
            ascent,
            descent,
            height: ascent + descent + f32::from(metrics.line_gap.max(0)) * unit,
        }
    }

    /// The width of a line of text in a style, in logical pixels: the advances of its glyphs with their kerning.
    #[must_use]
    pub fn measure(&self, text: &str, style: TextStyle) -> f32 {
        layout::place(self, text, style, |_| {})
    }

    /// Where a line of text is cut so that it and an ellipsis after it fit a width; the whole line when it fits. The
    /// spaces before the cut are left out, and nothing but the ellipsis is shown when not even one character fits.
    #[must_use]
    pub fn truncate(&self, text: &str, style: TextStyle, width: f32) -> Truncation {
        let mut ends: Vec<(usize, f32)> = Vec::new();
        let whole = layout::place(self, text, style, |placed| {
            ends.push((placed.end, placed.x + placed.advance));
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
    pub fn wrap(&self, text: &str, style: TextStyle, width: f32) -> Vec<std::ops::Range<usize>> {
        wrap::wrap(self, text, style, width)
    }

    /// How many glyphs were rasterized since these fonts were loaded: once a frame has been drawn, drawing it again
    /// rasterizes none.
    #[must_use]
    pub fn rasterized_glyph_count(&self) -> u64 {
        self.glyphs.rasterized()
    }

    /// The first font of a typeface.
    fn primary_font(&self, typeface: TypefaceId) -> Option<&Font> {
        let first = self.typefaces.get(usize::from(typeface.0))?.first()?;
        self.font(*first)
    }

    /// The fonts of a typeface, in order.
    fn typeface(&self, typeface: TypefaceId) -> &[FontId] {
        self.typefaces.get(usize::from(typeface.0)).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests;
