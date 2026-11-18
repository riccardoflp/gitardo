//! The reads of the Search page (decision D73): the files of the worktree it lists by name, and the lines of their
//! text that hold what is looked for, as git finds them.

mod text;

use std::collections::BTreeSet;
use std::ffi::OsString;

use crate::read::fields::arguments;
use crate::{Git, GitError};

pub use text::{TextEnd, TextRequest, parse_text, read_text, text_arguments};

/// Which files the search lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NamesRequest {
    /// The folder git is run in, from the top of the worktree, whose files alone are listed; none for the top.
    pub folder: Option<String>,
    /// The pathspecs that narrow the files, read from that folder (`file-search.pathspecs`).
    pub pathspecs: Vec<String>,
    /// Whether the files git ignores are listed too.
    pub ignored: bool,
}

/// The paths of the files the search lists and takes its folders from, from the top of the worktree, with `/`, each
/// once, in order: the files git tracks that are in the working tree, and the files it does not track, without those
/// it ignores unless they are asked for. `git` runs in the top folder of the worktree.
///
/// Feature: file-search.read-names
///
/// # Errors
///
/// When git cannot be started, fails or is cancelled.
pub fn read_names(git: &Git, request: &NamesRequest) -> Result<Vec<String>, GitError> {
    in_folder(git, request.folder.as_deref())
        .run_read(&names_arguments(request))
        .map(|output| parse_names(&output))
}

/// `git ls-files -z -t --full-name --cached --others --deleted --exclude-standard -- <pathspecs>`, without
/// `--exclude-standard` when what git ignores is listed too.
#[must_use]
pub fn names_arguments(request: &NamesRequest) -> Vec<OsString> {
    let mut found = arguments(&[
        "ls-files",
        "-z",
        "-t",
        "--full-name",
        "--cached",
        "--others",
        "--deleted",
    ]);
    if !request.ignored {
        found.push(OsString::from("--exclude-standard"));
    }
    found.push(OsString::from("--"));
    found.extend(request.pathspecs.iter().map(OsString::from));
    found
}

/// The paths [`names_arguments`] listed, each after the tag of its state and ended by a NUL. A file in conflict,
/// listed once for each of its stages, is kept once; a file git tracks that is not in the working tree any more,
/// listed again with the tag `R`, is left out, and so is a folder git does not enter, a repository of its own.
#[must_use]
pub fn parse_names(output: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(output);
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    let mut removed: Vec<&str> = Vec::new();
    for record in text.split('\0') {
        let Some((tag, path)) = record.split_once(' ') else {
            continue;
        };
        if tag == "R" {
            removed.push(path);
        } else if !path.is_empty() && !path.ends_with('/') {
            let _ = paths.insert(path);
        }
    }
    for path in removed {
        let _ = paths.remove(path);
    }
    paths.into_iter().map(str::to_owned).collect()
}

/// The same git, run in a folder of the worktree.
fn in_folder(git: &Git, folder: Option<&str>) -> Git {
    match folder {
        Some(folder) if !folder.is_empty() => git.in_directory(git.directory().join(folder)),
        _ => git.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{NamesRequest, names_arguments, parse_names};
    use crate::command_line;

    #[test]
    fn the_arguments_leave_out_what_git_ignores_unless_asked() {
        let mut request = NamesRequest {
            folder: None,
            pathspecs: vec![String::from(":(glob)**/*.rs")],
            ignored: false,
        };
        assert_eq!(
            command_line(&names_arguments(&request)),
            "git ls-files -z -t --full-name --cached --others --deleted --exclude-standard -- :(glob)**/*.rs"
        );
        request.ignored = true;
        assert_eq!(
            command_line(&names_arguments(&request)),
            "git ls-files -z -t --full-name --cached --others --deleted -- :(glob)**/*.rs"
        );
    }

    #[test]
    fn keeps_each_file_once_and_leaves_out_the_removed_ones() {
        let output = b"H src/b.rs\0H gone.rs\0M both.rs\0M both.rs\0R gone.rs\0? new file.txt\0? nested/\0";
        assert_eq!(parse_names(output), ["both.rs", "new file.txt", "src/b.rs"]);
        assert_eq!(parse_names(b""), Vec::<String>::new());
    }
}
