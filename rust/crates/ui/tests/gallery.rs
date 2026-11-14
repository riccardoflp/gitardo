//! The gallery, shaped as the Timeline, in each theme at 100 %, in the dark and light themes at 200 %, and in narrow
//! windows where the rail shows icons and the inspector opens over the timeline, compared with its reference images:
//! `GITARDO_GOLDEN_RECORD=1 cargo test -p ui --test gallery` records them again.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::ThemeName;
use support::Window;
use ui::gallery::{self, GalleryState};

/// The size of the window of the gallery at 100 %, in logical pixels: that of the design.
const SIZE: (u32, u32) = (1280, 800);
/// The size of the window of the gallery at 200 %, in logical pixels, smaller to keep the snapshots small.
const SIZE_2X: (u32, u32) = (960, 600);

/// Draws the gallery in its fixed state in a window of a size and compares it with its snapshot.
fn check(theme: ThemeName, (size, scale): ((u32, u32), u8), state: &GalleryState, name: &str) {
    let mut window = Window::new(theme, size, scale);
    gallery::prepare(&mut window.memory, &window.theme);
    let (intents, output) = window.frame(&[], |ui| gallery::view(state, ui));
    assert!(
        intents.is_empty(),
        "a frame with no input asks for nothing: {intents:?}"
    );
    assert!(
        output.caret.is_some(),
        "the message of the draft has the focus and shows its caret"
    );
    window.assert_snapshot(name);
}

#[test]
fn gallery_dark() {
    check(ThemeName::Dark, (SIZE, 1), &GalleryState::default(), "gallery-dark");
}

#[test]
fn gallery_light() {
    check(ThemeName::Light, (SIZE, 1), &GalleryState::default(), "gallery-light");
}

#[test]
fn gallery_high_contrast_dark() {
    check(
        ThemeName::HighContrastDark,
        (SIZE, 1),
        &GalleryState::default(),
        "gallery-high-contrast-dark",
    );
}

#[test]
fn gallery_high_contrast_light() {
    check(
        ThemeName::HighContrastLight,
        (SIZE, 1),
        &GalleryState::default(),
        "gallery-high-contrast-light",
    );
}

#[test]
fn gallery_dark_at_200_percent() {
    check(
        ThemeName::Dark,
        (SIZE_2X, 2),
        &GalleryState::default(),
        "gallery-dark-2x",
    );
}

#[test]
fn gallery_light_at_200_percent() {
    check(
        ThemeName::Light,
        (SIZE_2X, 2),
        &GalleryState::default(),
        "gallery-light-2x",
    );
}

#[test]
fn a_narrow_window_shows_the_rail_as_icons() {
    check(
        ThemeName::Dark,
        ((860, 640), 1),
        &GalleryState::default(),
        "gallery-icons-dark",
    );
}

#[test]
fn a_very_narrow_window_opens_the_inspector_over_the_timeline() {
    let state = GalleryState {
        is_inspector_open: true,
        ..GalleryState::default()
    };
    check(ThemeName::Light, ((680, 600), 1), &state, "gallery-over-light");
}
