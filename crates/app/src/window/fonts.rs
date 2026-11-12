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
}
