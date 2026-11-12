//! The gallery of every widget, in each theme at 100 %, and in the dark and light themes at 200 %, compared with its
//! reference images: `GITARDO_GOLDEN_RECORD=1 cargo test -p ui --test gallery` records them again.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::ThemeName;
use support::Window;
use ui::gallery::{self, GalleryState};

/// The size of the window of the gallery, in logical pixels.
const SIZE: (u32, u32) = (1024, 720);

/// Draws the gallery in its fixed state and compares it with its snapshot.
fn check(theme: ThemeName, scale: u8, name: &str) {
    let mut window = Window::new(theme, SIZE, scale);
    gallery::prepare(&mut window.memory, &window.theme);
    let state = GalleryState::default();
    let (intents, output) = window.frame(&[], |ui| gallery::view(&state, ui));
    assert!(
        intents.is_empty(),
        "a frame with no input asks for nothing: {intents:?}"
    );
    assert!(output.caret.is_some(), "the input box with the focus shows its caret");
    window.assert_snapshot(name);
}

#[test]
fn gallery_dark() {
    check(ThemeName::Dark, 1, "gallery-dark");
}

#[test]
fn gallery_light() {
    check(ThemeName::Light, 1, "gallery-light");
}

#[test]
fn gallery_high_contrast_dark() {
    check(ThemeName::HighContrastDark, 1, "gallery-high-contrast-dark");
}

#[test]
fn gallery_high_contrast_light() {
    check(ThemeName::HighContrastLight, 1, "gallery-high-contrast-light");
}

#[test]
fn gallery_dark_at_200_percent() {
    check(ThemeName::Dark, 2, "gallery-dark-2x");
}

#[test]
fn gallery_light_at_200_percent() {
    check(ThemeName::Light, 2, "gallery-light-2x");
}
