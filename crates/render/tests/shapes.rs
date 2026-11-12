//! Snapshots of shapes: rectangles, rounded rectangles, borders, circles, lines and curves, at scale 1 and 2.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use render::{Assets, Border, Canvas, Color, FillRule, FrameBuffer, LineCap, Path, PixelRect, Point, Rect, Stroke};
use support::assert_snapshot;

/// The logical width of the picture of shapes.
const WIDTH: u32 = 240;
/// The logical height of the picture of shapes.
const HEIGHT: u32 = 160;

/// Draws the picture of shapes at a whole scale, into a new frame.
fn draw_shapes(scale: u8) -> FrameBuffer {
    let mut frame = FrameBuffer::new(WIDTH * u32::from(scale), HEIGHT * u32::from(scale));
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, f32::from(scale));
    canvas.clear(Color::rgb(0x20, 0x23, 0x28));
    draw_rectangles(&mut canvas);
    draw_circles(&mut canvas);
    draw_lines(&mut canvas);
    draw_curves(&mut canvas);
    draw_clipped(&mut canvas);
    frame
}

fn draw_rectangles(canvas: &mut Canvas<'_>) {
    canvas.fill_rect(Rect::new(10.0, 10.0, 40.0, 30.0), Color::rgb(0x5A, 0x9C, 0xF0));
    canvas.fill_rect(Rect::new(30.0, 25.0, 40.0, 30.0), Color::rgba(0xE0, 0x90, 0x4A, 160));
    canvas.fill_rect(Rect::new(10.3, 60.6, 20.4, 9.8), Color::WHITE);
    canvas.fill_rounded_rect(Rect::new(80.0, 10.0, 50.0, 30.0), 6.0, Color::rgb(0x5C, 0xC2, 0x8A));
    canvas.fill_rounded_rect(Rect::new(140.0, 10.0, 30.0, 30.0), 15.0, Color::rgb(0xC7, 0x79, 0xE0));
    let border = Border {
        width: 1.0,
        radius: 0.0,
        color: Color::rgb(0xDD, 0xE1, 0xE6),
    };
    canvas.stroke_border(Rect::new(180.0, 10.0, 50.0, 30.0), &border);
    let rounded = Border {
        width: 2.0,
        radius: 8.0,
        color: Color::rgb(0xE0, 0xC0, 0x50),
    };
    canvas.stroke_border(Rect::new(40.0, 60.0, 50.0, 26.0), &rounded);
}

fn draw_circles(canvas: &mut Canvas<'_>) {
    canvas.fill_circle(Point::new(110.0, 70.0), 10.0, Color::rgb(0xF0, 0x6A, 0x6A));
    canvas.fill_circle(Point::new(135.0, 70.0), 4.0, Color::rgb(0x4A, 0xC0, 0xC0));
    let ring = Stroke {
        width: 1.5,
        cap: LineCap::Butt,
        color: Color::WHITE,
    };
    canvas.stroke_circle(Point::new(160.0, 70.0), 9.0, &ring);
    let mut star = Path::new();
    let points = [
        (200.0, 52.0),
        (210.0, 88.0),
        (182.0, 64.0),
        (218.0, 64.0),
        (190.0, 88.0),
    ];
    for (index, (x, y)) in points.into_iter().enumerate() {
        let _ = if index == 0 {
            star.move_to(Point::new(x, y))
        } else {
            star.line_to(Point::new(x, y))
        };
    }
    let _ = star.close();
    canvas.fill_path(&star, Color::rgb(0xE0, 0xB0, 0x50), FillRule::EvenOdd);
}

fn draw_lines(canvas: &mut Canvas<'_>) {
    for (y, cap) in [
        (105.0, LineCap::Butt),
        (115.0, LineCap::Round),
        (125.0, LineCap::Square),
    ] {
        let stroke = Stroke {
            width: 4.0,
            cap,
            color: Color::rgb(0xDD, 0xE1, 0xE6),
        };
        canvas.line(Point::new(15.0, y), Point::new(60.0, y), &stroke);
    }
    for (x, width) in [(75.0, 1.0), (83.0, 1.5), (91.0, 2.0)] {
        let stroke = Stroke {
            width,
            cap: LineCap::Butt,
            color: Color::rgb(0x5A, 0x9C, 0xF0),
        };
        canvas.line(Point::new(x, 100.0), Point::new(x + 30.0, 150.0), &stroke);
    }
    let mut zigzag = Path::new();
    let _ = zigzag
        .move_to(Point::new(15.0, 150.0))
        .line_to(Point::new(30.0, 138.0))
        .line_to(Point::new(45.0, 150.0))
        .line_to(Point::new(60.0, 138.0));
    let stroke = Stroke {
        width: 3.0,
        cap: LineCap::Round,
        color: Color::rgb(0xF0, 0x6A, 0x8A),
    };
    canvas.stroke_path(&zigzag, &stroke);
}

/// Curves like the lines of the graph: a lane leaving another for the next column, and a merge coming back.
fn draw_curves(canvas: &mut Canvas<'_>) {
    let lanes = [
        (130.0, Color::rgb(0x5A, 0x9C, 0xF0)),
        (144.0, Color::rgb(0xE0, 0x90, 0x4A)),
        (158.0, Color::rgb(0x5C, 0xC2, 0x8A)),
    ];
    for (x, color) in lanes {
        let stroke = Stroke {
            width: 1.5,
            cap: LineCap::Butt,
            color,
        };
        canvas.line(Point::new(x, 100.0), Point::new(x, 155.0), &stroke);
        let mut curve = Path::new();
        let _ = curve.move_to(Point::new(x, 110.0)).cubic_to(
            Point::new(x, 121.0),
            Point::new(x + 14.0, 117.0),
            Point::new(x + 14.0, 128.0),
        );
        canvas.stroke_path(&curve, &stroke);
        canvas.fill_circle(Point::new(x, 110.0), 4.0, color);
    }
    let mut quadratic = Path::new();
    let _ = quadratic
        .move_to(Point::new(180.0, 150.0))
        .quadratic_to(Point::new(200.0, 95.0), Point::new(230.0, 150.0));
    let stroke = Stroke {
        width: 2.0,
        cap: LineCap::Round,
        color: Color::rgb(0xA0, 0xB0, 0x50),
    };
    canvas.stroke_path(&quadratic, &stroke);
}

/// A circle cut by a clip, and a translucent band drawn inside a translated, clipped region.
fn draw_clipped(canvas: &mut Canvas<'_>) {
    canvas.save();
    canvas.translate(180.0, 100.0);
    canvas.clip(Rect::new(0.0, 0.0, 25.0, 25.0));
    canvas.fill_circle(Point::new(25.0, 25.0), 20.0, Color::rgb(0x4A, 0xC0, 0xC0));
    canvas.fill_rect(Rect::new(-50.0, 10.0, 200.0, 5.0), Color::rgba(255, 255, 255, 100));
    canvas.restore();
}

#[test]
fn shapes_at_scale_1() {
    assert_snapshot("shapes-1x", &draw_shapes(1));
}

#[test]
fn shapes_at_scale_2() {
    assert_snapshot("shapes-2x", &draw_shapes(2));
}

#[test]
fn drawing_marks_only_what_it_changes_as_dirty() {
    let mut frame = FrameBuffer::new(100, 100);
    let _ = frame.take_dirty_regions();
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, 2.0);
    canvas.fill_rect(Rect::new(5.0, 5.0, 10.0, 10.0), Color::WHITE);
    canvas.fill_circle(Point::new(40.0, 40.0), 5.0, Color::WHITE);
    canvas.save();
    canvas.clip(Rect::new(0.0, 0.0, 1.0, 1.0));
    canvas.fill_rect(Rect::new(20.0, 0.0, 10.0, 10.0), Color::WHITE);
    canvas.restore();
    let regions = frame.take_dirty_regions();
    assert_eq!(regions.len(), 2, "{regions:?}");
    assert_eq!(regions[0], PixelRect::new(10, 10, 30, 30));
    assert!(
        regions[1].contains(PixelRect::new(71, 71, 89, 89)) && regions[1].area() < 25 * 25,
        "{regions:?}"
    );
}

#[test]
fn a_large_rectangle_covers_exactly_its_snapped_pixels() {
    let mut frame = FrameBuffer::new(40, 30);
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, 1.5);
    canvas.fill_rect(Rect::new(1.0, 1.0, 10.0, 10.0), Color::WHITE);
    assert_eq!(frame.pixel(1, 1), Some(Color::BLACK));
    assert_eq!(frame.pixel(2, 2), Some(Color::WHITE));
    assert_eq!(frame.pixel(16, 16), Some(Color::WHITE));
    assert_eq!(frame.pixel(17, 17), Some(Color::BLACK));
}
