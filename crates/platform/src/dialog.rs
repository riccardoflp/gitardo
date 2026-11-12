//! The system's dialogs to choose a folder, a file to open, or where to save, through `rfd`. Every other dialog is
//! drawn by Gitardo.

use std::path::PathBuf;

use crate::window::Window;

/// What a dialog of the system asks for and where it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialog {
    /// The text of its title bar.
    pub title: String,
    /// The folder it opens in; the system picks one when none is given.
    pub folder: Option<PathBuf>,
    /// The name it proposes, when saving.
    pub file_name: Option<String>,
    /// The kinds of files it offers to show, the first one shown at first; every file when there are none.
    pub filters: Vec<FileFilter>,
}

/// A kind of files a dialog offers to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    /// Its name, such as *Patches*.
    pub name: String,
    /// The extensions of its files, without the dot: `patch`, `diff`.
    pub extensions: Vec<String>,
}

impl FileDialog {
    /// Asks the person to choose a folder, in a dialog that belongs to a window and keeps it waiting. None when
    /// they cancel.
    #[must_use]
    pub fn choose_folder(&self, owner: &Window) -> Option<PathBuf> {
        self.library_dialog().set_parent(owner.library()).pick_folder()
    }

    /// Asks the person to choose a file to open. None when they cancel.
    #[must_use]
    pub fn choose_file(&self, owner: &Window) -> Option<PathBuf> {
        self.library_dialog().set_parent(owner.library()).pick_file()
    }

    /// Asks the person where to save a file, the system asking whether to replace one that exists. None when they
    /// cancel.
    #[must_use]
    pub fn choose_save_path(&self, owner: &Window) -> Option<PathBuf> {
        self.library_dialog().set_parent(owner.library()).save_file()
    }

    /// The dialog of `rfd`, not shown yet.
    fn library_dialog(&self) -> rfd::FileDialog {
        let mut dialog = rfd::FileDialog::new().set_title(self.title.as_str());
        if let Some(folder) = &self.folder {
            dialog = dialog.set_directory(folder);
        }
        if let Some(name) = &self.file_name {
            dialog = dialog.set_file_name(name.as_str());
        }
        for filter in &self.filters {
            let extensions: Vec<&str> = filter
                .extensions
                .iter()
                .map(|extension| extension.trim_start_matches('.'))
                .collect();
            dialog = dialog.add_filter(filter.name.as_str(), &extensions);
        }
        dialog
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{FileDialog, FileFilter};

    /// A dialog of the system cannot be shown in a test: what is checked is that the dialog is made from the
    /// options, without showing it.
    #[test]
    fn makes_the_dialog_of_the_system_without_showing_it() {
        let options = FileDialog {
            title: String::from("Choose a patch"),
            folder: Some(PathBuf::from("projects")),
            file_name: Some(String::from("fix.patch")),
            filters: vec![FileFilter {
                name: String::from("Patches"),
                extensions: vec![String::from(".patch"), String::from("diff")],
            }],
        };
        let made = format!("{:?}", options.library_dialog());
        for expected in [
            "Choose a patch",
            "projects",
            "fix.patch",
            "Patches",
            "\"patch\"",
            "\"diff\"",
        ] {
            assert!(made.contains(expected), "{expected} is not in {made}");
        }
        assert!(!made.contains("\".patch\""), "{made}");
        let _ = FileDialog::default().library_dialog();
    }
}
