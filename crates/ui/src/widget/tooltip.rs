//! Tooltips: a few words in a box near the pointer, shown after it stayed a moment over a widget, kept inside the
//! window, on the layer above everything.

use std::time::Duration;

use render::{Border, Point, Rect, TextRun};

use super::metrics;
use crate::frame::{TextKind, Ui};
use crate::layer::Layer;

/// How long the pointer stays over a widget before its tooltip shows.
pub(crate) const DELAY: Duration = Duration::from_millis(600);

/// Where a box of a size goes near the pointer: below it, past the arrow, and above it when there is no room below,
/// moved left or right to stay inside the window.
pub(crate) fn place(size: (f32, f32), pointer: Point, window: Rect, gap: f32) -> Rect {
    let (width, height) = (size.0.min(window.width), size.1.min(window.height));
    let below = pointer.y + gap * 2.0;
    let y = if below + height <= window.bottom() {
        below
    } else {
        (pointer.y - gap - height).max(window.y)
    };
    let x = pointer.x.min(window.right() - width).max(window.x);
    Rect::new(x, y, width, height)
}

/// Draws a tooltip near the pointer.
pub(crate) fn draw(ui: &mut Ui<'_>, text: &str, pointer: Point) {
    let theme = *ui.theme();
    let style = ui.text_style(TextKind::Body);
    let padding = theme.sizes.spacing.small + theme.sizes.spacing.tiny;
    let widest = metrics::tooltip_width(&theme);
    let lines = ui.fonts().wrap(text, style, widest - 2.0 * padding);
    let line_height = ui.line_height(style);
    let text_width = lines
        .iter()
        .map(|line| ui.measure(text.get(line.clone()).unwrap_or_default(), style))
        .fold(0.0_f32, f32::max);
    let size = (
        text_width + 2.0 * padding,
        crate::number::float_of_count(lines.len()) * line_height + 2.0 * padding,
    );
    let rect = place(size, pointer, ui.window(), theme.sizes.row_height * 0.5);
    ui.layer(Layer::Tooltip, |ui| {
        let radius = theme.sizes.radius.medium;
        let shadow = Rect::new(rect.x, rect.y + theme.sizes.spacing.tiny, rect.width, rect.height);
        ui.fill_rounded_rect(shadow, radius, theme.colors.shadow);
        ui.fill_rounded_rect(rect, radius, theme.colors.tooltip_background);
        let border = Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.border,
        };
        ui.border(rect, border);
        let mut y = rect.y + padding;
        for line in &lines {
            let run = TextRun {
                text: text.get(line.clone()).unwrap_or_default(),
                style,
                color: theme.colors.tooltip_foreground,
                origin: Point::new(rect.x + padding, y),
                max_width: Some(rect.width - padding),
            };
            let _ = ui.text(&run);
            y += line_height;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tooltip_stays_inside_the_window() {
        let window = Rect::new(0.0, 0.0, 400.0, 300.0);
        assert_eq!(
            place((100.0, 30.0), Point::new(50.0, 50.0), window, 10.0),
            Rect::new(50.0, 70.0, 100.0, 30.0)
        );
        assert_eq!(
            place((100.0, 30.0), Point::new(380.0, 50.0), window, 10.0),
            Rect::new(300.0, 70.0, 100.0, 30.0)
        );
        assert_eq!(
            place((100.0, 30.0), Point::new(50.0, 290.0), window, 10.0),
            Rect::new(50.0, 250.0, 100.0, 30.0)
        );
        assert_eq!(place((500.0, 30.0), Point::new(50.0, 50.0), window, 10.0).width, 400.0);
    }
}
