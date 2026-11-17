//! What drawing keeps from one frame to the next: the fonts with the lines and glyphs drawn with them, and the icons.

use crate::file_icon::FileIcons;
use crate::icon::Icons;
use crate::text::Fonts;

/// The fonts, glyphs, icons and icons of files a [`Canvas`](crate::Canvas) draws with, kept by the caller between frames so that
/// each line is shaped once and each glyph and icon rasterized once.
#[derive(Debug, Default)]
pub struct Assets {
    /// The fonts loaded, their typefaces, and the lines and glyphs drawn with them.
    pub fonts: Fonts,
    /// The icons drawn so far, at each size.
    pub icons: Icons,
    /// The icons of files and folders drawn so far, at each size and in each colors, and the theme that names them.
    pub file_icons: FileIcons,
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
            file_icons: FileIcons::default(),
        }
    }
}
