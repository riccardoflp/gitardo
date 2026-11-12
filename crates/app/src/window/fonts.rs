//! The fonts of the system the interface is drawn with: Segoe UI, regular and bold, Cascadia Mono or Consolas for
//! code, and fonts tried after them for the characters they lack, such as symbols, emoji and Chinese, Japanese or
//! Korean text.

use std::path::{Path, PathBuf};

use platform::{FontFamily, FontStyle, SystemFonts};
use render::{Assets, Font, FontId};
use ui::Typefaces;

use crate::error::AppError;

/// The files of the fonts tried, in order, for a character the interface and code fonts lack. Those that are missing
/// or that the renderer cannot draw are left out.
const FALLBACK_FILES: [&str; 4] = ["seguisym.ttf", "seguiemj.ttf", "msyh.ttc", "malgun.ttf"];

/// Loads the fonts of the interface into the assets of a window, and returns the typefaces made of them.
///
/// # Errors
///
/// `AppError::Platform` when the fonts of the system could not be listed; `AppError::NoInterfaceFont` when Segoe UI
/// is missing; `AppError::FontFile` or `AppError::Font` when its regular face cannot be read.
#[cfg(windows)]
pub fn load_system_fonts(assets: &mut Assets) -> Result<Typefaces, AppError> {
    let system = platform::system_fonts()?;
    load_fonts(assets, &system)
}

/// Loads the fonts the system has into the assets, and returns the typefaces made of them.
///
/// # Errors
///
/// As [`load_system_fonts`], once the fonts are listed.
pub fn load_fonts(assets: &mut Assets, system: &SystemFonts) -> Result<Typefaces, AppError> {
    let interface = system.interface.as_ref().ok_or(AppError::NoInterfaceFont)?;
    let regular_path = face(interface, &[FontStyle::Regular]).ok_or(AppError::NoInterfaceFont)?;
    let regular = load(assets, regular_path)?;
    let bold = match face(interface, &[FontStyle::Bold, FontStyle::Semibold]) {
        Some(path) => load(assets, path).unwrap_or(regular),
        None => regular,
    };
    let mono = system
        .monospace
        .as_ref()
        .and_then(|family| face(family, &[FontStyle::Regular]))
        .and_then(|path| load(assets, path).ok())
        .unwrap_or(regular);
    let fallbacks: Vec<FontId> = fallback_files(&system.files)
        .iter()
        .filter_map(|path| load(assets, path).ok())
        .collect();
    let typeface = |assets: &mut Assets, first: &[FontId]| {
        let fonts: Vec<FontId> = first.iter().chain(&fallbacks).copied().collect();
        assets.fonts.add_typeface(&fonts).map_err(|error| AppError::Font {
            path: regular_path.to_path_buf(),
            error,
        })
    };
    Ok(Typefaces {
        sans: typeface(assets, &[regular])?,
        bold: typeface(assets, &[bold, regular])?,
        mono: typeface(assets, &[mono, regular])?,
    })
}

/// The file of the first of some styles a family has.
fn face<'family>(family: &'family FontFamily, styles: &[FontStyle]) -> Option<&'family Path> {
    styles.iter().find_map(|style| {
        family
            .faces
            .iter()
            .find(|face| face.style == *style)
            .map(|face| face.path.as_path())
    })
}

/// The files of the fonts tried for missing characters among the fonts of the system, in the order they are tried.
fn fallback_files(files: &[PathBuf]) -> Vec<&Path> {
    FALLBACK_FILES
        .iter()
        .filter_map(|wanted| {
            files.iter().map(PathBuf::as_path).find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
            })
        })
        .collect()
}

/// Reads a font file, the first font of it when it is a collection, and adds it to the assets.
fn load(assets: &mut Assets, path: &Path) -> Result<FontId, AppError> {
    let bytes = std::fs::read(path).map_err(|error| AppError::FontFile {
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;
    let font_error = |error| AppError::Font {
        path: path.to_path_buf(),
        error,
    };
    let font = Font::from_collection(bytes, 0).map_err(font_error)?;
    assets.fonts.add_font(font).map_err(font_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fallbacks_are_tried_in_their_order_whatever_the_case_of_their_names() {
        let files: Vec<PathBuf> = ["C:/Fonts/MALGUN.TTF", "C:/Fonts/arial.ttf", "C:/Fonts/seguisym.ttf"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let found = fallback_files(&files);
        assert_eq!(
            found,
            vec![Path::new("C:/Fonts/seguisym.ttf"), Path::new("C:/Fonts/MALGUN.TTF")]
        );
    }

    #[test]
    fn the_fonts_kept_for_the_tests_make_the_three_typefaces() {
        let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testkit/fonts");
        let family = |name: &str, file: &str| FontFamily {
            name: name.to_owned(),
            faces: vec![platform::FontFace {
                style: FontStyle::Regular,
                path: fonts.join(file),
            }],
        };
        let system = SystemFonts {
            files: vec![fonts.join("DejaVuSans.ttf")],
            interface: Some(family("DejaVu Sans", "DejaVuSans.ttf")),
            monospace: Some(family("DejaVu Sans Mono", "DejaVuSansMono.ttf")),
        };
        let mut assets = Assets::new();
        let typefaces = load_fonts(&mut assets, &system).unwrap();
        assert_ne!(typefaces.sans, typefaces.mono);
        let none = SystemFonts::default();
        assert_eq!(load_fonts(&mut Assets::new(), &none), Err(AppError::NoInterfaceFont));
    }
}
