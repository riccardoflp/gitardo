//! Snapshots of every icon at its size of 16 and at 24, at scale 1 and 2, on a dark and a light background.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use render::{Assets, Canvas, Color, FrameBuffer, Icon, Point, Rect};
use support::assert_snapshot;

/// The logical side of the cell of each icon.
const CELL: u32 = 32;

/// Draws every icon in a row of cells, at 16 then at 24, first light on dark and then dark on light.
fn draw_icons(scale: u8) -> FrameBuffer {
    let columns = u32::try_from(Icon::ALL.len()).unwrap();
    let mut frame = FrameBuffer::new(columns * CELL * u32::from(scale), 4 * CELL * u32::from(scale));
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, f32::from(scale));
    let cell = 32.0;
    let rows = [
        (16.0, Color::rgb(0x20, 0x23, 0x28), Color::rgb(0xDD, 0xE1, 0xE6)),
        (24.0, Color::rgb(0x20, 0x23, 0x28), Color::rgb(0xDD, 0xE1, 0xE6)),
        (16.0, Color::WHITE, Color::rgb(0x1F, 0x23, 0x28)),
        (24.0, Color::WHITE, Color::rgb(0x1F, 0x23, 0x28)),
    ];
    let mut y = 0.0;
    for (size, background, foreground) in rows {
        canvas.fill_rect(Rect::new(0.0, y, 10_000.0, cell), background);
        let mut x = 0.0;
        for icon in Icon::ALL {
            let inset = (cell - size) / 2.0;
            canvas.draw_icon(icon, Point::new(x + inset, y + inset), size, foreground);
            x += cell;
        }
        y += cell;
    }
    frame
}

#[test]
fn icons_at_scale_1() {
    assert_snapshot("icons-1x", &draw_icons(1));
}

#[test]
fn icons_at_scale_2() {
    assert_snapshot("icons-2x", &draw_icons(2));
}
