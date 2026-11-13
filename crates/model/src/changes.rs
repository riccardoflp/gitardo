//! The files of the working tree, as the Changes page lists them and the operations on them name them.

mod list;
mod open;
mod unsaved;

use std::ffi::OsString;

pub use list::{ChangeRow, ChangesInput, ChangesList, FileStatus, StagedBox, list};
pub use open::{OpenAction, OpenInput, OpenOnClick, OpenTarget, OpenVersion, PageRequest, open};
pub use unsaved::{UnsavedInput, ignore_candidates, unsaved};

/// How a file is named to the git commands that take patterns (`git add`, `git restore`, `git rm`, `git clean`,
/// `git stash push`, `git checkout <commit> -- <path>`), so that a name that is also a pattern means only that file:
/// `:(literal)<path>` when it holds `[`, `]`, `*`, `?` or `\` or starts with `:`, else the path as it is. `.` is never
/// rewritten.
///
/// Feature: changes.pathspec
#[must_use]
pub fn pathspec(path: &str) -> OsString {
    let is_pattern = path.contains(['[', ']', '*', '?', '\\']) || path.starts_with(':');
    if is_pattern && path != "." {
        OsString::from(format!(":(literal){path}"))
    } else {
        OsString::from(path)
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::pathspec;

    #[test]
    fn changes_pathspec__names_a_file_with_brackets_literally() {
        assert_eq!(pathspec("pages/[id].tsx"), ":(literal)pages/[id].tsx");
    }

    #[test]
    fn changes_pathspec__prefixes_only_the_names_that_are_patterns() {
        for path in ["a*.txt", "what?.md", ":odd", "back\\slash"] {
            assert_eq!(pathspec(path), format!(":(literal){path}").as_str());
        }
        assert_eq!(pathspec("src/main.rs"), "src/main.rs");
        assert_eq!(pathspec("."), ".");
    }
}
