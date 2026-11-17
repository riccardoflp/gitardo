//! The theme of Symbols, read from `icons/symbols/theme.txt`: which icon each extension, name of a file, name of a
//! folder and language has, and the icons of what nothing names. A file is named by its name, then its extensions from
//! the longest, then its language, as VS Code does with an icon theme; every name in lower case, as VS Code compares
//! them.

use std::collections::HashMap;

use super::{FileIcon, FileOrFolder, languages};

/// The text of the theme, one entry a line, its parts separated by a tab.
const THEME: &str = include_str!("../../icons/symbols/theme.txt");

/// The icons of the theme by what they are given to.
#[derive(Clone, Debug)]
pub(super) struct Lookup {
    /// By extension, without its dot, in lower case: `rs`, `d.ts`.
    extensions: HashMap<String, FileIcon>,
    /// By the whole name of a file, in lower case: `package.json`.
    names: HashMap<String, FileIcon>,
    /// By the name of a folder, in lower case: `src`.
    folders: HashMap<String, FileIcon>,
    /// By the language of VS Code: `php`.
    languages: HashMap<String, FileIcon>,
    /// The icon of a file nothing else names.
    file: FileIcon,
    /// The icon of a folder nothing else names.
    folder: FileIcon,
    /// The icon of the folder of the repository.
    top: FileIcon,
}

impl Lookup {
    /// The theme the program comes with.
    pub(super) fn bundled() -> Lookup {
        Lookup::parse(THEME)
    }

    /// A theme from its text. An entry that names an icon the program does not have is left out; a test reads every
    /// entry of the theme the program comes with.
    pub(super) fn parse(text: &str) -> Lookup {
        let mut lookup = Lookup {
            extensions: HashMap::new(),
            names: HashMap::new(),
            folders: HashMap::new(),
            languages: HashMap::new(),
            file: FileIcon::FIRST_FILE,
            folder: FileIcon::FIRST_FOLDER,
            top: FileIcon::FIRST_FOLDER,
        };
        for line in text.lines().filter(|line| !line.starts_with('#')) {
            let mut parts = line.split('\t');
            let (Some(what), Some(key), Some(icon)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            lookup.add(what, key, icon);
        }
        lookup
    }

    /// Adds an entry of the theme.
    fn add(&mut self, what: &str, key: &str, icon: &str) {
        let as_file = FileIcon::of_file_named(icon);
        let as_folder = FileIcon::of_folder_named(icon);
        match (what, key) {
            ("default", "file") => self.file = as_file.unwrap_or(self.file),
            ("default", "folder") => self.folder = as_folder.unwrap_or(self.folder),
            ("default", "top folder") => self.top = as_folder.unwrap_or(self.top),
            ("extension", _) => insert(&mut self.extensions, key.to_lowercase(), as_file),
            ("name", _) => insert(&mut self.names, key.to_lowercase(), as_file),
            ("folder", _) => insert(&mut self.folders, key.to_lowercase(), as_folder),
            ("language", _) => insert(&mut self.languages, key.to_owned(), as_file),
            // A kind of entry this program does not know is left out, as a theme of VS Code leaves it.
            _ => {}
        }
    }

    /// The icon of a file or a folder.
    pub(super) fn icon_of(&self, asked: FileOrFolder<'_>) -> FileIcon {
        match asked {
            FileOrFolder::File(name) => self.file(&name.to_lowercase()),
            FileOrFolder::Folder(name) => self.folders.get(&name.to_lowercase()).copied().unwrap_or(self.folder),
            FileOrFolder::Top => self.top,
        }
    }

    /// The icon of a file by its name in lower case.
    fn file(&self, name: &str) -> FileIcon {
        if let Some(icon) = self.names.get(name) {
            return *icon;
        }
        if let Some(icon) = super::extensions(name).find_map(|extension| self.extensions.get(extension)) {
            return *icon;
        }
        languages::language_of(name)
            .and_then(|language| self.languages.get(language))
            .copied()
            .unwrap_or(self.file)
    }

    /// How many entries name an icon the program has, for the test of the theme.
    #[cfg(test)]
    pub(super) fn count(&self) -> usize {
        self.extensions.len() + self.names.len() + self.folders.len() + self.languages.len()
    }
}

/// Adds an entry to a map, when its icon is one the program has.
fn insert(map: &mut HashMap<String, FileIcon>, key: String, icon: Option<FileIcon>) {
    if let Some(icon) = icon {
        let _ = map.insert(key, icon);
    }
}
