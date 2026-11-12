//! The theme a window is drawn in: the one chosen, or the system's when the choice is to follow it, high contrast
//! included; and whether the title bar is dark or light to go with it, or follows the system.

use platform::{Appearance, SystemTheme};
use render::{Theme, ThemeName};

/// The theme a choice gives with the system as it is. Following the system gives its high contrast theme when high
/// contrast is on.
#[must_use]
pub fn resolve_theme(choice: ThemeName, system: SystemTheme) -> Theme {
    let name = match (choice, system.is_high_contrast, system.appearance) {
        (ThemeName::System, true, Appearance::Dark) => ThemeName::HighContrastDark,
        (ThemeName::System, true, Appearance::Light) => ThemeName::HighContrastLight,
        (ThemeName::System, false, Appearance::Dark) => ThemeName::Dark,
        (ThemeName::System, false, Appearance::Light) => ThemeName::Light,
        (chosen, ..) => chosen,
    };
    Theme::new(name, render::Appearance::Dark)
}

/// Whether the title bar goes dark or light with a theme chosen; none when the choice is to follow the system, so that
/// the title bar follows it too and the system says when it changes.
#[must_use]
pub fn title_bar(choice: ThemeName, theme: &Theme) -> Option<Appearance> {
    if choice == ThemeName::System {
        return None;
    }
    Some(match theme.name {
        ThemeName::Dark | ThemeName::HighContrastDark => Appearance::Dark,
        ThemeName::Light | ThemeName::HighContrastLight | ThemeName::System => Appearance::Light,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(appearance: Appearance, is_high_contrast: bool) -> SystemTheme {
        SystemTheme {
            appearance,
            is_high_contrast,
        }
    }

    #[test]
    fn following_the_system_takes_its_appearance_and_its_high_contrast() {
        let cases = [
            (system(Appearance::Dark, false), ThemeName::Dark),
            (system(Appearance::Light, false), ThemeName::Light),
            (system(Appearance::Dark, true), ThemeName::HighContrastDark),
            (system(Appearance::Light, true), ThemeName::HighContrastLight),
        ];
        for (system, expected) in cases {
            assert_eq!(resolve_theme(ThemeName::System, system).name, expected);
        }
    }

    #[test]
    fn a_theme_chosen_stays_whatever_the_system_does() {
        let high_contrast = system(Appearance::Dark, true);
        assert_eq!(resolve_theme(ThemeName::Light, high_contrast).name, ThemeName::Light);
        assert_eq!(
            title_bar(ThemeName::Light, &resolve_theme(ThemeName::Light, high_contrast)),
            Some(Appearance::Light)
        );
        let dark = resolve_theme(ThemeName::HighContrastDark, system(Appearance::Light, false));
        assert_eq!(title_bar(ThemeName::HighContrastDark, &dark), Some(Appearance::Dark));
        let followed = resolve_theme(ThemeName::System, high_contrast);
        assert_eq!(title_bar(ThemeName::System, &followed), None);
    }
}
