//! Cards: the rounded boxes of the window, such as the inspector and the draft, and the floating ones of menus, dialogs,
//! pickers and tooltips with a soft shadow under them; and the lines that separate parts.

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

/// How many layers the shadow of a floating card is drawn in, each wider and lighter than the one before.
const SHADOW_LAYERS: u8 = 3;

impl Ui<'_> {
    /// Draws a card with an optional title at its top, and returns the rectangle inside it where its widgets go.
    pub fn panel(&mut self, rect: Rect, title: Option<&str>) -> Rect {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        self.card(rect, theme.sizes.radius.large);
        let mut area = Area::new(inset(rect, Insets::all(spacing.large + spacing.small)));
        if let Some(title) = title {
            let bar = area.top(theme.sizes.row_height);
            let _ = self.label(bar, &Label::new(title).kind(TextKind::Strong));
            let _ = area.top(spacing.small);
        }
        area.rest()
    }

    /// Draws a card of the window: the tone of cards, rounded, with a border.
    pub fn card(&mut self, rect: Rect, radius: f32) {
        let theme = *self.theme();
        self.fill_rounded_rect(rect, radius, theme.colors.card_background);
        let border = Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.border,
        };
        self.border(rect, border);
    }

    /// Draws a card that floats over the window, such as a menu, a dialog or a picker: a soft shadow under it, the tone
    /// of menus, rounded, with a border.
    pub(crate) fn floating_card(&mut self, rect: Rect, radius: f32) {
        let theme = *self.theme();
        let shadow = theme.colors.shadow;
        if !shadow.is_transparent() {
            let step = theme.sizes.spacing.tiny;
            for layer in (1..=SHADOW_LAYERS).rev() {
                let spread = f32::from(layer) * step;
                let alpha = shadow.alpha / (layer + 1);
                let under = Rect::new(rect.x, rect.y + spread, rect.width, rect.height).inset(-spread);
                self.fill_rounded_rect(under, radius + spread, shadow.with_alpha(alpha));
            }
        }
        self.fill_rounded_rect(rect, radius, theme.colors.menu_background);
        let border = Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.border,
        };
        self.border(rect, border);
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
