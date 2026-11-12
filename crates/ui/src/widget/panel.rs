//! Boxes that group widgets, with a border and an optional title, and the lines that separate parts.

use render::{Border, Rect};

use super::label::Label;
use crate::frame::{TextKind, Ui};
use crate::layout::{Area, Insets, inset};

/// Which way a separator runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Left to right: a separator between things above and below.
    Horizontal,
    /// Top to bottom: a separator between things left and right.
    Vertical,
}

impl Ui<'_> {
    /// Draws a box with a border and, when given, a title over a line, and returns the rectangle inside it where its
    /// widgets go.
    pub fn panel(&mut self, rect: Rect, title: Option<&str>) -> Rect {
        let theme = *self.theme();
        let radius = theme.sizes.radius.large;
        self.fill_rounded_rect(rect, radius, theme.colors.page_background);
        let border = Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.border,
        };
        self.border(rect, border);
        let spacing = theme.sizes.spacing;
        let mut area = Area::new(inset(rect, Insets::all(theme.sizes.border)));
        if let Some(title) = title {
            let bar = area.top(theme.sizes.row_height + spacing.medium);
            let text = inset(bar, Insets::symmetric(spacing.large, 0.0));
            let _ = self.label(text, &Label::new(title).kind(TextKind::Strong));
            let line = area.top(theme.sizes.border);
            self.fill_rect(line, theme.colors.border);
        }
        inset(area.rest(), Insets::all(spacing.large))
    }

    /// Draws a separator in the middle of a rectangle, as thick as a border.
    pub fn separator(&mut self, rect: Rect, axis: Axis) {
        let theme = *self.theme();
        let thickness = theme.sizes.border;
        let line = match axis {
            Axis::Horizontal => Rect::new(
                rect.x,
                (rect.center().y - thickness * 0.5).round(),
                rect.width,
                thickness,
            ),
            Axis::Vertical => Rect::new(
                (rect.center().x - thickness * 0.5).round(),
                rect.y,
                thickness,
                rect.height,
            ),
        };
        self.fill_rect(line, theme.colors.border);
    }
}
