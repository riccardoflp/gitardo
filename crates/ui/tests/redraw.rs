//! Drawing only what changed: a frame with nothing new draws nothing, the blink of the caret draws a tile, and a
//! frame drawn over the last one has exactly the pixels of the same frame drawn whole.

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use render::ThemeName;
use support::{Window, click, typed, wheel};
use ui::gallery::{self, GalleryState};

/// The size of the window, in logical pixels.
const SIZE: (u32, u32) = (1024, 720);

/// A window with the gallery drawn once in its fixed state.
fn gallery_window(theme: ThemeName, scale: u8) -> Window {
    let mut window = Window::new(theme, SIZE, scale);
    gallery::prepare(&mut window.memory, &window.theme);
    let state = GalleryState::default();
    let (_, output) = window.frame(&[], |ui| gallery::view(&state, ui));
    assert_eq!(output.dirty.len(), 1, "the first frame draws all of the window");
    window
}

#[test]
fn a_frame_with_nothing_new_draws_nothing() {
    let mut window = gallery_window(ThemeName::Dark, 1);
    let state = GalleryState::default();
    let (_, output) = window.frame(&[], |ui| gallery::view(&state, ui));
    assert!(output.dirty.is_empty(), "{:?}", output.dirty);
}

#[test]
fn the_blink_of_the_caret_and_the_turn_of_the_spinners_draw_only_around_them() {
    let mut window = gallery_window(ThemeName::Light, 2);
    let state = GalleryState::default();
    window.wait(600);
    let (_, output) = window.frame(&[], |ui| gallery::view(&state, ui));
    let area: u64 = output.dirty.iter().map(|region| region.area()).sum();
    // The caret and the two spinners, each in a tile of 128 pixels at 200 %, one of them on the edge of two.
    assert_eq!(output.dirty.len(), 3, "{:?}", output.dirty);
    assert!(area <= 4 * 128 * 128, "{area} pixels drawn again: {:?}", output.dirty);
    let caret = output.caret.unwrap();
    let physical = |value: f32| value * 2.0;
    assert!(output.dirty.iter().any(|region| {
        f64::from(region.left) <= f64::from(physical(caret.x)) && f64::from(physical(caret.x)) < f64::from(region.right)
    }));
}

#[test]
fn a_frame_drawn_over_the_last_has_the_pixels_of_the_same_frame_drawn_whole() {
    for (theme, scale) in [(ThemeName::Dark, 1), (ThemeName::HighContrastLight, 2)] {
        let mut incremental = gallery_window(theme, scale);
        let mut whole = Window::new(theme, SIZE, scale);
        whole.memory = incremental.memory.clone();
        whole.memory.redraw_everything();
        let state = GalleryState::default();
        let mut events = vec![typed("s"), wheel(700.0, 200.0, 3.0)];
        events.extend(click(30.0, 231.0));
        let mut drawn = Vec::new();
        for window in [&mut incremental, &mut whole] {
            window.wait(50);
            let (_, output) = window.frame(&events, |ui| gallery::view(&state, ui));
            drawn.push(output.dirty);
        }
        assert_eq!(incremental.frame.pixels(), whole.frame.pixels(), "{theme:?} at {scale}");
        let parts: u64 = drawn.first().unwrap().iter().map(|region| region.area()).sum();
        let all = u64::from(incremental.frame.width()) * u64::from(incremental.frame.height());
        assert!(
            parts < all / 2,
            "only the parts that changed are drawn: {:?}",
            drawn.first()
        );
        whole.memory.redraw_everything();
        for window in [&mut incremental, &mut whole] {
            window.wait(50);
            let _ = window.frame(&[], |ui| gallery::view(&state, ui));
        }
        assert_eq!(
            incremental.frame.pixels(),
            whole.frame.pixels(),
            "{theme:?} at {scale}, as the list glides"
        );
    }
}
