//! How long frames take, in a release build: `cargo test -p ui --release --test timing -- --ignored --nocapture`.
//! Not pass or fail but numbers to read; a frame should take well under 16 ms, and under 8 ms on the reference machine
//! (rule R10.5).

#![allow(clippy::unwrap_used, reason = "a test that cannot go on fails")]

mod support;

use std::time::{Duration, Instant};

use render::{Point, Rect, TextRun, ThemeName};
use support::{Window, wheel};
use ui::gallery::{self, GalleryState};
use ui::{List, SelectionMode, TextKind};

/// The frames measured after the first.
const FRAMES: u32 = 60;

/// Milliseconds of a duration, for printing.
fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn a_full_frame_of_the_gallery() {
    for scale in [1, 2] {
        let mut window = Window::new(ThemeName::Dark, (1920, 1080), scale);
        gallery::prepare(&mut window.memory, &window.theme);
        let state = GalleryState::default();
        let start = Instant::now();
        let _ = window.frame(&[], |ui| gallery::view(&state, ui));
        let first = start.elapsed();
        let start = Instant::now();
        for _ in 0..FRAMES {
            window.memory.redraw_everything();
            let _ = window.frame(&[], |ui| gallery::view(&state, ui));
        }
        let whole = start.elapsed() / FRAMES;
        let start = Instant::now();
        for _ in 0..FRAMES {
            let _ = window.frame(&[], |ui| gallery::view(&state, ui));
        }
        let unchanged = start.elapsed() / FRAMES;
        println!(
            "gallery at 1920 x 1080, scale {scale}: first frame {:.2} ms, a frame drawn whole {:.2} ms, a frame with nothing changed {:.2} ms",
            milliseconds(first),
            milliseconds(whole),
            milliseconds(unchanged),
        );
    }
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn a_list_of_a_million_rows_scrolled_by_the_wheel_every_frame() {
    let mut window = Window::new(ThemeName::Dark, (1920, 1080), 1);
    let list = List {
        name: "Rows",
        row_count: 1_000_000,
        row_height: window.theme.sizes.row_height,
        selection_mode: SelectionMode::Multiple,
        lead: 0.0,
    };
    let view = |ui: &mut ui::Ui<'_>| {
        let style = ui.text_style(TextKind::Body);
        let code = ui.text_style(TextKind::Code);
        let _ = ui.list("rows", ui.window(), &list, |ui, row| {
            let hash = format!("{:07x}", row.index.wrapping_mul(2_654_435_761) & 0x0FFF_FFFF);
            let text = format!(
                "Row {} of the list, a commit message of ordinary length for a row",
                row.index
            );
            let y = row.rect.y + 3.0;
            let muted = ui.theme().colors.muted_foreground;
            let _ = ui.text(&TextRun {
                text: &hash,
                style: code,
                color: muted,
                origin: Point::new(8.0, y),
                max_width: None,
            });
            let origin = Point::new(80.0, y);
            let width = Some(row.rect.width - 88.0);
            let _ = ui.text(&TextRun {
                text: &text,
                style,
                color: row.foreground,
                origin,
                max_width: width,
            });
            ui.fill_rect(
                Rect::new(row.rect.right() - 60.0, y, 40.0, 14.0),
                ui.theme().colors.badge_background,
            );
        });
    };
    let _ = window.frame(&[], view);
    let mut longest = Duration::ZERO;
    let start = Instant::now();
    for _ in 0..FRAMES {
        window.wait(16);
        let frame_start = Instant::now();
        let _ = window.frame(&[wheel(500.0, 500.0, 3.0)], view);
        longest = longest.max(frame_start.elapsed());
    }
    let average = start.elapsed() / FRAMES;
    println!(
        "a million rows at 1920 x 1080 scrolled by three lines each frame: {:.2} ms a frame on average, {:.2} ms at most",
        milliseconds(average),
        milliseconds(longest),
    );
}

#[test]
#[ignore = "a measure to read in a release build, not a check"]
fn the_list_of_the_gallery_scrolled_by_the_wheel_every_frame() {
    let mut gallery_window = Window::new(ThemeName::Dark, (1920, 1080), 1);
    gallery::prepare(&mut gallery_window.memory, &gallery_window.theme);
    let state = GalleryState::default();
    let _ = gallery_window.frame(&[], |ui| gallery::view(&state, ui));
    let start = Instant::now();
    for _ in 0..FRAMES {
        gallery_window.wait(16);
        let _ = gallery_window.frame(&[wheel(1200.0, 300.0, 3.0)], |ui| gallery::view(&state, ui));
    }
    println!(
        "the list of the gallery at 1920 x 1080 scrolled by three lines each frame: {:.2} ms a frame",
        milliseconds(start.elapsed() / FRAMES),
    );
}
