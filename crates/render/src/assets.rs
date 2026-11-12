//! What drawing keeps from one frame to the next: the fonts with the glyphs drawn with them, and the icons.

use crate::icon::Icons;
use crate::text::Fonts;

/// The fonts, glyphs and icons a [`Canvas`](crate::Canvas) draws with, kept by the caller between frames so that
/// each glyph and icon is rasterized once.
#[derive(Clone, Debug, Default)]
pub struct Assets {
    /// The fonts loaded, their typefaces, and the glyphs drawn with them.
    pub fonts: Fonts,
    /// The icons drawn so far, at each size.
    pub icons: Icons,
}

impl Assets {
    /// No fonts yet, and nothing drawn.
    #[must_use]
    pub fn new() -> Assets {
        Assets::default()
    }
}
