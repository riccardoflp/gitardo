//! The cache of glyphs: the coverage of each glyph drawn, by font, size, glyph and where it falls inside a pixel, so
//! that a glyph is rasterized once and then only copied.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::FontId;
use crate::flatten::flatten;
use crate::font::{Font, GlyphId};
use crate::geometry::{PixelRect, Transform};
use crate::number;
use crate::raster::{FillRule, Mask, rasterize};

/// The positions inside a pixel a glyph is rasterized at, along the line: four, a quarter of a pixel apart, which
/// spaces text evenly without rasterizing every glyph at every position.
pub(crate) const SUBPIXEL_POSITIONS: u8 = 4;

/// The most glyphs kept: past this the cache starts again, which only costs rasterizing the glyphs in sight again.
const MOST_GLYPHS: usize = 16_384;

/// A glyph at a size and a position inside a pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GlyphKey {
    /// The font.
    pub(crate) font: FontId,
    /// The glyph.
    pub(crate) glyph: GlyphId,
    /// The size in physical pixels, in 64ths of a pixel.
    pub(crate) size: u32,
    /// The position inside the pixel, in quarters of a pixel from its left side.
    pub(crate) subpixel: u8,
}

impl GlyphKey {
    /// The size in physical pixels.
    fn pixels(self) -> f32 {
        number::float_of_size(self.size) / 64.0
    }
}

/// The glyphs rasterized so far.
#[derive(Clone, Debug, Default)]
pub(crate) struct GlyphCache {
    /// The coverage of each glyph, over pixels counted from the pen on the baseline; nothing for a glyph that draws
    /// nothing.
    masks: HashMap<GlyphKey, Option<Mask>, BuildHasherDefault<KeyHasher>>,
    /// How many glyphs were rasterized since the cache was made.
    rasterized: u64,
}

impl GlyphCache {
    /// The coverage of a glyph, rasterized from its font the first time it is asked for.
    pub(crate) fn mask(&mut self, key: GlyphKey, font: &Font) -> Option<&Mask> {
        if !self.masks.contains_key(&key) {
            if self.masks.len() >= MOST_GLYPHS {
                self.masks.clear();
            }
            self.rasterized += 1;
            let _ = self.masks.insert(key, rasterize_glyph(key, font));
        }
        self.masks.get(&key).and_then(Option::as_ref)
    }

    /// How many glyphs were rasterized since the cache was made.
    pub(crate) fn rasterized(&self) -> u64 {
        self.rasterized
    }
}

/// The coverage of a glyph at its size, moved right by its position inside the pixel, over pixels counted from the
/// pen on the baseline, `y` growing downwards.
fn rasterize_glyph(key: GlyphKey, font: &Font) -> Option<Mask> {
    let outline = font.outline(key.glyph)?;
    let scale = key.pixels() / f32::from(font.units_per_em());
    let shift = f32::from(key.subpixel) / f32::from(SUBPIXEL_POSITIONS);
    let transform = Transform::translation(shift, 0.0).after(Transform::scale(scale, -scale));
    let path = outline.transformed(transform);
    let rect = PixelRect::enclosing(path.bounds()?);
    let mask = rasterize(&flatten(&path), rect, FillRule::NonZero);
    Some(mask)
}

/// A hasher for the keys of the cache: FNV-1a, Fowler, Noll and Vo's hash, as its authors describe it. The keys are
/// a few small numbers that no one chooses to collide, so a fast hash is enough.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyHasher(u64);

impl Default for KeyHasher {
    fn default() -> KeyHasher {
        KeyHasher(0xCBF2_9CE4_8422_2325)
    }
}

impl Hasher for KeyHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
