//! Pills: a few words with round ends on a row, such as a branch in the color of its lane, a tag, or a count.

use render::{Border, Icon, Point, Rect, TextRun};

use super::{Tone, is_high_contrast};
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::layout::{Align, place};

/// How opaque the tint behind a colored pill is, out of 255: enough to tell the colors apart, little enough that its
/// words, in the color itself, stay readable.
const TINT_ALPHA: u8 = 40;

/// A pill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Badge<'text> {
    /// The words in it.
    pub text: &'text str,
    /// The icon before them, such as a branch or a tag.
    pub icon: Option<Icon>,
    /// Its color: the normal or muted one for a count on the sunken tone, another for words in that color on a tint
    /// of it, such as the color of a lane.
    pub tone: Tone,
}

impl Ui<'_> {
    /// The width a pill takes.
    #[must_use]
    pub fn badge_width(&self, badge: &Badge<'_>) -> f32 {
        let theme = self.theme();
        let spacing = theme.sizes.spacing;
        let icon = badge.icon.map_or(0.0, |_| theme.sizes.font.small + spacing.small);
        self.measure(badge.text, self.text_style(TextKind::SmallStrong)) + icon + 2.0 * spacing.medium
    }

    /// The height of a pill.
    #[must_use]
    pub fn badge_height(&self) -> f32 {
        let theme = self.theme();
        self.line_height(self.text_style(TextKind::SmallStrong)) + 2.0 * theme.sizes.spacing.tiny
    }

    /// Draws a pill at the start of a rectangle, in the middle down, and returns the rectangle it takes.
    pub fn badge(&mut self, rect: Rect, badge: &Badge<'_>) -> Rect {
        let theme = *self.theme();
        let style = self.text_style(TextKind::SmallStrong);
        let line = self.line_height(style);
        let height = self.badge_height().min(rect.height);
        let width = self.badge_width(badge).min(rect.width);
        let shape = place(rect, (width, height), (Align::Start, Align::Center));
        let radius = height * 0.5;
        let (background, foreground, edge) = match badge.tone {
            Tone::Normal | Tone::Disabled => (theme.colors.badge_background, theme.colors.badge_foreground, None),
            Tone::Muted => (theme.colors.badge_background, theme.colors.muted_foreground, None),
            Tone::Accent | Tone::Success | Tone::Warning | Tone::Error | Tone::Lane(_) => {
                let color = badge.tone.color(&theme);
                let edge = is_high_contrast(&theme).then_some(color);
                (color.with_alpha(TINT_ALPHA), color, edge)
            }
        };
        self.fill_rounded_rect(shape, radius, background);
        if let Some(color) = edge {
            let width = theme.sizes.border;
            self.border(shape, Border { width, radius, color });
        }
        let mut x = shape.x + theme.sizes.spacing.medium;
        if let Some(icon) = badge.icon {
            let side = theme.sizes.font.small;
            let at = place(shape, (side, side), (Align::Start, Align::Center));
            self.icon(icon, Point::new(x, at.y), side, foreground);
            x += side + theme.sizes.spacing.small;
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
        let id = self.id(("badge", badge.text, shape.x.to_bits(), shape.y.to_bits()));
        self.access_node(id, shape, &Description::new(AccessRole::Label, badge.text));
        shape
    }
}
