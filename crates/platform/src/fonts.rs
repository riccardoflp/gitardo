//! The font files of the system, and the families Gitardo Desktop draws its interface and its code with.

use std::path::{Path, PathBuf};

/// The font files of the system and the default families found among them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemFonts {
    /// Every font file found, sorted: where a character missing from the default families is looked for.
    pub files: Vec<PathBuf>,
    /// The family of the interface, when the system has it.
    pub interface: Option<FontFamily>,
    /// The family of code, when the system has one of those looked for.
    pub monospace: Option<FontFamily>,
}

/// A family of fonts: its name and the file of each style found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamily {
    /// The name of the family, such as Segoe UI.
    pub name: String,
    /// Its styles, the regular one first.
    pub faces: Vec<FontFace>,
}

/// One style of a family, in one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFace {
    /// The style.
    pub style: FontStyle,
    /// The file.
    pub path: PathBuf,
}

/// The style of a face of a family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontStyle {
    /// Upright, at the normal weight.
    Regular,
    /// Slanted, at the normal weight.
    Italic,
    /// Upright, between regular and bold.
    Semibold,
    /// Upright and bold.
    Bold,
    /// Slanted and bold.
    BoldItalic,
}

/// A family the program looks for: its name and the file name of each of its styles, the regular one first.
pub(crate) struct WantedFamily {
    /// The name of the family.
    pub(crate) name: &'static str,
    /// The file name of each style.
    pub(crate) files: &'static [(FontStyle, &'static str)],
}

/// The first of the wanted families whose regular style is among the files, with the styles found for it.
pub(crate) fn first_family(wanted: &[WantedFamily], files: &[PathBuf]) -> Option<FontFamily> {
    wanted.iter().find_map(|family| found_family(family, files))
}

/// A wanted family as found among the files, when its regular style is there.
fn found_family(family: &WantedFamily, files: &[PathBuf]) -> Option<FontFamily> {
    let faces: Vec<FontFace> = family
        .files
        .iter()
        .filter_map(|(style, name)| {
            let path = files.iter().find(|path| has_file_name(path, name))?;
            Some(FontFace {
                style: *style,
                path: path.clone(),
            })
        })
        .collect();
    let has_regular = faces.first().is_some_and(|face| face.style == FontStyle::Regular);
    has_regular.then(|| FontFamily {
        name: family.name.to_owned(),
        faces,
    })
}

/// Whether a path names this file, in any case, as the file systems of Windows and macOS ignore it.
fn has_file_name(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|file| file.to_str())
        .is_some_and(|file| file.eq_ignore_ascii_case(name))
}

/// Whether a file is a font the program can read, by its extension.
pub(crate) fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["ttf", "otf", "ttc"]
                .iter()
                .any(|font| extension.eq_ignore_ascii_case(font))
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{FontStyle, WantedFamily, first_family, is_font_file};

    const WANTED: [WantedFamily; 2] = [
        WantedFamily {
            name: "First",
            files: &[(FontStyle::Regular, "first.ttf"), (FontStyle::Bold, "firstb.ttf")],
        },
        WantedFamily {
            name: "Second",
            files: &[(FontStyle::Regular, "second.ttf"), (FontStyle::Bold, "secondb.ttf")],
        },
    ];

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(|name| PathBuf::from("fonts").join(name)).collect()
    }

    #[test]
    fn picks_the_first_family_found() {
        let both = first_family(&WANTED, &paths(&["SECOND.TTF", "First.ttf", "firstb.ttf"])).unwrap();
        assert_eq!(both.name, "First");
        assert_eq!(both.faces.len(), 2);
        assert_eq!(both.faces[1].style, FontStyle::Bold);
        let second = first_family(&WANTED, &paths(&["second.ttf", "firstb.ttf"])).unwrap();
        assert_eq!(second.name, "Second");
        assert_eq!(second.faces.len(), 1);
        assert_eq!(first_family(&WANTED, &paths(&["secondb.ttf"])), None);
    }

    #[test]
    fn knows_font_files() {
        assert!(is_font_file(&PathBuf::from("a/segoeui.TTF")));
        assert!(is_font_file(&PathBuf::from("a/cambria.ttc")));
        assert!(!is_font_file(&PathBuf::from("a/vgasys.fon")));
        assert!(!is_font_file(&PathBuf::from("a/desktop.ini")));
    }
}
