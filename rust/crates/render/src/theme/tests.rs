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
        Theme::new(ThemeName::Dark, Appearance::Dark).lane(5),
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
            (colors.foreground, colors.rail_background, "text in the branch rail"),
            (colors.foreground, colors.card_background, "text on a card"),
            (
                colors.muted_foreground,
                colors.page_background,
                "muted text on the timeline",
            ),
            (colors.muted_foreground, colors.card_background, "muted text on a card"),
            (
                colors.list_selection_foreground,
                colors.list_selection,
                "a selected row",
            ),
            (colors.button_foreground, colors.button_background, "a button"),
            (colors.input_foreground, colors.input_background, "an input box"),
            (colors.menu_foreground, colors.menu_background, "a menu"),
            (
                colors.secondary_button_foreground,
                colors.secondary_button_background,
                "a button that is not the main one",
            ),
        ] {
            let ratio = contrast(foreground, background);
            assert!(ratio >= least, "{name:?}: {what} has a contrast of {ratio:.2}");
        }
    }
}

#[test]
fn code_is_readable_on_the_pages_in_every_theme() {
    for name in [
        ThemeName::Dark,
        ThemeName::Light,
        ThemeName::HighContrastDark,
        ThemeName::HighContrastLight,
    ] {
        let theme = Theme::new(name, Appearance::Dark);
        let least = if matches!(name, ThemeName::HighContrastDark | ThemeName::HighContrastLight) {
            7.0
        } else {
            4.5
        };
        let code = theme.code;
        for (color, kind) in [
            (code.comment, "comments"),
            (code.string, "strings"),
            (code.number, "numbers"),
            (code.keyword, "keywords"),
            (code.operator, "operators"),
            (code.punctuation, "punctuation"),
            (code.type_name, "types"),
            (code.function, "functions"),
            (code.variable, "variables"),
            (code.property, "properties"),
            (code.constant, "constants"),
            (code.tag, "tags"),
            (code.attribute, "attributes"),
        ] {
            let ratio = contrast(color, theme.colors.page_background);
            assert!(
                ratio >= least,
                "{name:?}: the code of {kind} has a contrast of {ratio:.2}"
            );
        }
    }
}

#[test]
fn the_256_colors_of_a_terminal_are_the_16_of_the_theme_then_the_cube_and_the_greys_of_xterm() {
    let theme = Theme::new(ThemeName::Dark, Appearance::Dark);
    assert_eq!(theme.terminal_color(1), theme.terminal[1]);
    assert_eq!(theme.terminal_color(16), Color::rgb(0, 0, 0));
    assert_eq!(theme.terminal_color(196), Color::rgb(255, 0, 0));
    assert_eq!(theme.terminal_color(231), Color::rgb(255, 255, 255));
    assert_eq!(theme.terminal_color(232), Color::rgb(8, 8, 8));
    assert_eq!(theme.terminal_color(255), Color::rgb(238, 238, 238));
}
