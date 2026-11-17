//! What is selected, in the colors of each theme (D69): the selected rows of a list and selected text wear the hue of
//! the accent or none, the text on them reads, and each is told apart from the grounds beside it.

use super::contrast;
use crate::color::Color;
use crate::theme::{Appearance, Colors, Theme, ThemeName};

/// The four themes a window can be drawn in, `system` being one of the first two.
const THEMES: [ThemeName; 4] = [
    ThemeName::Dark,
    ThemeName::Light,
    ThemeName::HighContrastDark,
    ThemeName::HighContrastLight,
];

/// How far the three channels of a ground may move apart from those of the ground under it, out of 255, and still be
/// a step of it with no hue of its own: the neutral steps of the themes stay within 3, and a wash of grey rounds to
/// within 3 too.
const NO_HUE: i16 = 6;

/// How far a hue may be from that of the accent, in degrees, and still be the accent's: amber stays amber and blue
/// blue, and teal is more than 130 from amber.
const SAME_HUE: f64 = 15.0;

/// The least difference of two grounds for one to be told from the other, in Oklab, where black and white are 1
/// apart: about what the eye makes out between two fields side by side.
const TOLD_APART: f64 = 0.02;

/// The colors of a theme, and the least contrast its text keeps: 7 to 1 in the high contrast themes, 4.5 to 1 in the
/// others.
fn colors_and_least_contrast(name: ThemeName) -> (Colors, f64) {
    let least = if matches!(name, ThemeName::HighContrastDark | ThemeName::HighContrastLight) {
        7.0
    } else {
        4.5
    };
    (Theme::new(name, Appearance::Dark).colors, least)
}

/// What a color adds to the ground it is drawn on: red, green and blue, each from -255 to 255.
pub(super) fn tint(color: Color, ground: Color) -> [i16; 3] {
    let channel = |color: u8, ground: u8| i16::from(color) - i16::from(ground);
    [
        channel(color.red, ground.red),
        channel(color.green, ground.green),
        channel(color.blue, ground.blue),
    ]
}

/// The hue of a tint in degrees, found as that of a color is, which holds for a difference too since a hue does not
/// change when the three channels move by the same amount; nothing when the channels moved alike, within `NO_HUE`.
fn hue(tint: [i16; 3]) -> Option<f64> {
    let [red, green, blue] = tint;
    let (most, least) = (red.max(green).max(blue), red.min(green).min(blue));
    if most - least <= NO_HUE {
        return None;
    }
    let part = |first: i16, second: i16| f64::from(first - second) / f64::from(most - least);
    let sixths = if most == red {
        part(green, blue)
    } else if most == green {
        2.0 + part(blue, red)
    } else {
        4.0 + part(red, green)
    };
    Some((sixths * 60.0).rem_euclid(360.0))
}

/// Whether a ground drawn on another has the hue the accent would give it, or none.
pub(super) fn has_the_hue_of_the_accent_or_none(color: Color, ground: Color, accent: Color) -> bool {
    let Some(hue_of_color) = hue(tint(color, ground)) else {
        return true;
    };
    let Some(hue_of_accent) = hue(tint(accent, ground)) else {
        return false;
    };
    let apart = (hue_of_color - hue_of_accent).abs();
    apart.min(360.0 - apart) <= SAME_HUE
}

/// How far apart two opaque colors look: the distance between them in the Oklab color space.
fn difference(first: Color, second: Color) -> f64 {
    let oklab = |color: Color| {
        let linear = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        let (red, green, blue) = (linear(color.red), linear(color.green), linear(color.blue));
        let long = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
        let medium = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
        let short = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();
        [
            0.210_454_255_3 * long + 0.793_617_785 * medium - 0.004_072_046_8 * short,
            1.977_998_495_1 * long - 2.428_592_205 * medium + 0.450_593_709_9 * short,
            0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766 * short,
        ]
    };
    let (first, second) = (oklab(first), oklab(second));
    first
        .iter()
        .zip(second)
        .map(|(first, second)| (first - second).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// The grounds a list is drawn on: the timeline and the pages, and the cards.
fn grounds_of_lists(colors: &Colors) -> [(Color, &'static str); 2] {
    [(colors.page_background, "a page"), (colors.card_background, "a card")]
}

/// The grounds text is selected on: a page, a card and an input box.
fn grounds_of_text(colors: &Colors) -> [(Color, &'static str); 3] {
    [
        (colors.page_background, "a page"),
        (colors.card_background, "a card"),
        (colors.input_background, "an input box"),
    ]
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__what_is_selected_or_under_the_pointer_has_the_hue_of_the_accent_or_none_in_every_theme() {
    for name in THEMES {
        let (colors, _) = colors_and_least_contrast(name);
        for (ground, place) in grounds_of_lists(&colors) {
            for (row, what) in [
                (colors.list_selection, "the selected rows of a list"),
                (
                    colors.list_inactive_selection,
                    "the selection of a list without the focus",
                ),
                (colors.list_hover, "the row under the pointer"),
            ] {
                assert!(
                    has_the_hue_of_the_accent_or_none(row, ground, colors.accent),
                    "{name:?}: {what} on {place} have a hue that is not the accent's"
                );
            }
        }
        assert!(
            has_the_hue_of_the_accent_or_none(colors.menu_selection, colors.menu_background, colors.accent),
            "{name:?}: the row lit in a menu has a hue that is not the accent's"
        );
        for (ground, place) in grounds_of_text(&colors) {
            assert!(
                has_the_hue_of_the_accent_or_none(colors.text_selection.over(ground), ground, colors.accent),
                "{name:?}: selected text on {place} has a hue that is not the accent's"
            );
        }
    }
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__in_the_dark_and_the_light_theme_a_selected_row_is_the_accent_mixed_into_the_page_one_part_in_six() {
    for name in [ThemeName::Dark, ThemeName::Light] {
        let (colors, _) = colors_and_least_contrast(name);
        let mixed = |ground: u8, accent: u8| f64::from(ground) + (f64::from(accent) - f64::from(ground)) / 6.0;
        for (selection, ground, accent) in [
            (colors.list_selection.red, colors.page_background.red, colors.accent.red),
            (
                colors.list_selection.green,
                colors.page_background.green,
                colors.accent.green,
            ),
            (
                colors.list_selection.blue,
                colors.page_background.blue,
                colors.accent.blue,
            ),
        ] {
            let apart = (f64::from(selection) - mixed(ground, accent)).abs();
            assert!(
                apart <= 2.0,
                "{name:?}: a channel of the selection is {selection}, {apart:.1} from the accent mixed into the page"
            );
        }
        assert!(
            hue(tint(colors.list_selection, colors.page_background)).is_some(),
            "{name:?}: the selection is a step of the page with no hue"
        );
    }
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__the_text_of_a_selected_row_reads_in_every_theme() {
    for name in THEMES {
        let (colors, least) = colors_and_least_contrast(name);
        for (text, what) in [
            (colors.list_selection_foreground, "the text of a selected row"),
            (colors.foreground, "the text of the interface on a selected row"),
            (colors.muted_foreground, "muted text on a selected row"),
        ] {
            let ratio = contrast(text, colors.list_selection);
            assert!(ratio >= least, "{name:?}: {what} has a contrast of {ratio:.2}");
        }
    }
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__a_selected_row_is_told_apart_from_its_ground_the_row_under_the_pointer_and_a_list_without_the_focus()
{
    for name in THEMES {
        let (colors, _) = colors_and_least_contrast(name);
        for (other, what) in [
            (colors.page_background, "the ground of a page"),
            (colors.card_background, "the ground of a card"),
            (colors.list_hover, "the row under the pointer"),
            (
                colors.list_inactive_selection,
                "the selection of a list without the focus",
            ),
        ] {
            let apart = difference(colors.list_selection, other);
            assert!(
                apart >= TOLD_APART,
                "{name:?}: a selected row is {apart:.4} from {what}"
            );
        }
    }
}

#[test]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
fn settings_theme__selected_text_is_told_apart_from_a_match_of_a_search_and_from_its_ground_and_its_text_reads() {
    for name in THEMES {
        let (colors, least) = colors_and_least_contrast(name);
        for (ground, place) in grounds_of_text(&colors) {
            let (selected, found) = (colors.text_selection.over(ground), colors.find_match.over(ground));
            let from_a_match = difference(selected, found);
            assert!(
                from_a_match >= TOLD_APART,
                "{name:?}: selected text on {place} is {from_a_match:.4} from a match of a search"
            );
            let from_the_ground = difference(selected, ground);
            assert!(
                from_the_ground >= TOLD_APART,
                "{name:?}: selected text on {place} is {from_the_ground:.4} from its ground"
            );
            let ratio = contrast(colors.foreground, selected);
            assert!(
                ratio >= least,
                "{name:?}: selected text on {place} has a contrast of {ratio:.2}"
            );
        }
    }
}
