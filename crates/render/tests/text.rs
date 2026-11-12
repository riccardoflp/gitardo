//! Snapshots of text in each of the fonts kept for the tests, at scale 1 and 2: kerning, accents and other scripts,
//! a line cut with an ellipsis, a character no font has, and a paragraph wrapped by words.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use render::{Canvas, Color, FrameBuffer, Point, TextRun, TypefaceId};
use support::{Typefaces, assert_snapshot, assets};

/// The logical width of each picture of text.
const WIDTH: u32 = 380;
/// The logical height of each picture of text.
const HEIGHT: u32 = 150;

/// The lines drawn whole, one under the other.
const LINES: [&str; 3] = [
    "The quick brown fox jumps over the lazy dog 0123456789",
    "AVA Wave To Yo: kerning, \u{201C}quotes\u{201D} & (brackets) [x] {y} @#%",
    "\u{395}\u{3BB}\u{3BB}\u{3B7}\u{3BD}\u{3B9}\u{3BA}\u{3AC}, \u{41A}\u{438}\u{440}\u{438}\u{43B}\u{43B}\u{438}\u{446}\u{430}, \u{DC}n\u{EF}c\u{F6}d\u{E9} \u{DF} \u{F8} \u{142}",
];

/// Draws the picture of text in a typeface at a whole scale.
fn draw_text(choose: fn(&Typefaces) -> TypefaceId, scale: u8) -> FrameBuffer {
    let (mut assets, typefaces) = assets();
    let typeface = choose(&typefaces);
    let style = Typefaces::style(typeface, 13.0);
    let mut frame = FrameBuffer::new(WIDTH * u32::from(scale), HEIGHT * u32::from(scale));
    let mut canvas = Canvas::new(&mut frame, &mut assets, f32::from(scale));
    canvas.clear(Color::rgb(0x20, 0x23, 0x28));
    let foreground = Color::rgb(0xDD, 0xE1, 0xE6);
    let run = |text, y, max_width| TextRun {
        text,
        style,
        color: foreground,
        origin: Point::new(8.0, y),
        max_width,
    };
    let mut y = 6.0;
    for line in LINES {
        let _ = canvas.draw_text(&run(line, y, None));
        y += 18.0;
    }
    let long = "src/components/branches/the list of branches.rs was changed in this commit";
    let _ = canvas.draw_text(&run(long, y, Some(240.0)));
    let missing = "A box for \u{E000}, which no font has";
    let _ = canvas.draw_text(&TextRun {
        color: Color::rgb(0xE0, 0xB0, 0x50),
        ..run(missing, y + 18.0, None)
    });
    let paragraph = "Gitardo runs the real git: what it does can be done again in a terminal, and what is done in a terminal shows in it at once.";
    let height = canvas.assets().fonts.line_metrics(style).height;
    let lines = canvas.assets().fonts.wrap(paragraph, style, 300.0);
    for (index, range) in lines.into_iter().enumerate() {
        let line_y = y + 40.0 + height * f32::from(u8::try_from(index).unwrap());
        let _ = canvas.draw_text(&run(&paragraph[range], line_y, None));
    }
    frame
}

#[test]
fn sans_at_scale_1() {
    assert_snapshot("text-sans-1x", &draw_text(|typefaces| typefaces.sans, 1));
}

#[test]
fn sans_at_scale_2() {
    assert_snapshot("text-sans-2x", &draw_text(|typefaces| typefaces.sans, 2));
}

#[test]
fn bold_at_scale_1() {
    assert_snapshot("text-bold-1x", &draw_text(|typefaces| typefaces.bold, 1));
}

#[test]
fn bold_at_scale_2() {
    assert_snapshot("text-bold-2x", &draw_text(|typefaces| typefaces.bold, 2));
}

#[test]
fn mono_at_scale_1() {
    assert_snapshot("text-mono-1x", &draw_text(|typefaces| typefaces.mono, 1));
}

#[test]
fn mono_at_scale_2() {
    assert_snapshot("text-mono-2x", &draw_text(|typefaces| typefaces.mono, 2));
}

#[test]
fn glyphs_are_rasterized_once_and_then_copied_from_the_cache() {
    let (mut assets, typefaces) = assets();
    let style = Typefaces::style(typefaces.sans, 13.0);
    let mut frame = FrameBuffer::new(400, 40);
    let mut draw = |assets: &mut render::Assets| {
        let mut canvas = Canvas::new(&mut frame, assets, 1.0);
        let run = TextRun {
            text: LINES[0],
            style,
            color: Color::WHITE,
            origin: Point::new(4.0, 4.0),
            max_width: None,
        };
        canvas.draw_text(&run)
    };
    let width = draw(&mut assets);
    let after_first = assets.fonts.rasterized_glyph_count();
    assert!(after_first > 20);
    let again = draw(&mut assets);
    assert_eq!(assets.fonts.rasterized_glyph_count(), after_first);
    assert!((width - again).abs() < f32::EPSILON);
    assert!((width - assets.fonts.measure(LINES[0], style)).abs() < 1e-3);
}
