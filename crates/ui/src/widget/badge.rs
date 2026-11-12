//! Badges: a short label in a rounded box on a row, such as the name of a branch in the graph or a count.

use render::{Border, Icon, Point, Rect, TextRun};

use super::Tone;
use crate::frame::{TextKind, Ui};
use crate::layout::{Align, place};

/// How opaque the tint of a colored badge is, out of 255: enough to tell the colors apart, little enough that the
/// text on it stays readable.
const TINT_ALPHA: u8 = 48;

/// A badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Badge<'text> {
    /// The words in it.
    pub text: &'text str,
    /// The icon before them, such as a branch or a tag.
    pub icon: Option<Icon>,
    /// Its color: the normal one for a plain badge, another for a tinted badge with a border of that color.
    pub tone: Tone,
}

impl Ui<'_> {
    /// The width a badge takes.
    #[must_use]
    pub fn badge_width(&self, badge: &Badge<'_>) -> f32 {
        let theme = self.theme();
        let spacing = theme.sizes.spacing;
        let icon = badge.icon.map_or(0.0, |_| theme.sizes.font.small + spacing.tiny);
        self.measure(badge.text, self.text_style(TextKind::Small)) + icon + 2.0 * (spacing.small + spacing.tiny)
    }

    /// Draws a badge at the start of a rectangle, as high as a line of small text with room around it, in the middle
    /// down, and returns the rectangle it takes.
    pub fn badge(&mut self, rect: Rect, badge: &Badge<'_>) -> Rect {
        let theme = *self.theme();
        let style = self.text_style(TextKind::Small);
        let line = self.line_height(style);
        let height = (line + theme.sizes.spacing.tiny * 2.0).min(rect.height);
        let width = self.badge_width(badge).min(rect.width);
        let shape = place(rect, (width, height), (Align::Start, Align::Center));
        let radius = theme.sizes.radius.small;
        let (background, foreground, edge) = match badge.tone {
            Tone::Normal | Tone::Muted | Tone::Disabled => {
                (theme.colors.badge_background, theme.colors.badge_foreground, None)
            }
            Tone::Accent | Tone::Success | Tone::Warning | Tone::Error => {
                let color = badge.tone.color(&theme);
                (color.with_alpha(TINT_ALPHA), theme.colors.foreground, Some(color))
            }
        };
        self.fill_rounded_rect(shape, radius, background);
        if let Some(color) = edge {
            let width = theme.sizes.border;
            self.border(shape, Border { width, radius, color });
        }
        let mut x = shape.x + theme.sizes.spacing.small + theme.sizes.spacing.tiny;
        if let Some(icon) = badge.icon {
            let side = theme.sizes.font.small;
            let at = place(shape, (side, side), (Align::Start, Align::Center));
            let icon_color = edge.unwrap_or(foreground);
            self.icon(icon, Point::new(x, at.y), side, icon_color);
            x += side + theme.sizes.spacing.tiny;
        }
        let text_y = place(shape, (0.0, line), (Align::Start, Align::Center)).y;
        let run = TextRun {
            text: badge.text,
            style,
            color: foreground,
            origin: Point::new(x, text_y),
            max_width: Some((shape.right() - x - theme.sizes.spacing.small).max(0.0)),
        };
        let _ = self.text(&run);
        shape
    }
}
