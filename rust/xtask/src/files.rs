//! The files of the workspace that the checks read.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;

/// A file of the workspace, with its path from the root of the workspace written with `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Where it is, from the root of the workspace, with `/` between folders on every system.
    pub relative: String,
    /// Where it is on the disk.
    pub path: PathBuf,
}

/// The extensions of the files that hold text the project writes.
const TEXT_EXTENSIONS: [&str; 7] = ["rs", "toml", "md", "txt", "lock", "json", "yml"];

/// The names of text files that have no extension.
const TEXT_NAMES: [&str; 2] = [".gitattributes", ".gitignore"];

/// Every file under `root`, sorted, leaving out the build output and the folders of tools: `target`, and folders
/// whose name starts with a dot but for `.cargo`.
pub fn workspace_files(root: &Path) -> Result<Vec<SourceFile>, Error> {
    let mut found = Vec::new();
    collect(root, "", &mut found)?;
    found.sort_by(|first, second| first.relative.cmp(&second.relative));
    Ok(found)
}

fn collect(folder: &Path, prefix: &str, found: &mut Vec<SourceFile>) -> Result<(), Error> {
    let read_error = |source| Error::Read {
        path: folder.to_path_buf(),
        source,
    };
    for entry in fs::read_dir(folder).map_err(read_error)? {
        let entry = entry.map_err(read_error)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        let kind = entry.file_type().map_err(read_error)?;
        if kind.is_dir() && is_walked(&name) {
            collect(&entry.path(), &format!("{relative}/"), found)?;
        } else if kind.is_file() {
            found.push(SourceFile {
                relative,
                path: entry.path(),
            });
        }
    }
    Ok(())
}

fn is_walked(folder_name: &str) -> bool {
    folder_name != "target" && (folder_name == ".cargo" || !folder_name.starts_with('.'))
}

/// Whether the file holds text the project writes, as opposed to images, fonts and other binary files.
pub fn is_text(file: &SourceFile) -> bool {
    let name = file.relative.rsplit('/').next().unwrap_or(&file.relative);
    TEXT_NAMES.contains(&name)
        || Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| TEXT_EXTENSIONS.contains(&extension))
}

/// Whether the file is Rust source.
pub fn is_rust(file: &SourceFile) -> bool {
    has_extension(&file.relative, "rs")
}

/// Whether a file name ends with an extension, in any case.
pub fn has_extension(name: &str, extension: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

/// The text of a file.
pub fn read_text(path: &Path) -> Result<String, Error> {
    let bytes = fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{SourceFile, is_text, is_walked};
    use std::path::PathBuf;

    #[test]
    fn walks_the_folders_of_the_project_only() {
        let cases = [
            ("crates", true),
            (".cargo", true),
            ("target", false),
            (".git", false),
            (".claude", false),
        ];
        for (name, expected) in cases {
            assert_eq!(is_walked(name), expected, "{name}");
        }
    }

    #[test]
    fn tells_text_from_binary_files() {
        let cases = [
            ("crates/json/src/lib.rs", true),
            ("Cargo.lock", true),
            (".gitattributes", true),
            ("docs/features/README.md", true),
            ("crates/testkit/fonts/a.ttf", false),
            ("crates/views/tests/snapshots/a.png", false),
        ];
        for (relative, expected) in cases {
            let file = SourceFile {
                relative: relative.to_owned(),
                path: PathBuf::from(relative),
            };
            assert_eq!(is_text(&file), expected, "{relative}");
        }
    }
}
