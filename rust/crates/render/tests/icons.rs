//! Snapshots of every icon at 16 and at 32 logical pixels, at scale 1 and 2, on the ground of the dark and of the light
//! theme, and the licence of the outlines (`graph.icons`).

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use std::path::Path;

use render::{Appearance, Assets, Canvas, FrameBuffer, Icon, Point, Rect, Theme, ThemeName};
use support::assert_snapshot;

/// The number of icons on each line of a block.
const COLUMNS: u16 = 16;

/// The sizes each icon is drawn at, in logical pixels, and the side of its cell at that size.
const SIZES: [(u16, u16); 2] = [(16, 24), (32, 40)];

/// Draws every icon in blocks of lines of cells: at 16 then at 32, in the dark theme then in the light one.
fn draw_icons(scale: u8) -> FrameBuffer {
    let lines = u16::try_from(Icon::ALL.len()).unwrap().div_ceil(COLUMNS);
    let height: u16 = SIZES.iter().map(|(_, cell)| 2 * cell * lines).sum();
    let width = 40 * COLUMNS;
    let physical = |logical: u16| u32::from(logical) * u32::from(scale);
    let mut frame = FrameBuffer::new(physical(width), physical(height));
    let mut assets = Assets::new();
    let mut canvas = Canvas::new(&mut frame, &mut assets, f32::from(scale));
    let mut y = 0;
    for name in [ThemeName::Dark, ThemeName::Light] {
        let colors = Theme::new(name, Appearance::Dark).colors;
        for (size, cell) in SIZES {
            let block = Rect::new(0.0, f32::from(y), f32::from(width), f32::from(cell * lines));
            canvas.fill_rect(block, colors.page_background);
            for (index, icon) in (0..).zip(Icon::ALL) {
                let (line, column) = (index / COLUMNS, index % COLUMNS);
                let inset = (cell - size) / 2;
                let corner = Point::new(f32::from(column * cell + inset), f32::from(y + line * cell + inset));
                canvas.draw_icon(icon, corner, f32::from(size), colors.foreground);
            }
            y += cell * lines;
        }
    }
    frame
}

#[test]
fn graph_icons__each_icon_at_16_and_32_px_matches_its_snapshot_in_the_dark_and_the_light_theme() {
    assert_snapshot("icons-1x", &draw_icons(1));
    assert_snapshot("icons-2x", &draw_icons(2));
}

#[test]
fn graph_icons__the_licence_of_lucide_is_in_the_repository_beside_its_outlines() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons").join("lucide");
    let licence = std::fs::read_to_string(folder.join("LICENSE")).unwrap();
    assert!(licence.starts_with("ISC License"), "{licence}");
    assert!(licence.contains("Lucide Icons and Contributors"));
    for icon in Icon::ALL {
        let outline = folder.join(format!("{}.svg", icon.lucide_name()));
        let source = std::fs::read_to_string(&outline).unwrap();
        assert!(source.contains("@license lucide-static"), "{}", outline.display());
    }
}
