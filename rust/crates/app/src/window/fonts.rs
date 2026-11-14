//! The fonts the interface is drawn with: Inter for the interface and `JetBrains Mono` for hashes and code, which come
//! with the program (D18), and the fonts of the system for the characters they lack, such as emoji and Chinese,
//! Japanese or Korean text.

use render::{Assets, FontFamily, FontWeight, Fonts};
use ui::Typefaces;

use crate::error::AppError;

/// Loads the fonts of the interface into the assets of a window, with the fonts of the system after them for the
/// characters they lack, and returns the typefaces made of them. This reads the folders of fonts of the system once.
///
/// # Errors
///
/// `AppError::Font` when a typeface of the interface cannot be made of the fonts loaded.
pub fn load_system_fonts(assets: &mut Assets) -> Result<Typefaces, AppError> {
    let mut fonts = Fonts::bundled();
    fonts.load_system_fonts();
    let typefaces = typefaces(&mut fonts)?;
    assets.fonts = fonts;
    Ok(typefaces)
}

/// The typefaces of the interface: Inter at its regular, medium and semibold weights, and `JetBrains Mono` for code.
///
/// # Errors
///
/// `AppError::Font` when the fonts have no such family, which only happens to fonts made without those of the
/// program.
pub fn typefaces(fonts: &mut Fonts) -> Result<Typefaces, AppError> {
    let mut add = |family, weight| fonts.add_typeface(&family, weight).map_err(AppError::Font);
    Ok(Typefaces {
        sans: add(FontFamily::Interface, FontWeight::REGULAR)?,
        medium: add(FontFamily::Interface, FontWeight::MEDIUM)?,
        bold: add(FontFamily::Interface, FontWeight::SEMIBOLD)?,
        mono: add(FontFamily::Code, FontWeight::REGULAR)?,
    })
}

/// The typefaces of families chosen by name (`font.ui`, `font.code`): Inter and `JetBrains Mono`, which come with the
/// program, or fonts of the system. A family the fonts do not have gives the program's own in its place.
///
/// # Errors
///
/// `AppError::Font` when even the program's fonts are missing, which only happens to fonts made without them.
pub fn chosen_typefaces(fonts: &mut Fonts, interface: &str, code: &str) -> Result<Typefaces, AppError> {
    let mut add = |name: &str, own: FontFamily, weight| {
        let family = if name == bundled_name(&own) {
            own.clone()
        } else {
            FontFamily::Named(name.to_owned())
        };
        fonts
            .add_typeface(&family, weight)
            .or_else(|_| fonts.add_typeface(&own, weight))
            .map_err(AppError::Font)
    };
    Ok(Typefaces {
        sans: add(interface, FontFamily::Interface, FontWeight::REGULAR)?,
        medium: add(interface, FontFamily::Interface, FontWeight::MEDIUM)?,
        bold: add(interface, FontFamily::Interface, FontWeight::SEMIBOLD)?,
        mono: add(code, FontFamily::Code, FontWeight::REGULAR)?,
    })
}

/// The name the settings give a family that comes with the program.
fn bundled_name(family: &FontFamily) -> &str {
    match family {
        FontFamily::Interface => "Inter",
        FontFamily::Code => "JetBrains Mono",
        FontFamily::Named(name) => name,
    }
}

#[cfg(test)]
mod tests {
    use render::TextStyle;

    use super::*;

    #[test]
    fn the_fonts_of_the_program_make_the_three_typefaces() {
        let mut fonts = Fonts::bundled();
        let made = typefaces(&mut fonts).unwrap();
        assert_ne!(made.sans, made.mono);
        assert_ne!(made.sans, made.bold);
        let style = |typeface| TextStyle { typeface, size: 13.0 };
        assert!(fonts.measure("Gitardo", style(made.bold)) > fonts.measure("Gitardo", style(made.sans)));
        assert!(matches!(typefaces(&mut Fonts::new()), Err(AppError::Font(_))));
    }

    #[test]
    fn a_family_the_fonts_do_not_have_gives_the_programs_own() {
        let mut fonts = Fonts::bundled();
        let own = typefaces(&mut fonts).unwrap();
        let chosen = chosen_typefaces(&mut fonts, "No Such Family", "JetBrains Mono").unwrap();
        let style = |typeface| TextStyle { typeface, size: 13.0 };
        assert_eq!(
            fonts.measure("Gitardo", style(chosen.sans)),
            fonts.measure("Gitardo", style(own.sans))
        );
    }
}
