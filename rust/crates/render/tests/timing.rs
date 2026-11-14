//! How long frames take to draw, in a release build: `cargo test -p render --release --test timing -- --ignored
//! --nocapture`. Not pass or fail but numbers to read, except that glyphs must come from the cache after the first
//! frame.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use std::time::{Duration, Instant};

use render::{Assets, Canvas, Color, FrameBuffer, Icon, Point, Rect, TextRun};
use support::{Typefaces, assets};

/// The frames drawn after the first, to average.
const FRAMES: u32 = 30;

/// Sixty rows of about eighty characters from a first row on, as a screen of the graph shows them.
fn rows(first: usize) -> Vec<String> {
    (first..first + 60)
        .map(|row| format!("{row:>3} 3f2a1c9 Fix the parser of the configuration file when a section has no name main"))
        .collect()
}

/// Draws one frame: a clear, then for each row its background, five badges and lane rectangles, an icon and its text,
/// which makes 361 rectangles and 4,800 glyphs.
fn draw_frame(frame: &mut FrameBuffer, assets: &mut Assets, (typefaces, scale): (&Typefaces, f32), rows: &[String]) {
    let mut canvas = Canvas::new(frame, assets, scale);
    canvas.clear(Color::rgb(0x20, 0x23, 0x28));
    let style = Typefaces::style(typefaces.sans, 13.0);
    let mut y = 0.0;
    for (index, text) in rows.iter().enumerate() {
        if index % 2 == 0 {
            canvas.fill_rect(Rect::new(0.0, y, 1920.0, 18.0), Color::rgb(0x26, 0x2A, 0x30));
        }
        let mut x = 1400.0;
        for _ in 0..5 {
            canvas.fill_rect(Rect::new(x, y + 2.0, 40.0, 14.0), Color::rgba(0x5A, 0x9C, 0xF0, 200));
            x += 46.0;
        }
        canvas.draw_icon(
            Icon::Branch,
            Point::new(4.0, y + 1.0),
            16.0,
            Color::rgb(0xDD, 0xE1, 0xE6),
        );
        let run = TextRun {
            text,
            style,
            color: Color::rgb(0xDD, 0xE1, 0xE6),
            origin: Point::new(24.0, y + 1.0),
            max_width: Some(1300.0),
        };
        let _ = canvas.draw_text(&run);
        y += 18.0;
    }
}

/// Times the first frame and the average of the frames after it, at a whole scale, on a frame of 1920 × 1080 logical
/// pixels.
fn time_frames(scale: u8) -> (Duration, Duration, u64) {
    let (mut assets, typefaces) = assets();
    let mut frame = FrameBuffer::new(1920 * u32::from(scale), 1080 * u32::from(scale));
    let scale = f32::from(scale);
    let rows = rows(0);
    let start = Instant::now();
    draw_frame(&mut frame, &mut assets, (&typefaces, scale), &rows);
    let first = start.elapsed();
    let rasterized = assets.fonts.rasterized_glyph_count();
    let start = Instant::now();
    for _ in 0..FRAMES {
        draw_frame(&mut frame, &mut assets, (&typefaces, scale), &rows);
    }
    let average = start.elapsed() / FRAMES;
    (first, average, assets.fonts.rasterized_glyph_count() - rasterized)
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn a_full_frame_of_text_and_rectangles() {
    for scale in [1, 2] {
        let (first, average, rasterized_after_first) = time_frames(scale);
        println!(
            "1920 x 1080 logical pixels at scale {scale}: first frame {:.2} ms, then {:.2} ms a frame; glyphs rasterized after the first frame: {rasterized_after_first}",
            first.as_secs_f64() * 1000.0,
            average.as_secs_f64() * 1000.0,
        );
        assert_eq!(rasterized_after_first, 0);
    }
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn frames_of_rows_never_drawn_before() {
    let (mut assets, typefaces) = assets();
    let mut frame = FrameBuffer::new(1920, 1080);
    draw_frame(&mut frame, &mut assets, (&typefaces, 1.0), &rows(0));
    let screens: Vec<Vec<String>> = (1..=FRAMES)
        .map(|screen| rows(60 * usize::try_from(screen).unwrap()))
        .collect();
    let start = Instant::now();
    for screen in &screens {
        draw_frame(&mut frame, &mut assets, (&typefaces, 1.0), screen);
    }
    println!(
        "1920 x 1080 at scale 1, every row new, as when scrolling a page at a time: {:.2} ms a frame",
        (start.elapsed() / FRAMES).as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn a_clear_of_a_full_frame() {
    let mut frame = FrameBuffer::new(1920, 1080);
    let mut assets = Assets::new();
    let start = Instant::now();
    for _ in 0..FRAMES {
        let mut canvas = Canvas::new(&mut frame, &mut assets, 1.0);
        canvas.clear(Color::rgb(0x20, 0x23, 0x28));
    }
    println!(
        "clear of 1920 x 1080: {:.3} ms",
        (start.elapsed() / FRAMES).as_secs_f64() * 1000.0
    );
}
