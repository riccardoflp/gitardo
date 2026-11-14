//! Drawing a line of text on a canvas: each glyph rasterized once at its size and its position inside a pixel, a
//! quarter of a pixel apart along the line, and then laid over the frame at its place, the baseline on a whole pixel.

use cosmic_text::SwashContent;

use super::engine::{Engine, Face};
use super::{ELLIPSIS, TextStyle};
use crate::canvas::Canvas;
use crate::color::Color;
use crate::frame::MaskView;
use crate::geometry::{PixelRect, Point};
use crate::number;

/// A line of text to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextRun<'text> {
    /// The text. Line breaks in it are not drawn: a line is one line.
    pub text: &'text str,
    /// The typeface and size.
    pub style: TextStyle,
    /// The color of the glyphs; glyphs in color, such as emoji, keep their own.
    pub color: Color,
    /// The top left corner of the line, in logical pixels; the baseline is the ascent of the typeface below it.
    pub origin: Point,
    /// The widest the line may be, in logical pixels: a longer line is cut and ends with an ellipsis. Nothing for no
    /// limit.
    pub max_width: Option<f32>,
}

/// A line of text to turn into images of glyphs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphRun<'text> {
    /// The text.
    pub(crate) text: &'text str,
    /// The typeface and size.
    pub(crate) style: TextStyle,
    /// Where the line starts on the baseline, in physical pixels of the frame.
    pub(crate) pen: Point,
    /// Physical pixels for each logical pixel.
    pub(crate) scale: f32,
}

/// What a glyph covers.
#[derive(Clone, Copy, Debug)]
pub(crate) enum GlyphPixels<'image> {
    /// How much of each pixel the glyph covers, a byte for each, to fill in the color of the text.
    Coverage(&'image [u8]),
    /// The colors of a glyph in color, four bytes for each pixel: red, green, blue and opacity.
    Colors(&'image [u8]),
}

/// The image of a glyph at its place in the frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphImage<'image> {
    /// The pixels of the frame it covers.
    pub(crate) rect: PixelRect,
    /// What it covers them with.
    pub(crate) pixels: GlyphPixels<'image>,
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
        let baseline = number::float(number::round(origin.y + ascent * self.scale()));
        let mut width = self.draw_line(shown, run, Point::new(origin.x, baseline));
        if ellipsis {
            let at = Point::new(origin.x + width * self.scale(), baseline);
            width += self.draw_line(ELLIPSIS, run, at);
        }
        width
    }

    /// Draws the glyphs of some text with the pen starting at a point in physical pixels on the baseline, and returns
    /// their width in logical pixels.
    fn draw_line(&mut self, text: &str, run: &TextRun<'_>, pen: Point) -> f32 {
        let glyphs = GlyphRun {
            text,
            style: run.style,
            pen,
            scale: self.scale(),
        };
        let clip = self.physical_clip();
        let frame = &mut *self.frame;
        self.assets.fonts.glyph_images(&glyphs, |image| match image.pixels {
            GlyphPixels::Coverage(coverage) => {
                let mask = MaskView {
                    rect: image.rect,
                    coverage,
                };
                frame.fill_mask(mask, run.color, clip);
            }
            GlyphPixels::Colors(colors) => frame.fill_colors(image.rect, colors, clip),
        })
    }
}

/// Calls `visit` with the image of each glyph of a line of text in a face, and returns the width of the line in
/// logical pixels. The typeface's number keeps the lines it shaped apart from those of other typefaces.
pub(crate) fn glyph_images(
    engine: &mut Engine,
    face: &Face,
    (typeface, run): (u16, &GlyphRun<'_>),
    mut visit: impl FnMut(GlyphImage<'_>),
) -> f32 {
    let key = (typeface, run.style.size.to_bits());
    let Some(shaped) = engine.shaped(key, face, run.text, run.style.size) else {
        return 0.0;
    };
    let width = shaped.width;
    let placed: Vec<_> = shaped
        .glyphs
        .iter()
        .map(|glyph| glyph.physical((run.pen.x, run.pen.y), run.scale))
        .collect();
    for glyph in placed {
        let Some(image) = engine.glyph(glyph.cache_key) else {
            continue;
        };
        let pixels = match image.content {
            SwashContent::Mask => GlyphPixels::Coverage(&image.data),
            SwashContent::Color => GlyphPixels::Colors(&image.data),
            // Glyphs are asked for in shades of gray, never with a coverage for each color of a pixel.
            SwashContent::SubpixelMask => continue,
        };
        let left = glyph.x + image.placement.left;
        let top = glyph.y - image.placement.top;
        let rect = PixelRect::new(
            left,
            top,
            left + number::coordinate(image.placement.width),
            top + number::coordinate(image.placement.height),
        );
        visit(GlyphImage { rect, pixels });
    }
    width
}
