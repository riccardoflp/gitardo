//! Buttons: the main button of a place in the accent, the other buttons on the sunken tone, buttons drawn as their
//! outline, pills and buttons that are only an icon, each with an optional dot, icon or icon of a file before its words, a chevron
//! after them for a button that opens a menu, and a tooltip that finishes the thought of its label.

use std::hash::Hash;

use render::{Border, Color, FileOrFolder, Icon, Point, Rect, TextRun, Theme};

use super::{Availability, is_high_contrast, metrics};
use crate::access::{AccessRole, Description};
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
    /// The other buttons, on the sunken tone of the ground.
    #[default]
    Secondary,
    /// A button drawn as its outline, for the actions of a card such as the inspector.
    Outline,
    /// An outlined button with round ends, on the tone of cards, such as the repository in the header.
    Pill,
    /// A button that is only an icon, or words, with no background until the pointer is over it.
    Icon,
}

/// A button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button<'text> {
    /// The words on it; none for a button that is only an icon.
    pub label: &'text str,
    /// The icon before the words, or the only icon.
    pub icon: Option<Icon>,
    /// A dot of a color before the words, such as the color of a repository or of the lane of a branch.
    pub dot: Option<Color>,
    /// The icon of a file or a folder before the words, after the icon (`file-view.file-icons`).
    pub file_icon: Option<FileOrFolder<'text>>,
    /// The icon after the words, such as the chevron of a button that opens a menu.
    pub trailing: Option<Icon>,
    /// The keys that do what it does, in a small box after its words, such as `Ctrl K`.
    pub key_hint: Option<&'text str>,
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
            dot: None,
            file_icon: None,
            trailing: None,
            key_hint: None,
            kind: ButtonKind::Primary,
            availability: Availability::Enabled,
            tooltip: None,
        }
    }

    /// Another button, with words.
    #[must_use]
    pub fn secondary(label: &'text str) -> Button<'text> {
        Button::primary(label).kind(ButtonKind::Secondary)
    }

    /// A button drawn as its outline, with words.
    #[must_use]
    pub fn outline(label: &'text str) -> Button<'text> {
        Button::primary(label).kind(ButtonKind::Outline)
    }

    /// A button that is only an icon, with the tooltip that says what it does.
    #[must_use]
    pub fn icon(icon: Icon, tooltip: &'text str) -> Button<'text> {
        Button {
            icon: Some(icon),
            tooltip: Some(tooltip),
            ..Button::primary("").kind(ButtonKind::Icon)
        }
    }

    /// The main button of a place that does something that cannot be undone, with words.
    #[must_use]
    pub fn danger(label: &'text str) -> Button<'text> {
        Button::primary(label).kind(ButtonKind::Danger)
    }

    /// The same button drawn as another kind.
    #[must_use]
    pub fn kind(self, kind: ButtonKind) -> Button<'text> {
        Button { kind, ..self }
    }

    /// The same button with an icon before its words.
    #[must_use]
    pub fn with_icon(self, icon: Icon) -> Button<'text> {
        Button {
            icon: Some(icon),
            ..self
        }
    }

    /// The same button with the icon of a file or a folder before its words, after its icon.
    #[must_use]
    pub fn with_file_icon(self, asked: FileOrFolder<'text>) -> Button<'text> {
        Button {
            file_icon: Some(asked),
            ..self
        }
    }

    /// The same button with a dot of a color before its words.
    #[must_use]
    pub fn with_dot(self, color: Color) -> Button<'text> {
        Button {
            dot: Some(color),
            ..self
        }
    }

    /// The same button with an icon after its words.
    #[must_use]
    pub fn with_trailing(self, icon: Icon) -> Button<'text> {
        Button {
            trailing: Some(icon),
            ..self
        }
    }

    /// The same button with the keys that do what it does after its words.
    #[must_use]
    pub fn with_key_hint(self, keys: &'text str) -> Button<'text> {
        Button {
            key_hint: Some(keys),
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

    /// The kind of text of its words: the main buttons stand out.
    fn text_kind(&self) -> TextKind {
        match self.kind {
            ButtonKind::Primary | ButtonKind::Danger | ButtonKind::Pill => TextKind::Strong,
            ButtonKind::Secondary | ButtonKind::Outline | ButtonKind::Icon => TextKind::Body,
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

/// The widths of the parts of a button, from its left: the dot, the icon, the words and the icon after them.
struct Parts {
    /// The dot, with the gap after it.
    dot: f32,
    /// The icon before the words, with the gap after it when words follow.
    icon: f32,
    /// The icon of a file or a folder, with the gap after it when words follow.
    file_icon: f32,
    /// The words.
    words: f32,
    /// The icon after the words, with the gap before it.
    trailing: f32,
    /// The box of the keys, with the gap before it.
    key_hint: f32,
}

impl Parts {
    /// The width of all the parts.
    fn width(&self) -> f32 {
        self.dot + self.icon + self.file_icon + self.words + self.key_hint + self.trailing
    }
}

impl Ui<'_> {
    /// The size a button takes: its parts with room around them, and the height of a control.
    #[must_use]
    pub fn button_size(&self, button: &Button<'_>) -> (f32, f32) {
        let theme = self.theme();
        let height = metrics::control_height(theme);
        if button.label.is_empty() && button.dot.is_none() && button.trailing.is_none() {
            return (height, height);
        }
        (
            self.button_parts(button).width() + 2.0 * theme.sizes.spacing.large,
            height,
        )
    }

    /// The widths of the parts of a button.
    fn button_parts(&self, button: &Button<'_>) -> Parts {
        let theme = self.theme();
        let gap = theme.sizes.spacing.medium;
        let has_words = !button.label.is_empty();
        let words = if has_words {
            self.measure(button.label, self.text_style(button.text_kind()))
        } else {
            0.0
        };
        let then = |width: f32| width + if has_words { gap } else { 0.0 };
        Parts {
            dot: button.dot.map_or(0.0, |_| then(dot_side(theme))),
            icon: button.icon.map_or(0.0, |_| then(theme.sizes.icon)),
            file_icon: button.file_icon.map_or(0.0, |_| then(theme.sizes.icon)),
            words,
            trailing: button.trailing.map_or(0.0, |_| trailing_side(theme) + gap * 0.5),
            key_hint: button.key_hint.map_or(0.0, |keys| self.key_hint_width(keys) + gap),
        }
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
        let theme = *self.theme();
        let look = look(&theme, button, &response);
        let radius = match button.kind {
            ButtonKind::Pill => rect.height * 0.5,
            _ => theme.sizes.radius.large,
        };
        self.fill_rounded_rect(rect, radius, look.background);
        if let Some(color) = look.border {
            let width = theme.sizes.border;
            self.border(rect, Border { width, radius, color });
        }
        self.button_content(rect, button, look.foreground);
        if response.shows_focus {
            self.focus_ring(rect, radius);
        }
        if let Some(tooltip) = button.tooltip {
            self.tooltip(&response, tooltip);
        }
        let name = if button.label.is_empty() {
            button.tooltip.unwrap_or_default()
        } else {
            button.label
        };
        let description = Description::new(AccessRole::Button, name).disabled(!is_enabled);
        self.access_node(id, rect, &description);
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

    /// Draws the dot, the icon, the words and the icon after them in the middle of a button.
    fn button_content(&mut self, rect: Rect, button: &Button<'_>, color: Color) {
        let theme = *self.theme();
        let parts = self.button_parts(button);
        let content = place(
            rect.inset(theme.sizes.spacing.small),
            (parts.width(), rect.height),
            (Align::Center, Align::Start),
        );
        let middle = |side: f32, x: f32| {
            place(
                Rect::new(x, rect.y, side, rect.height),
                (side, side),
                (Align::Start, Align::Center),
            )
        };
        let mut x = content.x;
        if let Some(dot) = button.dot {
            let side = dot_side(&theme);
            self.fill_circle(middle(side, x).center(), side * 0.5, dot);
            x += parts.dot;
        }
        if let Some(icon) = button.icon {
            let at = middle(theme.sizes.icon, x);
            self.icon(icon, Point::new(at.x, at.y), theme.sizes.icon, color);
            x += parts.icon;
        }
        if let Some(asked) = button.file_icon {
            let at = middle(theme.sizes.icon, x);
            self.file_icon(asked, Point::new(at.x, at.y), theme.sizes.icon, color);
            x += parts.file_icon;
        }
        if !button.label.is_empty() {
            let style = self.text_style(button.text_kind());
            let line = place(rect, (0.0, self.line_height(style)), (Align::Start, Align::Center));
            let room = content.right() - x - parts.key_hint - parts.trailing;
            let run = TextRun {
                text: button.label,
                style,
                color,
                origin: Point::new(x, line.y),
                max_width: Some(room.max(0.0)),
            };
            x += self.text(&run);
        }
        if let Some(keys) = button.key_hint {
            let hint = Rect::new(
                x + theme.sizes.spacing.medium,
                rect.y,
                parts.key_hint - theme.sizes.spacing.medium,
                rect.height,
            );
            self.key_hint(hint, keys);
            x += parts.key_hint;
        }
        if let Some(icon) = button.trailing {
            let side = trailing_side(&theme);
            let at = middle(side, x + parts.trailing - side);
            let muted = theme.colors.muted_foreground;
            let tint = if button.kind == ButtonKind::Pill { muted } else { color };
            self.icon(icon, Point::new(at.x, at.y), side, tint);
        }
    }
}

/// The side of the dot before the words of a button.
fn dot_side(theme: &Theme) -> f32 {
    theme.sizes.spacing.medium + theme.sizes.spacing.tiny
}

/// The side of the icon after the words of a button: smaller than the icon before them.
fn trailing_side(theme: &Theme) -> f32 {
    theme.sizes.icon * 0.75
}

/// The colors of a button, by its kind and whether it is enabled, under the pointer or pressed.
fn look(theme: &Theme, button: &Button<'_>, response: &Response) -> Look {
    let colors = &theme.colors;
    let is_lit = response.hovered || response.pressed;
    let high_contrast = is_high_contrast(theme).then_some(colors.border);
    if button.availability == Availability::Disabled {
        let background = match button.kind {
            ButtonKind::Icon | ButtonKind::Outline => Color::TRANSPARENT,
            ButtonKind::Pill => colors.card_background,
            ButtonKind::Primary | ButtonKind::Danger | ButtonKind::Secondary => colors.secondary_button_background,
        };
        let outlined = matches!(button.kind, ButtonKind::Outline | ButtonKind::Pill);
        return Look {
            background,
            foreground: colors.disabled_foreground,
            border: (outlined || high_contrast.is_some()).then_some(colors.border),
        };
    }
    let lit = |lit: Color, plain: Color| if is_lit { lit } else { plain };
    match button.kind {
        ButtonKind::Primary => Look {
            background: lit(colors.button_hover, colors.button_background),
            foreground: colors.button_foreground,
            border: high_contrast,
        },
        ButtonKind::Danger => Look {
            background: lit(
                colors.error.with_alpha(DANGER_LIT).over(colors.page_background),
                colors.error,
            ),
            foreground: readable_on(colors.error, colors.button_foreground, colors.window_background),
            border: high_contrast,
        },
        ButtonKind::Secondary => Look {
            background: lit(colors.secondary_button_hover, colors.secondary_button_background),
            foreground: colors.secondary_button_foreground,
            border: high_contrast,
        },
        ButtonKind::Outline => Look {
            background: lit(colors.list_hover, Color::TRANSPARENT),
            foreground: colors.foreground,
            border: Some(colors.border),
        },
        ButtonKind::Pill => Look {
            background: lit(colors.list_hover, colors.card_background),
            foreground: colors.foreground,
            border: Some(colors.border),
        },
        ButtonKind::Icon => Look {
            background: lit(colors.list_hover, Color::TRANSPARENT),
            foreground: colors.foreground,
            border: high_contrast.filter(|_| is_lit),
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
