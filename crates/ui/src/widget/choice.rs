//! Choices: check boxes, radio buttons and toggle switches, each a mark followed by its label. The view holds whether
//! each is on, and turns it when the widget says it was clicked.

use std::hash::Hash;

use render::{Border, Icon, Point, Rect, Theme};

use super::label::Label;
use super::{Availability, Tone, metrics};
use crate::frame::Ui;
use crate::interaction::{Response, Sense};
use crate::layout::{Align, place};

/// What kind of mark a choice has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChoiceKind {
    /// A square with a check mark: one of several things that can each be on.
    CheckBox,
    /// A circle with a dot: one of several choices of which one is on.
    Radio,
    /// A switch: a setting that applies at once.
    Toggle,
}

/// A check box, a radio button or a toggle switch, with its label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Choice<'text> {
    /// What kind of mark it has.
    pub kind: ChoiceKind,
    /// The words after the mark.
    pub label: &'text str,
    /// Whether it is on.
    pub is_on: bool,
    /// Whether it can be used now.
    pub availability: Availability,
}

impl<'text> Choice<'text> {
    /// A check box.
    #[must_use]
    pub fn check_box(label: &'text str, is_on: bool) -> Choice<'text> {
        Choice {
            kind: ChoiceKind::CheckBox,
            label,
            is_on,
            availability: Availability::Enabled,
        }
    }

    /// A radio button.
    #[must_use]
    pub fn radio(label: &'text str, is_on: bool) -> Choice<'text> {
        Choice {
            kind: ChoiceKind::Radio,
            ..Choice::check_box(label, is_on)
        }
    }

    /// A toggle switch.
    #[must_use]
    pub fn toggle(label: &'text str, is_on: bool) -> Choice<'text> {
        Choice {
            kind: ChoiceKind::Toggle,
            ..Choice::check_box(label, is_on)
        }
    }

    /// The same choice, disabled.
    #[must_use]
    pub fn disabled(self) -> Choice<'text> {
        Choice {
            availability: Availability::Disabled,
            ..self
        }
    }
}

impl Ui<'_> {
    /// Draws a check box, a radio button or a toggle switch with its label, and says whether it was clicked: on the
    /// mark or on the label, or with Space while it has the focus.
    pub fn choice(&mut self, key: impl Hash, rect: Rect, choice: &Choice<'_>) -> Response {
        let id = self.id(key);
        let is_enabled = choice.availability == Availability::Enabled;
        let sense = if is_enabled {
            Sense::CLICK | Sense::FOCUS
        } else {
            Sense::HOVER
        };
        let response = self.interact(id, rect, sense);
        let theme = *self.theme();
        let mark_width = match choice.kind {
            ChoiceKind::CheckBox | ChoiceKind::Radio => metrics::mark_side(&theme),
            ChoiceKind::Toggle => metrics::toggle_width(&theme),
        };
        let side = metrics::mark_side(&theme);
        let mark = place(rect, (mark_width, side), (Align::Start, Align::Center));
        let state = MarkState {
            is_on: choice.is_on,
            is_enabled,
            is_lit: is_enabled && (response.hovered || response.pressed),
        };
        match choice.kind {
            ChoiceKind::CheckBox => self.check_mark(mark, state),
            ChoiceKind::Radio => self.radio_mark(mark, state),
            ChoiceKind::Toggle => self.toggle_mark(mark, state),
        }
        let gap = theme.sizes.spacing.medium;
        let text = Rect::new(
            mark.right() + gap,
            rect.y,
            (rect.right() - mark.right() - gap).max(0.0),
            rect.height,
        );
        let tone = if is_enabled { Tone::Normal } else { Tone::Disabled };
        let label = Label {
            has_tooltip_when_cut: false,
            ..Label::new(choice.label).tone(tone)
        };
        let _ = self.label(text, &label);
        if response.shows_focus {
            let radius = theme.sizes.radius.small;
            self.focus_ring(mark.inset(-theme.sizes.spacing.tiny), radius);
        }
        response
    }

    /// The square of a check box, filled with a check mark when on.
    fn check_mark(&mut self, mark: Rect, state: MarkState) {
        let theme = *self.theme();
        let radius = theme.sizes.radius.small;
        let colors = mark_colors(&theme, state);
        self.fill_rounded_rect(mark, radius, colors.fill);
        if !state.is_on || colors.edge != colors.fill {
            let width = theme.sizes.border;
            self.border(
                mark,
                Border {
                    width,
                    radius,
                    color: colors.edge,
                },
            );
        }
        if state.is_on {
            self.icon(Icon::Check, Point::new(mark.x, mark.y), mark.height, colors.dot);
        }
    }

    /// The circle of a radio button, with a dot when on.
    fn radio_mark(&mut self, mark: Rect, state: MarkState) {
        let theme = *self.theme();
        let colors = mark_colors(&theme, state);
        let center = mark.center();
        let radius = mark.height * 0.5;
        self.fill_circle(center, radius, colors.edge);
        self.fill_circle(center, radius - theme.sizes.border, colors.fill);
        if state.is_on {
            self.fill_circle(center, radius * 0.4, colors.dot);
        }
    }

    /// The track of a toggle switch, with its knob at the right when on and at the left when off.
    fn toggle_mark(&mut self, mark: Rect, state: MarkState) {
        let theme = *self.theme();
        let colors = mark_colors(&theme, state);
        let radius = mark.height * 0.5;
        self.fill_rounded_rect(mark, radius, colors.edge);
        self.fill_rounded_rect(mark.inset(theme.sizes.border), radius, colors.fill);
        let inset = theme.sizes.spacing.tiny + theme.sizes.border;
        let knob_radius = radius - inset;
        let x = if state.is_on {
            mark.right() - radius
        } else {
            mark.x + radius
        };
        self.fill_circle(Point::new(x, mark.center().y), knob_radius, colors.dot);
    }
}

/// Whether a mark is on, can be used, and is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MarkState {
    /// On.
    is_on: bool,
    /// Can be used.
    is_enabled: bool,
    /// Under the pointer or pressed.
    is_lit: bool,
}

/// The colors of a mark: its edge, what fills it, and its check, dot or knob.
struct MarkColors {
    /// The edge.
    edge: render::Color,
    /// Inside the edge.
    fill: render::Color,
    /// The check mark, the dot or the knob.
    dot: render::Color,
}

/// The colors of a mark in a state.
fn mark_colors(theme: &Theme, state: MarkState) -> MarkColors {
    let colors = &theme.colors;
    if !state.is_enabled {
        return MarkColors {
            edge: colors.disabled_foreground,
            fill: colors.input_background,
            dot: colors.disabled_foreground,
        };
    }
    match (state.is_on, state.is_lit) {
        (true, false) => MarkColors {
            edge: colors.button_background,
            fill: colors.button_background,
            dot: colors.button_foreground,
        },
        (true, true) => MarkColors {
            edge: colors.button_hover,
            fill: colors.button_hover,
            dot: colors.button_foreground,
        },
        (false, is_lit) => MarkColors {
            edge: if is_lit { colors.focus_ring } else { colors.input_border },
            fill: colors.input_background,
            dot: colors.muted_foreground,
        },
    }
}
