//! Themes: the colors and sizes every widget draws with, named by what they are for, never by what color they are,
//! so that a view says `list_selection` and each theme decides what that looks like (R10.3).

mod contrast;
mod dark;
mod light;

use crate::color::Color;

/// The theme the user chose, as the setting `theme` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeName {
    /// Dark or light as the system is, following it when it changes.
    #[default]
    System,
    /// Light text on dark backgrounds.
    Dark,
    /// Dark text on light backgrounds.
    Light,
    /// White text on black, with strong borders.
    HighContrastDark,
    /// Black text on white, with strong borders.
    HighContrastLight,
}

impl ThemeName {
    /// The theme a value of the setting `theme` names, or nothing for a value it does not know.
    #[must_use]
    pub fn from_setting(value: &str) -> Option<ThemeName> {
        match value {
            "system" => Some(ThemeName::System),
            "dark" => Some(ThemeName::Dark),
            "light" => Some(ThemeName::Light),
            "highContrastDark" => Some(ThemeName::HighContrastDark),
            "highContrastLight" => Some(ThemeName::HighContrastLight),
            _ => None,
        }
    }

    /// The value of the setting `theme` for this theme.
    #[must_use]
    pub fn setting(self) -> &'static str {
        match self {
            ThemeName::System => "system",
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::HighContrastDark => "highContrastDark",
            ThemeName::HighContrastLight => "highContrastLight",
        }
    }
}

/// Whether the system shows dark or light windows, which the platform reads, for the theme `system`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Appearance {
    /// Dark windows.
    #[default]
    Dark,
    /// Light windows.
    Light,
}

/// The colors of a theme, each named by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    /// Behind everything: the window where nothing else is.
    pub window_background: Color,
    /// Behind the panes of the side bar.
    pub side_bar_background: Color,
    /// Behind the pages of the tabs, such as the graph.
    pub page_background: Color,
    /// Behind the panel of the command log and the tasks.
    pub panel_background: Color,
    /// Behind the toolbar.
    pub toolbar_background: Color,
    /// Behind the status bar.
    pub status_bar_background: Color,
    /// The text of the status bar.
    pub status_bar_foreground: Color,
    /// Behind a tab that is not the one shown.
    pub tab_background: Color,
    /// Behind the tab shown.
    pub tab_active_background: Color,
    /// The text of a tab that is not the one shown.
    pub tab_foreground: Color,
    /// The text of the tab shown.
    pub tab_active_foreground: Color,
    /// The text of the interface.
    pub foreground: Color,
    /// Text that says less: descriptions, dates, hashes, placeholders.
    pub muted_foreground: Color,
    /// The text of what cannot be used now.
    pub disabled_foreground: Color,
    /// Behind a row of a list under the pointer.
    pub list_hover: Color,
    /// Behind the selected rows of a list that has the focus.
    pub list_selection: Color,
    /// The text of the selected rows of a list that has the focus.
    pub list_selection_foreground: Color,
    /// Behind the selected rows of a list without the focus.
    pub list_inactive_selection: Color,
    /// Behind an input box.
    pub input_background: Color,
    /// The text typed in an input box.
    pub input_foreground: Color,
    /// The text an empty input box shows.
    pub input_placeholder: Color,
    /// The border of an input box.
    pub input_border: Color,
    /// Behind the main button of a place.
    pub button_background: Color,
    /// Behind the main button under the pointer.
    pub button_hover: Color,
    /// The text of the main button.
    pub button_foreground: Color,
    /// Behind the other buttons.
    pub secondary_button_background: Color,
    /// Behind the other buttons under the pointer.
    pub secondary_button_hover: Color,
    /// The text of the other buttons.
    pub secondary_button_foreground: Color,
    /// The lines between parts of the window, and around boxes.
    pub border: Color,
    /// The ring around what has the keyboard focus.
    pub focus_ring: Color,
    /// What stands out: the current branch, links, the active tab's mark.
    pub accent: Color,
    /// Errors.
    pub error: Color,
    /// Warnings.
    pub warning: Color,
    /// Information.
    pub info: Color,
    /// What went well.
    pub success: Color,
    /// Behind a count or a label on a row, such as a branch name in the graph.
    pub badge_background: Color,
    /// The text of a badge.
    pub badge_foreground: Color,
    /// The part of a scroll bar that moves.
    pub scrollbar_thumb: Color,
    /// The part of a scroll bar that moves, under the pointer or dragged.
    pub scrollbar_thumb_hover: Color,
    /// Behind menus, pickers and dialogs.
    pub menu_background: Color,
    /// The text of menus.
    pub menu_foreground: Color,
    /// Behind the item of a menu under the pointer or the keys.
    pub menu_selection: Color,
    /// The text of that item.
    pub menu_selection_foreground: Color,
    /// Behind a tooltip.
    pub tooltip_background: Color,
    /// The text of a tooltip.
    pub tooltip_foreground: Color,
    /// The shadow under menus and dialogs, usually see-through.
    pub shadow: Color,
    /// Behind text selected in an editor or a box.
    pub text_selection: Color,
    /// Behind what find in page found.
    pub find_match: Color,
    /// Lines and files added.
    pub added: Color,
    /// Lines and files removed.
    pub removed: Color,
    /// Files changed.
    pub modified: Color,
    /// Files in conflict.
    pub conflict: Color,
    /// Behind lines added, in a diff.
    pub added_background: Color,
    /// Behind lines removed, in a diff.
    pub removed_background: Color,
}

/// The distances between things, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    /// Between an icon and its text.
    pub tiny: f32,
    /// Inside small widgets, such as a badge.
    pub small: f32,
    /// Inside buttons and rows, and between widgets.
    pub medium: f32,
    /// Between groups of widgets.
    pub large: f32,
    /// Around a page or a dialog.
    pub huge: f32,
}

/// The radii of rounded corners, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radii {
    /// Badges and small boxes.
    pub small: f32,
    /// Buttons and input boxes.
    pub medium: f32,
    /// Menus, dialogs and tooltips.
    pub large: f32,
}

/// The sizes of text, in logical pixels, before the setting `font.size` scales them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontSizes {
    /// Badges and small print.
    pub small: f32,
    /// The text of the interface.
    pub body: f32,
    /// Code, diffs and the command log.
    pub code: f32,
    /// The titles of pages and dialogs.
    pub heading: f32,
}

/// The sizes of a theme, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    /// The distances between things.
    pub spacing: Spacing,
    /// The radii of rounded corners.
    pub radius: Radii,
    /// The sizes of text.
    pub font: FontSizes,
    /// The height of a row of a list or a tree.
    pub row_height: f32,
    /// The side of an icon.
    pub icon: f32,
    /// The width of borders.
    pub border: f32,
    /// The width of the ring around what has the focus.
    pub focus_ring: f32,
    /// The width of a scroll bar.
    pub scrollbar: f32,
    /// The height of the toolbar and of the row of tabs.
    pub bar_height: f32,
    /// The height of the status bar.
    pub status_bar_height: f32,
    /// The width of the lines of the graph.
    pub graph_line: f32,
    /// The radius of the dot of a commit in the graph.
    pub graph_dot: f32,
}

/// The colors and sizes everything is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// The theme this is, `system` already resolved to dark or light.
    pub name: ThemeName,
    /// The colors.
    pub colors: Colors,
    /// The sizes.
    pub sizes: Sizes,
    /// The colors of the lanes of the graph, used in turn.
    pub lanes: [Color; 8],
}

impl Theme {
    /// The theme of a name; `system` becomes the dark or the light theme as the system's appearance is.
    #[must_use]
    pub fn new(name: ThemeName, appearance: Appearance) -> Theme {
        match (name, appearance) {
            (ThemeName::Dark, _) | (ThemeName::System, Appearance::Dark) => dark::THEME,
            (ThemeName::Light, _) | (ThemeName::System, Appearance::Light) => light::THEME,
            (ThemeName::HighContrastDark, _) => contrast::DARK,
            (ThemeName::HighContrastLight, _) => contrast::LIGHT,
        }
    }

    /// The color of a lane of the graph: the lanes take the colors in turn.
    #[must_use]
    pub fn lane(&self, lane: usize) -> Color {
        self.lanes
            .get(lane % self.lanes.len())
            .copied()
            .unwrap_or(self.colors.accent)
    }
}

/// The sizes of the themes that are not high contrast.
pub(crate) const SIZES: Sizes = Sizes {
    spacing: Spacing {
        tiny: 2.0,
        small: 4.0,
        medium: 8.0,
        large: 12.0,
        huge: 20.0,
    },
    radius: Radii {
        small: 3.0,
        medium: 4.0,
        large: 6.0,
    },
    font: FontSizes {
        small: 11.0,
        body: 13.0,
        code: 13.0,
        heading: 18.0,
    },
    row_height: 22.0,
    icon: 16.0,
    border: 1.0,
    focus_ring: 1.0,
    scrollbar: 10.0,
    bar_height: 34.0,
    status_bar_height: 22.0,
    graph_line: 1.5,
    graph_dot: 4.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_name_the_five_themes_and_system_follows_the_appearance() {
        for name in [
            ThemeName::System,
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::HighContrastDark,
            ThemeName::HighContrastLight,
        ] {
            assert_eq!(ThemeName::from_setting(name.setting()), Some(name));
        }
        assert_eq!(ThemeName::from_setting("solarized"), None);
        assert_eq!(
            Theme::new(ThemeName::System, Appearance::Light),
            Theme::new(ThemeName::Light, Appearance::Dark)
        );
        assert_eq!(Theme::new(ThemeName::System, Appearance::Dark).name, ThemeName::Dark);
        assert_eq!(
            Theme::new(ThemeName::Dark, Appearance::Dark).lane(9),
            Theme::new(ThemeName::Dark, Appearance::Dark).lanes[1]
        );
    }

    /// The contrast of two colors as the Web Content Accessibility Guidelines 2.1 define it, from 1 to 21.
    fn contrast(first: Color, second: Color) -> f64 {
        let luminance = |color: Color| {
            let channel = |value: u8| {
                let value = f64::from(value) / 255.0;
                if value <= 0.039_28 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
        };
        let (lighter, darker) = (
            luminance(first).max(luminance(second)),
            luminance(first).min(luminance(second)),
        );
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn text_is_readable_on_its_backgrounds_in_every_theme() {
        for name in [
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::HighContrastDark,
            ThemeName::HighContrastLight,
        ] {
            let theme = Theme::new(name, Appearance::Dark);
            let colors = theme.colors;
            let least = if matches!(name, ThemeName::HighContrastDark | ThemeName::HighContrastLight) {
                7.0
            } else {
                4.5
            };
            for (foreground, background, what) in [
                (colors.foreground, colors.page_background, "text on a page"),
                (colors.foreground, colors.side_bar_background, "text in the side bar"),
                (
                    colors.list_selection_foreground,
                    colors.list_selection,
                    "a selected row",
                ),
                (colors.button_foreground, colors.button_background, "a button"),
                (colors.input_foreground, colors.input_background, "an input box"),
                (colors.menu_foreground, colors.menu_background, "a menu"),
                (
                    colors.status_bar_foreground,
                    colors.status_bar_background,
                    "the status bar",
                ),
            ] {
                let ratio = contrast(foreground, background);
                assert!(ratio >= least, "{name:?}: {what} has a contrast of {ratio:.2}");
            }
        }
    }
}
