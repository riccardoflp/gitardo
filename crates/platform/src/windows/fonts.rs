//! The fonts of Windows: the files in the Fonts folder of Windows and in the person's own, and the families of the
//! interface and of code.

use std::path::{Path, PathBuf};

use crate::error::PlatformError;
use crate::fonts::{FontStyle, SystemFonts, WantedFamily, first_family, is_font_file};

/// The family of the interface: Segoe UI, on every Windows.
const INTERFACE: [WantedFamily; 1] = [WantedFamily {
    name: "Segoe UI",
    files: &[
        (FontStyle::Regular, "segoeui.ttf"),
        (FontStyle::Italic, "segoeuii.ttf"),
        (FontStyle::Semibold, "seguisb.ttf"),
        (FontStyle::Bold, "segoeuib.ttf"),
        (FontStyle::BoldItalic, "segoeuiz.ttf"),
    ],
}];

/// The families of code, the first found wins: Cascadia Mono, from Windows 11 on, then Consolas.
const MONOSPACE: [WantedFamily; 2] = [
    WantedFamily {
        name: "Cascadia Mono",
        files: &[
            (FontStyle::Regular, "CascadiaMono.ttf"),
            (FontStyle::Italic, "CascadiaMonoItalic.ttf"),
        ],
    },
    WantedFamily {
        name: "Consolas",
        files: &[
            (FontStyle::Regular, "consola.ttf"),
            (FontStyle::Italic, "consolai.ttf"),
            (FontStyle::Bold, "consolab.ttf"),
            (FontStyle::BoldItalic, "consolaz.ttf"),
        ],
    },
];

/// The font files of the system, and the families of the interface and of code found among them.
///
/// # Errors
///
/// `PlatformError::Folder` when the Fonts folder of Windows cannot be read. The person's own folder is read when it
/// is there.
pub fn system_fonts() -> Result<SystemFonts, PlatformError> {
    let mut files = font_files(&windows_fonts_folder())?;
    if let Some(folder) = user_fonts_folder() {
        // Fonts installed for the person alone, from Windows 10 version 1809; most people have none, or no folder.
        files.extend(font_files(&folder).unwrap_or_default());
    }
    files.sort();
    let interface = first_family(&INTERFACE, &files);
    let monospace = first_family(&MONOSPACE, &files);
    Ok(SystemFonts {
        files,
        interface,
        monospace,
    })
}

/// The Fonts folder of Windows: `%WINDIR%\Fonts`.
fn windows_fonts_folder() -> PathBuf {
    let windows = std::env::var_os("WINDIR")
        .or_else(|| std::env::var_os("SystemRoot"))
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    windows.join("Fonts")
}

/// The folder of the fonts the person installed for themselves alone.
fn user_fonts_folder() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local).join(r"Microsoft\Windows\Fonts"))
}

/// The font files of a folder.
fn font_files(folder: &Path) -> Result<Vec<PathBuf>, PlatformError> {
    let entries = std::fs::read_dir(folder).map_err(|error| PlatformError::Folder {
        path: folder.to_path_buf(),
        kind: error.kind(),
    })?;
    // An entry that cannot be read, such as a file removed meanwhile, is left out: the other fonts are still usable.
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_font_file(path))
        .collect())
}
