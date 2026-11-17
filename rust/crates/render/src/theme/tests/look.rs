//! The hues of the dark and of the light theme against what the look allows (D17): a calm ground and its steps, one
//! accent, and the colors of the states where they say that state. The lanes after the first, the colors of the code,
//! of a terminal and of the icons of files are hues of their own by what they are for.

use super::selection::{has_the_hue_of_the_accent_or_none, tint};
use crate::color::Color;
use crate::theme::{Appearance, Colors, Theme, ThemeName};

/// How far the three channels of a color may move apart from those of the ground of the page, out of 255, and still
/// be a step of the ground or a text with no hue of its own: the warm text on the graphite of the dark theme and the
/// white of a card on the paper of the light one are within 9.
const A_STEP: i16 = 10;

/// Whether a color drawn on the page is a step of its ground, or has the hue of the accent.
fn is_a_step_or_the_accent(color: Color, colors: &Colors) -> bool {
    let ground = colors.page_background;
    let [red, green, blue] = tint(color.over(ground), ground);
    let spread = red.max(green).max(blue) - red.min(green).min(blue);
    spread <= A_STEP || has_the_hue_of_the_accent_or_none(color.over(ground), ground, colors.accent)
}

/// The colors that are the ground, its steps, the text and the lines on it, and what wears the accent: every color of
/// a theme but those of the states. A new color of the themes is added here or to `states`.
fn grounds_and_accents(colors: &Colors) -> [(Color, &'static str); 41] {
    [
        (colors.window_background, "window_background"),
        (colors.ground_background, "ground_background"),
        (colors.header_background, "header_background"),
        (colors.rail_background, "rail_background"),
        (colors.page_background, "page_background"),
        (colors.card_background, "card_background"),
        (colors.drawer_background, "drawer_background"),
        (colors.foreground, "foreground"),
        (colors.muted_foreground, "muted_foreground"),
        (colors.disabled_foreground, "disabled_foreground"),
        (colors.list_hover, "list_hover"),
        (colors.list_selection, "list_selection"),
        (colors.list_selection_foreground, "list_selection_foreground"),
        (colors.list_inactive_selection, "list_inactive_selection"),
        (colors.input_background, "input_background"),
        (colors.input_foreground, "input_foreground"),
        (colors.input_placeholder, "input_placeholder"),
        (colors.input_border, "input_border"),
        (colors.button_background, "button_background"),
        (colors.button_hover, "button_hover"),
        (colors.button_foreground, "button_foreground"),
        (colors.secondary_button_background, "secondary_button_background"),
        (colors.secondary_button_hover, "secondary_button_hover"),
        (colors.secondary_button_foreground, "secondary_button_foreground"),
        (colors.border, "border"),
        (colors.focus_ring, "focus_ring"),
        (colors.accent, "accent"),
        (colors.badge_background, "badge_background"),
        (colors.badge_foreground, "badge_foreground"),
        (colors.scrollbar_thumb, "scrollbar_thumb"),
        (colors.scrollbar_thumb_hover, "scrollbar_thumb_hover"),
        (colors.menu_background, "menu_background"),
        (colors.menu_foreground, "menu_foreground"),
        (colors.menu_selection, "menu_selection"),
        (colors.menu_selection_foreground, "menu_selection_foreground"),
        (colors.tooltip_background, "tooltip_background"),
        (colors.tooltip_foreground, "tooltip_foreground"),
        (colors.close_hover_foreground, "close_hover_foreground"),
        (colors.shadow, "shadow"),
        (colors.text_selection, "text_selection"),
        (colors.find_match, "find_match"),
    ]
}

/// The colors of the states: an error, a warning, information, success, a conflict, and the red behind **Close**.
fn states(colors: &Colors) -> [(Color, &'static str); 6] {
    [
        (colors.error, "error"),
        (colors.warning, "warning"),
        (colors.info, "info"),
        (colors.success, "success"),
        (colors.conflict, "conflict"),
        (colors.close_hover, "close_hover"),
    ]
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__every_color_of_the_dark_and_the_light_theme_is_a_step_of_the_ground_the_accent_or_that_of_a_state() {
    for name in [ThemeName::Dark, ThemeName::Light] {
        let theme = Theme::new(name, Appearance::Dark);
        let colors = theme.colors;
        for (color, what) in grounds_and_accents(&colors) {
            assert!(
                is_a_step_or_the_accent(color, &colors),
                "{name:?}: {what} has a hue that is neither the ground's nor the accent's"
            );
        }
        // The colors of the states are hues of their own, each told from the ground.
        for (color, what) in states(&colors) {
            let [red, green, blue] = tint(color, colors.page_background);
            let spread = red.max(green).max(blue) - red.min(green).min(blue);
            assert!(spread > A_STEP, "{name:?}: {what} is a step of the ground");
        }
        // What a file or a line became wears the color of the state it is like, and its ground a wash of it.
        assert_eq!(colors.added, colors.success, "{name:?}: added");
        assert_eq!(colors.removed, colors.error, "{name:?}: removed");
        assert_eq!(colors.modified, colors.warning, "{name:?}: modified");
        let opaque = |color: Color| color.with_alpha(u8::MAX);
        assert_eq!(
            opaque(colors.added_background),
            colors.added,
            "{name:?}: added_background"
        );
        assert_eq!(
            opaque(colors.removed_background),
            colors.removed,
            "{name:?}: removed_background"
        );
        // The buttons, the ring of the focus, a match of a search and the first lane wear the one accent.
        assert_eq!(colors.button_background, colors.accent, "{name:?}: button_background");
        assert_eq!(colors.focus_ring, colors.accent, "{name:?}: focus_ring");
        assert_eq!(opaque(colors.find_match), colors.accent, "{name:?}: find_match");
        assert_eq!(theme.lanes.first(), Some(&colors.accent), "{name:?}: the first lane");
    }
}
