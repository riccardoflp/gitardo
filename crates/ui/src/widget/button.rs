//! Buttons: the main button of a place, the other buttons, and buttons that are only an icon, each with an optional
//! icon before its words and a tooltip that finishes the thought of its label.

use std::hash::Hash;

use render::{Border, Color, Icon, Point, Rect, TextRun, Theme};

use super::{Availability, is_high_contrast, metrics};
use crate::frame::{TextKind, Ui};
use crate::interaction::{Response, Sense};
use crate::layout::{Align, place};

/// How a button looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonKind {
    /// The main button of a place, filled with the accent of the theme.
    Primary,
    /// The main button of a place when what it does cannot be undone, filled with the color of errors.
    Danger,
    /// The other buttons.
    #[default]
    Secondary,
    /// A button that is only an icon, with no background until the pointer is over it.
    Icon,
}

/// A button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button<'text> {
    /// The words on it; none for a button that is only an icon.
    pub label: &'text str,
    /// The icon before the words, or the only icon.
    pub icon: Option<Icon>,
    /// How it looks.
    pub kind: ButtonKind,
    /// Whether it can be used now.
    pub availability: Availability,
    /// What it does, in a tooltip; a button that is only an icon should always have one.
    pub tooltip: Option<&'text str>,
}

impl<'text> Button<'text> {
    /// The main button of a place, with words.
    #[must_use]
    pub fn primary(label: &'text str) -> Button<'text> {
        Button {
            label,
            icon: None,
            kind: ButtonKind::Primary,
            availability: Availability::Enabled,
            tooltip: None,
        }
    }

    /// Another button, with words.
    #[must_use]
    pub fn secondary(label: &'text str) -> Button<'text> {
        Button {
            kind: ButtonKind::Secondary,
            ..Button::primary(label)
        }
    }

    /// A button that is only an icon, with the tooltip that says what it does.
    #[must_use]
    pub fn icon(icon: Icon, tooltip: &'text str) -> Button<'text> {
        Button {
            label: "",
            icon: Some(icon),
            kind: ButtonKind::Icon,
            availability: Availability::Enabled,
            tooltip: Some(tooltip),
        }
    }

    /// The main button of a place that does something that cannot be undone, with words.
    #[must_use]
    pub fn danger(label: &'text str) -> Button<'text> {
        Button {
            kind: ButtonKind::Danger,
            ..Button::primary(label)
        }
    }

    /// The same button with an icon before its words.
    #[must_use]
    pub fn with_icon(self, icon: Icon) -> Button<'text> {
        Button {
            icon: Some(icon),
            ..self
        }
    }

    /// The same button with a tooltip.
    #[must_use]
    pub fn with_tooltip(self, tooltip: &'text str) -> Button<'text> {
        Button {
            tooltip: Some(tooltip),
            ..self
        }
    }

    /// The same button, disabled.
    #[must_use]
    pub fn disabled(self) -> Button<'text> {
        Button {
            availability: Availability::Disabled,
            ..self
        }
    }
}

/// The colors of a button in a state.
struct Look {
    /// Behind it.
    background: Color,
    /// Its words and icon.
    foreground: Color,
    /// Its border, when it has one.
    border: Option<Color>,
}

impl Ui<'_> {
    /// The size a button takes: its words and icon with room around them, and the height of a control.
    #[must_use]
    pub fn button_size(&self, button: &Button<'_>) -> (f32, f32) {
        let theme = self.theme();
        let height = metrics::control_height(theme);
        let spacing = theme.sizes.spacing;
        let icon = button.icon.map_or(0.0, |_| theme.sizes.icon);
        if button.label.is_empty() {
            return (height, height);
        }
        let words = self.measure(button.label, self.text_style(TextKind::Body));
        let gap = if button.icon.is_some() {
            spacing.small + spacing.tiny
        } else {
            0.0
        };
        (icon + gap + words + 2.0 * spacing.large, height)
    }

    /// Draws a button and says whether it was clicked.
    pub fn button(&mut self, key: impl Hash, rect: Rect, button: &Button<'_>) -> Response {
        let id = self.id(key);
        let is_enabled = button.availability == Availability::Enabled;
        let sense = if is_enabled {
            Sense::CLICK | Sense::FOCUS
        } else {
            Sense::HOVER
        };
        let response = self.interact(id, rect, sense);
        let look = look(self.theme(), button, &response);
        let radius = self.theme().sizes.radius.medium;
        self.fill_rounded_rect(rect, radius, look.background);
        if let Some(color) = look.border {
            let width = self.theme().sizes.border;
            self.border(rect, Border { width, radius, color });
        }
        self.button_content(rect, button, look.foreground);
        if response.shows_focus {
            self.focus_ring(rect, radius);
        }
        if let Some(tooltip) = button.tooltip {
            self.tooltip(&response, tooltip);
        }
        response
    }

    /// Draws the ring around what has the focus, inside its rectangle.
    pub fn focus_ring(&mut self, rect: Rect, radius: f32) {
        let theme = self.theme();
        let border = Border {
            width: theme.sizes.focus_ring,
            radius,
            color: theme.colors.focus_ring,
        };
        self.border(rect, border);
    }

    /// Draws the icon and the words of a button in the middle of its rectangle.
    fn button_content(&mut self, rect: Rect, button: &Button<'_>, color: Color) {
        let theme = *self.theme();
        let style = self.text_style(TextKind::Body);
        let icon_side = button.icon.map_or(0.0, |_| theme.sizes.icon);
        let gap = if button.icon.is_some() && !button.label.is_empty() {
            theme.sizes.spacing.small + theme.sizes.spacing.tiny
        } else {
            0.0
        };
        let words = if button.label.is_empty() {
            0.0
        } else {
            self.measure(button.label, style)
        };
        let content = place(
            rect.inset(theme.sizes.spacing.small),
            (icon_side + gap + words, rect.height),
            (Align::Center, Align::Start),
        );
        if let Some(icon) = button.icon {
            let square = Rect::new(content.x, rect.y, icon_side, rect.height);
            let at = place(square, (icon_side, icon_side), (Align::Start, Align::Center));
            self.icon(icon, Point::new(at.x, at.y), icon_side, color);
        }
        if !button.label.is_empty() {
            let height = self.line_height(style);
            let x = content.x + icon_side + gap;
            let line = place(rect, (0.0, height), (Align::Start, Align::Center));
            let run = TextRun {
                text: button.label,
                style,
                color,
                origin: Point::new(x, line.y),
                max_width: Some((content.right() - x).max(0.0)),
            };
            let _ = self.text(&run);
        }
    }
}

/// The colors of a button, by its kind and whether it is enabled, under the pointer or pressed.
fn look(theme: &Theme, button: &Button<'_>, response: &Response) -> Look {
    let colors = &theme.colors;
    let is_lit = response.hovered || response.pressed;
    let high_contrast = is_high_contrast(theme);
    if button.availability == Availability::Disabled {
        let background = match button.kind {
            ButtonKind::Icon => Color::TRANSPARENT,
            ButtonKind::Primary | ButtonKind::Danger | ButtonKind::Secondary => colors.secondary_button_background,
        };
        return Look {
            background,
            foreground: colors.disabled_foreground,
            border: high_contrast.then_some(colors.disabled_foreground),
        };
    }
    match button.kind {
        ButtonKind::Primary => Look {
            background: if is_lit {
                colors.button_hover
            } else {
                colors.button_background
            },
            foreground: colors.button_foreground,
            border: high_contrast.then_some(colors.border),
        },
        ButtonKind::Danger => Look {
            background: if is_lit {
                colors.error.with_alpha(DANGER_LIT).over(colors.page_background)
            } else {
                colors.error
            },
            foreground: readable_on(colors.error, colors.button_foreground, colors.window_background),
            border: high_contrast.then_some(colors.border),
        },
        ButtonKind::Secondary => Look {
            background: if is_lit {
                colors.secondary_button_hover
            } else {
                colors.secondary_button_background
            },
            foreground: colors.secondary_button_foreground,
            border: high_contrast.then_some(colors.border),
        },
        ButtonKind::Icon => Look {
            background: if is_lit { colors.list_hover } else { Color::TRANSPARENT },
            foreground: colors.foreground,
            border: (high_contrast && is_lit).then_some(colors.border),
        },
    }
}

/// How much of the color of errors shows over the page under a dangerous button the pointer is over, out of 255.
const DANGER_LIT: u8 = 210;

/// Of two colors, the one easier to read on a background: the one of the higher contrast with it.
pub(crate) fn readable_on(background: Color, first: Color, second: Color) -> Color {
    let contrast = |color: Color| {
        let (one, other) = (luminance(color), luminance(background));
        (one.max(other) + 0.05) / (one.min(other) + 0.05)
    };
    if contrast(first) >= contrast(second) {
        first
    } else {
        second
    }
}

/// How light a color looks, from 0 to 1, as the Web Content Accessibility Guidelines 2.1 weigh its channels.
fn luminance(color: Color) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
}
