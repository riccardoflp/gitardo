//! What git ignores in the checkout, with its size on disk.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use model::worktree_setup::{FOLDER_FILES_COUNTED, FolderSize, IgnoredEntry, IgnoredKind};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Which checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredRequest {
    /// The top folder of the worktree the views show.
    pub folder: PathBuf,
}

/// What git ignores in the checkout: a folder git ignores whole as one entry, such as `node_modules`, and each other
/// file on its own, each once. Each has its size on disk: a file its bytes; a folder its files and bytes, counted
/// without following links, at most [`FOLDER_FILES_COUNTED`] files; a link is not looked into.
///
/// Feature: worktree-setup.read-ignored
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_ignored(git: &Git, request: &IgnoredRequest) -> Result<Vec<IgnoredEntry>, GitError> {
    let paths = parse_ignored(&git.in_directory(&request.folder).run_read(&ignored_arguments())?);
    Ok(paths
        .into_iter()
        .filter_map(|(path, is_folder)| {
            let on_disk = path
                .split('/')
                .fold(request.folder.clone(), |folder, name| folder.join(name));
            kind_on_disk(&on_disk, is_folder).map(|kind| IgnoredEntry { path, kind })
        })
        .collect())
}

/// `git ls-files --others --ignored --exclude-standard --directory -z`: what git ignores, a folder it ignores whole as
/// one entry with a `/` after it.
#[must_use]
pub fn ignored_arguments() -> Vec<OsString> {
    arguments(&[
        "ls-files",
        "--others",
        "--ignored",
        "--exclude-standard",
        "--directory",
        "-z",
    ])
}

/// The paths that `git ls-files --directory -z` printed, each with whether it is a folder, without what is inside a
/// folder also listed: git lists both `.vscode/` and `.vscode/settings.json` for `.vscode/*` and `.vscode/` in one
/// `.gitignore`.
#[must_use]
pub fn parse_ignored(output: &[u8]) -> Vec<(String, bool)> {
    let text = String::from_utf8_lossy(output);
    let entries: Vec<(String, bool)> = text
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| (entry.trim_end_matches('/').to_owned(), entry.ends_with('/')))
        .collect();
    let folders: Vec<String> = entries
        .iter()
        .filter(|(_, is_folder)| *is_folder)
        .map(|(path, _)| format!("{path}/"))
        .collect();
    let mut kept: Vec<(String, bool)> = Vec::new();
    for (path, is_folder) in entries {
        let is_inside_another = folders.iter().any(|folder| path.starts_with(folder.as_str()));
        if !is_inside_another && !kept.iter().any(|(known, _)| *known == path) {
            kept.push((path, is_folder));
        }
    }
    kept
}

/// What an entry is on disk; none when it is gone since git listed it.
fn kind_on_disk(path: &Path, is_folder: bool) -> Option<IgnoredKind> {
    let metadata = fs::symlink_metadata(path).ok()?;
    Some(if metadata.file_type().is_symlink() {
        IgnoredKind::Link
    } else if is_folder || metadata.is_dir() {
        IgnoredKind::Folder(folder_size(path))
    } else {
        IgnoredKind::File { bytes: metadata.len() }
    })
}

/// The files of a folder and their bytes, without following links, counting at most [`FOLDER_FILES_COUNTED`] files.
fn folder_size(folder: &Path) -> FolderSize {
    let mut size = FolderSize::default();
    let mut waiting = vec![folder.to_path_buf()];
    while let Some(next) = waiting.pop() {
        // A folder that cannot be listed adds nothing to the size; the size says what could be counted.
        let Ok(children) = fs::read_dir(&next) else {
            continue;
        };
        for child in children.filter_map(Result::ok) {
            let Ok(file_type) = child.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                waiting.push(child.path());
            } else if file_type.is_file() {
                if size.files == FOLDER_FILES_COUNTED {
                    size.has_more = true;
                    return size;
                }
                size.files += 1;
                size.bytes += child.metadata().map_or(0, |metadata| metadata.len());
            }
        }
    }
    size
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::parse_ignored;

    #[test]
    fn worktree_setup_read_ignored__nothing_is_listed_twice() {
        let output = b".vscode/\0.vscode/settings.json\0node_modules/\0.env\0.env\0.vscode-other\0";
        let entries = parse_ignored(output);
        let expected = [
            (String::from(".vscode"), true),
            (String::from("node_modules"), true),
            (String::from(".env"), false),
            (String::from(".vscode-other"), false),
        ];
        assert_eq!(entries, expected);
    }
}
