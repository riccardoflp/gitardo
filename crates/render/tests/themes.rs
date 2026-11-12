//! A swatch of each of the five themes: every color token in a box, the lanes of the graph, and a few widgets drawn
//! with them, so that a change of a token shows in the snapshots.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "a helper of a test that cannot go on fails the test"
)]

mod support;

use render::{
    Appearance, Border, Canvas, Color, Colors, FrameBuffer, Icon, LineCap, Point, Rect, Stroke, TextRun, Theme,
    ThemeName,
};
use support::{Typefaces, assert_snapshot, assets};

/// The logical width of a swatch.
const WIDTH: u32 = 320;
/// The logical height of a swatch.
const HEIGHT: u32 = 200;

/// The tokens behind things.
fn backgrounds(colors: &Colors) -> Vec<Color> {
    vec![
        colors.window_background,
        colors.side_bar_background,
        colors.page_background,
        colors.panel_background,
        colors.toolbar_background,
        colors.status_bar_background,
        colors.tab_background,
        colors.tab_active_background,
        colors.list_hover,
        colors.list_selection,
        colors.list_inactive_selection,
        colors.input_background,
        colors.button_background,
        colors.button_hover,
        colors.secondary_button_background,
        colors.secondary_button_hover,
        colors.badge_background,
        colors.menu_background,
        colors.menu_selection,
        colors.tooltip_background,
        colors.added_background,
        colors.removed_background,
        colors.text_selection,
        colors.find_match,
        colors.scrollbar_thumb,
        colors.scrollbar_thumb_hover,
        colors.shadow,
    ]
}

/// The tokens of text, lines and states.
fn foregrounds(colors: &Colors) -> Vec<Color> {
    vec![
        colors.foreground,
        colors.muted_foreground,
        colors.disabled_foreground,
        colors.status_bar_foreground,
        colors.tab_foreground,
        colors.tab_active_foreground,
        colors.list_selection_foreground,
        colors.input_foreground,
        colors.input_placeholder,
        colors.input_border,
        colors.button_foreground,
        colors.secondary_button_foreground,
        colors.badge_foreground,
        colors.menu_foreground,
        colors.menu_selection_foreground,
        colors.tooltip_foreground,
        colors.border,
        colors.focus_ring,
        colors.accent,
        colors.error,
        colors.warning,
        colors.info,
        colors.success,
        colors.added,
        colors.removed,
        colors.modified,
        colors.conflict,
    ]
}

/// Draws a row of boxes, one for each color.
fn draw_boxes(canvas: &mut Canvas<'_>, theme: &Theme, colors: &[Color], y: f32) {
    let mut x = 8.0;
    for color in colors {
        canvas.fill_rect(Rect::new(x, y, 10.0, 10.0), *color);
        let border = Border {
            width: 1.0,
            radius: 0.0,
            color: theme.colors.border,
        };
        canvas.stroke_border(Rect::new(x - 1.0, y - 1.0, 12.0, 12.0), &border);
        x += 11.0;
    }
}

/// Draws a list row, selected, a button, an input box and the lanes of the graph, with the theme's tokens.
fn draw_widgets(canvas: &mut Canvas<'_>, theme: &Theme, typefaces: &Typefaces) {
    let (colors, sizes) = (theme.colors, theme.sizes);
    let style = Typefaces::style(typefaces.sans, sizes.font.body);
    let row = Rect::new(0.0, 50.0, 200.0, sizes.row_height);
    canvas.fill_rect(row, colors.list_selection);
    canvas.draw_icon(
        Icon::Branch,
        Point::new(8.0, row.y + 3.0),
        sizes.icon,
        colors.list_selection_foreground,
    );
    let text = |text, origin, color| TextRun {
        text,
        style,
        color,
        origin,
        max_width: None,
    };
    let _ = canvas.draw_text(&text(
        "main",
        Point::new(30.0, row.y + 3.0),
        colors.list_selection_foreground,
    ));
    let _ = canvas.draw_text(&text(
        "3 commits ahead",
        Point::new(70.0, row.y + 3.0),
        colors.muted_foreground,
    ));
    let button = Rect::new(8.0, 84.0, 104.0, 24.0);
    canvas.fill_rounded_rect(button, sizes.radius.medium, colors.button_background);
    let _ = canvas.draw_text(&text("Create branch", Point::new(16.0, 88.0), colors.button_foreground));
    let input = Rect::new(120.0, 84.0, 120.0, 24.0);
    canvas.fill_rounded_rect(input, sizes.radius.medium, colors.input_background);
    let focus = Border {
        width: sizes.focus_ring,
        radius: sizes.radius.medium,
        color: colors.focus_ring,
    };
    canvas.stroke_border(input, &focus);
    let _ = canvas.draw_text(&text(
        "Search commits",
        Point::new(128.0, 88.0),
        colors.input_placeholder,
    ));
    for lane in 0..theme.lanes.len() {
        let x = 16.0 + 14.0 * f32::from(u8::try_from(lane).unwrap());
        let stroke = Stroke {
            width: sizes.graph_line,
            cap: LineCap::Butt,
            color: theme.lane(lane),
        };
        canvas.line(Point::new(x, 120.0), Point::new(x, 160.0), &stroke);
        canvas.fill_circle(Point::new(x, 140.0), sizes.graph_dot, theme.lane(lane));
    }
    let _ = canvas.draw_text(&text("Fix the parser", Point::new(130.0, 132.0), colors.foreground));
}

/// Draws the swatch of a theme at a whole scale.
fn draw_swatch(theme: &Theme, scale: u8) -> FrameBuffer {
    let (mut assets, typefaces) = assets();
    let mut frame = FrameBuffer::new(WIDTH * u32::from(scale), HEIGHT * u32::from(scale));
    let mut canvas = Canvas::new(&mut frame, &mut assets, f32::from(scale));
    canvas.clear(theme.colors.page_background);
    draw_boxes(&mut canvas, theme, &backgrounds(&theme.colors), 8.0);
    draw_boxes(&mut canvas, theme, &foregrounds(&theme.colors), 26.0);
    draw_widgets(&mut canvas, theme, &typefaces);
    canvas.fill_rect(Rect::new(0.0, 176.0, 320.0, 24.0), theme.colors.status_bar_background);
    let style = Typefaces::style(typefaces.sans, theme.sizes.font.small);
    let status = TextRun {
        text: "main \u{2191}2 \u{2193}0",
        style,
        color: theme.colors.status_bar_foreground,
        origin: Point::new(8.0, 181.0),
        max_width: None,
    };
    let _ = canvas.draw_text(&status);
    frame
}

#[test]
fn dark_theme() {
    assert_snapshot(
        "theme-dark",
        &draw_swatch(&Theme::new(ThemeName::Dark, Appearance::Light), 1),
    );
}

#[test]
fn light_theme() {
    assert_snapshot(
        "theme-light",
        &draw_swatch(&Theme::new(ThemeName::Light, Appearance::Dark), 1),
    );
}

#[test]
fn high_contrast_dark_theme() {
    assert_snapshot(
        "theme-high-contrast-dark",
        &draw_swatch(&Theme::new(ThemeName::HighContrastDark, Appearance::Light), 1),
    );
}

#[test]
fn high_contrast_light_theme() {
    assert_snapshot(
        "theme-high-contrast-light",
        &draw_swatch(&Theme::new(ThemeName::HighContrastLight, Appearance::Dark), 1),
    );
}

#[test]
fn system_theme_as_the_system_is() {
    let dark = Theme::new(ThemeName::System, Appearance::Dark);
    let light = Theme::new(ThemeName::System, Appearance::Light);
    assert_eq!(dark, Theme::new(ThemeName::Dark, Appearance::Dark));
    assert_snapshot("theme-system-light-2x", &draw_swatch(&light, 2));
}
