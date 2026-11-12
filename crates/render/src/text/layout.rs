//! Placing the glyphs of a line of text: one glyph for each character, from the first font of the typeface that has
//! it, each after the advance of the one before and its kerning.

use super::{FontId, Fonts, TextStyle};
use crate::font::GlyphId;

/// How many spaces a tab is as wide as.
const TAB_WIDTH: f32 = 4.0;
/// The width of the box drawn for a character no font has, in ems.
const MISSING_WIDTH: f32 = 0.6;

/// What is drawn for a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A glyph of a font.
    Glyph {
        /// The font.
        font: FontId,
        /// The glyph.
        glyph: GlyphId,
    },
    /// A box, for a character no font of the typeface has.
    Missing,
    /// Nothing, for a character that only moves the pen, such as a tab.
    Blank,
}

/// A character placed on the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    /// What is drawn.
    pub(crate) shape: Shape,
    /// Where it starts, in logical pixels from the start of the line.
    pub(crate) x: f32,
    /// How far the pen moves after it, kerning included.
    pub(crate) advance: f32,
    /// The byte of the text where its character starts.
    pub(crate) start: usize,
    /// The byte after its character.
    pub(crate) end: usize,
    /// Whether its character is white space, where a line may break.
    pub(crate) is_space: bool,
}

/// Places every character of a line, giving each to `visit` in order, and returns the width of the line. Line breaks
/// and other control characters take no room.
pub(crate) fn place(fonts: &Fonts, text: &str, style: TextStyle, mut visit: impl FnMut(Placed)) -> f32 {
    let typeface = fonts.typeface(style.typeface);
    let mut x = 0.0_f32;
    let mut before: Option<(FontId, GlyphId)> = None;
    for (start, character) in text.char_indices() {
        let end = start + character.len_utf8();
        let (shape, advance) = shape_of(fonts, typeface, character, style.size);
        // Kerning moves a glyph closer to the one before, when both come from the same font.
        if let (Some((font_before, glyph_before)), Shape::Glyph { font, glyph }) = (before, shape)
            && font_before == font
        {
            x += kerning_between(fonts, font, (glyph_before, glyph), style.size);
        }
        visit(Placed {
            shape,
            x,
            advance,
            start,
            end,
            is_space: character.is_whitespace(),
        });
        before = match shape {
            Shape::Glyph { font, glyph } => Some((font, glyph)),
            Shape::Missing | Shape::Blank => None,
        };
        x += advance;
    }
    x
}

/// What is drawn for a character, and its advance in logical pixels.
fn shape_of(fonts: &Fonts, typeface: &[FontId], character: char, size: f32) -> (Shape, f32) {
    if character == '\t' {
        let space = typeface
            .first()
            .and_then(|id| advance_of(fonts, *id, ' ', size))
            .unwrap_or(size * 0.3);
        return (Shape::Blank, space * TAB_WIDTH);
    }
    if character.is_control() {
        return (Shape::Blank, 0.0);
    }
    for id in typeface {
        let Some(font) = fonts.font(*id) else {
            continue;
        };
        if let Some(glyph) = font.glyph_id(character) {
            let advance = f32::from(font.advance(glyph)) * size / f32::from(font.units_per_em());
            return (Shape::Glyph { font: *id, glyph }, advance);
        }
    }
    (Shape::Missing, size * MISSING_WIDTH)
}

/// The advance of a character in a font, when the font has it.
fn advance_of(fonts: &Fonts, id: FontId, character: char, size: f32) -> Option<f32> {
    let font = fonts.font(id)?;
    let glyph = font.glyph_id(character)?;
    Some(f32::from(font.advance(glyph)) * size / f32::from(font.units_per_em()))
}

/// The kerning between two glyphs of a font, in logical pixels.
fn kerning_between(fonts: &Fonts, id: FontId, (first, second): (GlyphId, GlyphId), size: f32) -> f32 {
    fonts.font(id).map_or(0.0, |font| {
        f32::from(font.kerning(first, second)) * size / f32::from(font.units_per_em())
    })
}
