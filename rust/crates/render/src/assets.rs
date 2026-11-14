//! What drawing keeps from one frame to the next: the fonts with the lines and glyphs drawn with them, and the icons.

use crate::icon::Icons;
use crate::text::Fonts;

/// The fonts, glyphs and icons a [`Canvas`](crate::Canvas) draws with, kept by the caller between frames so that
/// each line is shaped once and each glyph and icon rasterized once.
#[derive(Debug, Default)]
pub struct Assets {
    /// The fonts loaded, their typefaces, and the lines and glyphs drawn with them.
    pub fonts: Fonts,
    /// The icons drawn so far, at each size.
    pub icons: Icons,
}

impl Assets {
    /// No fonts yet, and nothing drawn: enough for shapes and icons.
    #[must_use]
    pub fn new() -> Assets {
        Assets::default()
    }

    /// The fonts that come with the program, as [`Fonts::bundled`] loads them, and nothing drawn.
    #[must_use]
    pub fn bundled() -> Assets {
        Assets {
            fonts: Fonts::bundled(),
            icons: Icons::default(),
        }
    }
}
