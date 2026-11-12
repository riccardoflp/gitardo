//! What the snapshot tests of `render` share: the fonts that come with the program, and the comparison of a frame with
//! its reference image.

#![allow(dead_code, reason = "each test file uses its own part of the support")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

use std::path::Path;

use render::{Assets, FontFamily, FontWeight, FrameBuffer, TextStyle, TypefaceId};
use testkit::png::{self, Image};

/// The typefaces made of the fonts that come with the program.
pub struct Typefaces {
    /// Inter, for the interface.
    pub sans: TypefaceId,
    /// Its bold face.
    pub bold: TypefaceId,
    /// `JetBrains Mono`, for code.
    pub mono: TypefaceId,
}

impl Typefaces {
    /// A style of a typeface at a size.
    pub fn style(typeface: TypefaceId, size: f32) -> TextStyle {
        TextStyle { typeface, size }
    }
}

/// Assets with the fonts that come with the program, and no font of the system, and their typefaces.
pub fn assets() -> (Assets, Typefaces) {
    let mut assets = Assets::bundled();
    let mut add = |family, weight| assets.fonts.add_typeface(&family, weight).unwrap();
    let typefaces = Typefaces {
        sans: add(FontFamily::Interface, FontWeight::REGULAR),
        bold: add(FontFamily::Interface, FontWeight::BOLD),
        mono: add(FontFamily::Code, FontWeight::REGULAR),
    };
    (assets, typefaces)
}

/// Checks that a frame has the pixels of its reference image, `tests/snapshots/<name>.png`, as
/// [`testkit::png::assert_snapshot`] does.
pub fn assert_snapshot(name: &str, frame: &FrameBuffer) {
    let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.png"));
    let actual = Image {
        width: frame.width(),
        height: frame.height(),
        pixels: frame.to_rgba(),
    };
    png::assert_snapshot(reference, &actual);
}
