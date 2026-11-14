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
    /// The ground the islands of the window stand on, between them and around them (D21): a step darker than the
    /// islands, or the same in the high contrast themes, where their border tells them apart.
    pub ground_background: Color,
    /// Behind the header: the repository, the branch, Quick search, Sync.
    pub header_background: Color,
    /// Behind the branch rail on the left.
    pub rail_background: Color,
    /// Behind the timeline and the pages opened in its place.
    pub page_background: Color,
    /// Behind a card: the inspector, the draft, a group of settings.
    pub card_background: Color,
    /// Behind the Activity drawer.
    pub drawer_background: Color,
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
    /// What stands out: the current branch, links, the main button, the last step of the trail.
    pub accent: Color,
    /// Errors.
    pub error: Color,
    /// Warnings.
    pub warning: Color,
    /// Information.
    pub info: Color,
    /// What went well.
    pub success: Color,
    /// Behind a count or a pill on a row, such as how many files a commit changed.
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

/// What a token of code is, which the theme gives a color (`file-view.highlight`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeKind {
    /// A comment.
    Comment,
    /// A string, or a character.
    String,
    /// A number.
    Number,
    /// A keyword of the language.
    Keyword,
    /// An operator.
    Operator,
    /// Punctuation.
    Punctuation,
    /// The name of a type.
    Type,
    /// The name of a function.
    Function,
    /// The name of a variable.
    Variable,
    /// The name of a property, a field or a key.
    Property,
    /// A constant.
    Constant,
    /// The name of a tag of markup.
    Tag,
    /// The name of an attribute, or a decorator.
    Attribute,
    /// Anything else, in the color of text.
    Plain,
}

/// The colors of the tokens of code, by their kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeColors {
    /// Comments.
    pub comment: Color,
    /// Strings and characters.
    pub string: Color,
    /// Numbers.
    pub number: Color,
    /// Keywords.
    pub keyword: Color,
    /// Operators.
    pub operator: Color,
    /// Punctuation.
    pub punctuation: Color,
    /// The names of types.
    pub type_name: Color,
    /// The names of functions.
    pub function: Color,
    /// The names of variables.
    pub variable: Color,
    /// The names of properties, fields and keys.
    pub property: Color,
    /// Constants.
    pub constant: Color,
    /// The names of tags.
    pub tag: Color,
    /// The names of attributes, and decorators.
    pub attribute: Color,
}

impl CodeColors {
    /// The color of a kind of token; plain text is drawn in the color given for it.
    #[must_use]
    pub fn color(&self, kind: CodeKind, plain: Color) -> Color {
        match kind {
            CodeKind::Comment => self.comment,
            CodeKind::String => self.string,
            CodeKind::Number => self.number,
            CodeKind::Keyword => self.keyword,
            CodeKind::Operator => self.operator,
            CodeKind::Punctuation => self.punctuation,
            CodeKind::Type => self.type_name,
            CodeKind::Function => self.function,
            CodeKind::Variable => self.variable,
            CodeKind::Property => self.property,
            CodeKind::Constant => self.constant,
            CodeKind::Tag => self.tag,
            CodeKind::Attribute => self.attribute,
            CodeKind::Plain => plain,
        }
    }
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
    /// The second line of a row and what is said beside a control: the author and the date of a commit, its hash, a
    /// count, what runs.
    pub detail: f32,
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
    /// The height of buttons, input boxes and the other controls on a line.
    pub control_height: f32,
    /// The side of an icon.
    pub icon: f32,
    /// The width of borders.
    pub border: f32,
    /// The width of the ring around what has the focus.
    pub focus_ring: f32,
    /// The width of a scroll bar.
    pub scrollbar: f32,
    /// The height of the header.
    pub header_height: f32,
    /// The height of a row of the timeline: the subject on one line, the author and the date on the next.
    pub timeline_row_height: f32,
    /// The width of the branch rail.
    pub rail_width: f32,
    /// The width of the inspector.
    pub inspector_width: f32,
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
    /// The colors of the lanes of the graph, used in turn: four, told apart by lightness as well as hue.
    pub lanes: [Color; 4],
    /// The colors of the tokens of code.
    pub code: CodeColors,
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
        small: 4.0,
        medium: 6.0,
        large: 10.0,
    },
    font: FontSizes {
        small: 11.0,
        body: 13.0,
        detail: 12.0,
        code: 13.0,
        heading: 18.0,
    },
    row_height: 26.0,
    control_height: 32.0,
    icon: 16.0,
    border: 1.0,
    focus_ring: 1.0,
    scrollbar: 10.0,
    header_height: 56.0,
    timeline_row_height: 52.0,
    rail_width: 248.0,
    inspector_width: 344.0,
    graph_line: 2.0,
    graph_dot: 5.0,
};

#[cfg(test)]
mod tests;
