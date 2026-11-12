//! Drawing a line of text on a canvas: each glyph copied from the cache at its place, snapped to the baseline and to
//! a quarter of a pixel along the line.

use super::cache::{GlyphKey, SUBPIXEL_POSITIONS};
use super::layout::{self, Shape};
use super::{ELLIPSIS, TextStyle};
use crate::canvas::Canvas;
use crate::color::Color;
use crate::geometry::{PixelRect, Point};
use crate::number;

/// The height of the box drawn for a character no font has, in ems.
const MISSING_HEIGHT: f32 = 0.7;

/// A line of text to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRun<'text> {
    /// The text. Line breaks in it are not drawn: a line is one line.
    pub text: &'text str,
    /// The typeface and size.
    pub style: TextStyle,
    /// The color of the glyphs.
    pub color: Color,
    /// The top left corner of the line, in logical pixels; the baseline is the ascent of the typeface below it.
    pub origin: Point,
    /// The widest the line may be, in logical pixels: a longer line is cut and ends with an ellipsis. Nothing for no
    /// limit.
    pub max_width: Option<f32>,
}

impl Canvas<'_> {
    /// Draws a line of text, cut with an ellipsis when it is wider than the run allows, and returns the width drawn,
    /// in logical pixels.
    pub fn draw_text(&mut self, run: &TextRun<'_>) -> f32 {
        let (shown, ellipsis) = match run.max_width {
            Some(width) => {
                let truncation = self.assets.fonts.truncate(run.text, run.style, width);
                (run.text.get(..truncation.shown).unwrap_or_default(), truncation.is_cut)
            }
            None => (run.text, false),
        };
        let ascent = self.assets.fonts.line_metrics(run.style).ascent;
        let origin = self.to_physical_point(run.origin);
        let baseline = number::round(origin.y + ascent * self.scale());
        let mut width = self.draw_line(shown, run, Point::new(origin.x, number::float(baseline)));
        if ellipsis {
            let at = Point::new(origin.x + width * self.scale(), number::float(baseline));
            width += self.draw_line(ELLIPSIS, run, at);
        }
        width
    }

    /// Draws the glyphs of some text with the pen starting at a point in physical pixels on the baseline, and returns
    /// their width in logical pixels.
    fn draw_line(&mut self, text: &str, run: &TextRun<'_>, pen: Point) -> f32 {
        let scale = self.scale();
        let size = number::key(run.style.size * scale * 64.0);
        let clip = self.physical_clip();
        let mut placements = Vec::with_capacity(text.len());
        let width = layout::place(&self.assets.fonts, text, run.style, |placed| placements.push(placed));
        let baseline = number::round(pen.y);
        for placed in placements {
            let x = pen.x + placed.x * scale;
            match placed.shape {
                Shape::Glyph { font, glyph } => {
                    let (column, subpixel) = split_position(x);
                    let key = GlyphKey {
                        font,
                        glyph,
                        size,
                        subpixel,
                    };
                    let fonts = &mut self.assets.fonts;
                    let Some(font_data) = fonts.faces.get(usize::from(font.0)) else {
                        continue;
                    };
                    if let Some(mask) = fonts.glyphs.mask(key, font_data) {
                        self.frame.fill_mask(mask.view_at(column, baseline), run.color, clip);
                    }
                }
                Shape::Missing => {
                    let box_width = placed.advance * scale * 0.8;
                    let rect = PixelRect::new(
                        number::round(x + placed.advance * scale * 0.1),
                        number::round(pen.y - run.style.size * scale * MISSING_HEIGHT),
                        number::round(x + placed.advance * scale * 0.1 + box_width),
                        baseline,
                    );
                    self.draw_missing_box(rect, run.color);
                }
                Shape::Blank => {}
            }
        }
        width
    }

    /// The outline of the box drawn for a character no font has, one pixel wide at each scale.
    fn draw_missing_box(&mut self, rect: PixelRect, color: Color) {
        let line = number::round(self.scale()).max(1);
        let clip = self.physical_clip();
        let sides = [
            PixelRect::new(rect.left, rect.top, rect.right, rect.top + line),
            PixelRect::new(rect.left, rect.bottom - line, rect.right, rect.bottom),
            PixelRect::new(rect.left, rect.top, rect.left + line, rect.bottom),
            PixelRect::new(rect.right - line, rect.top, rect.right, rect.bottom),
        ];
        for side in sides {
            self.frame.fill_rect(side.intersection(clip), color);
        }
    }
}

/// The pixel a position along the line falls in, and its place inside it in quarters of a pixel, rounded to the
/// nearest quarter.
fn split_position(x: f32) -> (i32, u8) {
    let column = number::floor(x);
    let quarters = number::round((x - number::float(column)) * f32::from(SUBPIXEL_POSITIONS));
    let positions = i32::from(SUBPIXEL_POSITIONS);
    let column = column + quarters / positions;
    let subpixel = u8::try_from(quarters % positions).unwrap_or(0);
    (column, subpixel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_split_into_a_pixel_and_quarters() {
        assert_eq!(split_position(10.0), (10, 0));
        assert_eq!(split_position(10.26), (10, 1));
        assert_eq!(split_position(10.9), (11, 0));
        assert_eq!(split_position(-0.25), (-1, 3));
    }
}
