//! The widgets of the toolkit. Each is a method of [`Ui`](crate::Ui) that takes the rectangle it is laid out in and
//! what it shows, draws itself with the tokens of the theme only, and returns what happened to it.

pub(crate) mod badge;
pub(crate) mod button;
pub(crate) mod choice;
pub(crate) mod dialog;
pub(crate) mod drag;
pub(crate) mod find;
pub(crate) mod input_box;
pub(crate) mod label;
pub(crate) mod lanes;
pub(crate) mod list;
pub(crate) mod menu;
pub(crate) mod panel;
pub(crate) mod picker;
pub(crate) mod progress;
pub(crate) mod scroll;
pub(crate) mod splitter;
pub(crate) mod tooltip;

use render::{Color, Theme, ThemeName};

/// What a piece of text or an icon says by its color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tone {
    /// The text of the interface.
    #[default]
    Normal,
    /// Less important: descriptions, dates, hashes.
    Muted,
    /// What cannot be used now.
    Disabled,
    /// What stands out, such as the current branch.
    Accent,
    /// What went well.
    Success,
    /// A warning.
    Warning,
    /// An error.
    Error,
    /// The color of a lane of the graph, by its number: the lanes take the colors of the theme in turn.
    Lane(usize),
}

impl Tone {
    /// The color of the tone in a theme.
    #[must_use]
    pub fn color(self, theme: &Theme) -> Color {
        let colors = &theme.colors;
        match self {
            Tone::Normal => colors.foreground,
            Tone::Muted => colors.muted_foreground,
            Tone::Disabled => colors.disabled_foreground,
            Tone::Accent => colors.accent,
            Tone::Success => colors.success,
            Tone::Warning => colors.warning,
            Tone::Error => colors.error,
            Tone::Lane(lane) => theme.lane(lane),
        }
    }
}

/// Whether a widget can be used now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Availability {
    /// It reacts to the pointer and the keys.
    #[default]
    Enabled,
    /// It is drawn faded and reacts to nothing but a tooltip.
    Disabled,
}

/// The sizes of widgets, each made from the tokens of the theme.
pub(crate) mod metrics {
    use render::Theme;

    /// The height of buttons, input boxes and other controls on a line.
    pub(crate) fn control_height(theme: &Theme) -> f32 {
        theme.sizes.control_height
    }

    /// The side of the box of a check box and of the circle of a radio button.
    pub(crate) fn mark_side(theme: &Theme) -> f32 {
        theme.sizes.icon
    }

    /// The width of the track of a toggle switch.
    pub(crate) fn toggle_width(theme: &Theme) -> f32 {
        theme.sizes.icon * 2.0
    }

    /// The thickness of a progress bar.
    pub(crate) fn progress_thickness(theme: &Theme) -> f32 {
        theme.sizes.spacing.small
    }

    /// The widest a tooltip grows before its text wraps.
    pub(crate) fn tooltip_width(theme: &Theme) -> f32 {
        theme.sizes.row_height * 16.0
    }

    /// The width of the part of a splitter that takes the pointer: the line and some room each side of it.
    pub(crate) fn splitter_grip(theme: &Theme) -> f32 {
        theme.sizes.border + 2.0 * theme.sizes.spacing.small
    }
}

/// Whether a theme is one of the high contrast ones, which draw a border around every control.
pub(crate) fn is_high_contrast(theme: &Theme) -> bool {
    matches!(theme.name, ThemeName::HighContrastDark | ThemeName::HighContrastLight)
}
