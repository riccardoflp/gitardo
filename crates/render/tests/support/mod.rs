//! What the snapshot tests of `render` share: the fonts kept for the tests, and the comparison of a frame with its
//! reference image.

#![allow(dead_code, reason = "each test file uses its own part of the support")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

use std::path::Path;

use render::{Assets, Font, FrameBuffer, TextStyle, TypefaceId};
use testkit::png::{self, Image};

/// The typefaces made of the fonts kept for the tests.
pub struct Typefaces {
    /// The sans-serif font, for the interface.
    pub sans: TypefaceId,
    /// Its bold face.
    pub bold: TypefaceId,
    /// The monospaced font, for code, falling back to the sans-serif one.
    pub mono: TypefaceId,
}

impl Typefaces {
    /// A style of a typeface at a size.
    pub fn style(typeface: TypefaceId, size: f32) -> TextStyle {
        TextStyle { typeface, size }
    }
}

/// Assets with the fonts kept for the tests loaded, and their typefaces.
pub fn assets() -> (Assets, Typefaces) {
    let mut assets = Assets::new();
    let fonts_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testkit/fonts");
    let mut load = |name: &str| {
        let bytes = std::fs::read(fonts_folder.join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assets.fonts.add_font(Font::from_bytes(bytes).unwrap()).unwrap()
    };
    let (sans, bold, mono) = (
        load("DejaVuSans.ttf"),
        load("DejaVuSans-Bold.ttf"),
        load("DejaVuSansMono.ttf"),
    );
    let typefaces = Typefaces {
        sans: assets.fonts.add_typeface(&[sans]).unwrap(),
        bold: assets.fonts.add_typeface(&[bold, sans]).unwrap(),
        mono: assets.fonts.add_typeface(&[mono, sans]).unwrap(),
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
