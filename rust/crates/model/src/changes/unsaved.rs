//! Which files of a repository have edits not saved in Gitardo's own editors, for the Changes page: the File page and
//! the working tree side of the Diff page.

use std::path::Path;

/// What [`unsaved`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct UnsavedInput<'a> {
    /// The files with edits not saved, as the editors name them: from the top of the working tree with `/`, or by
    /// their whole path for a file outside it, such as the user's settings file.
    pub buffers: &'a [String],
    /// The folders of the submodules, from the top with `/`.
    pub submodules: &'a [String],
    /// The folders of the other worktrees that are inside this one, from the top with `/`.
    pub worktrees: &'a [String],
    /// The candidates `git check-ignore` was asked about, and those of them it ignores; a candidate not asked about yet
    /// waits for its answer.
    pub checked: (&'a [String], &'a [String]),
}

/// The paths git is asked about with `git check-ignore`: the files with edits not saved that are inside the
/// repository, and not in a submodule or in another worktree inside it, sorted.
#[must_use]
pub fn ignore_candidates(input: &UnsavedInput<'_>) -> Vec<String> {
    let mut candidates: Vec<String> = input
        .buffers
        .iter()
        .filter(|path| is_inside(path))
        .filter(|path| {
            !input
                .submodules
                .iter()
                .chain(input.worktrees)
                .any(|folder| is_under(path, folder))
        })
        .cloned()
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates
}

/// The files of the repository with edits not saved, by their paths from its top, sorted: the files outside it, in its
/// submodules, in the other worktrees inside it and those git ignores are left out.
///
/// Feature: changes.unsaved
#[must_use]
pub fn unsaved(input: &UnsavedInput<'_>) -> Vec<String> {
    let (asked, ignored) = input.checked;
    ignore_candidates(input)
        .into_iter()
        .filter(|path| asked.contains(path) && !ignored.contains(path))
        .collect()
}

/// Whether a path an editor names is inside the working tree: given from its top, not by a whole path, on any system,
/// nor going up.
fn is_inside(path: &str) -> bool {
    let has_drive = path.as_bytes().get(1) == Some(&b':');
    !Path::new(path).is_absolute()
        && !has_drive
        && !path.starts_with(['/', '\\'])
        && !path.starts_with("../")
        && path != ".."
}

/// Whether a path is inside a folder, both from the top with `/`.
fn is_under(path: &str, folder: &str) -> bool {
    let folder = folder.trim_end_matches('/');
    path.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{UnsavedInput, ignore_candidates, unsaved};

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn changes_unsaved__a_file_typed_in_and_not_saved_is_in_the_result_and_once_saved_it_is_not() {
        let typed = paths(&["src/a.rs"]);
        let input = UnsavedInput {
            buffers: &typed,
            checked: (&typed, &[]),
            ..UnsavedInput::default()
        };
        assert_eq!(unsaved(&input), ["src/a.rs"]);
        let saved = UnsavedInput { buffers: &[], ..input };
        assert_eq!(unsaved(&saved), Vec::<String>::new());
    }

    #[test]
    fn changes_unsaved__a_file_of_a_submodule_and_a_file_git_ignores_are_left_out() {
        let buffers = paths(&["vendor/lib/a.c", "node_modules/x/index.js", "src/b.rs"]);
        let submodules = paths(&["vendor/lib"]);
        let worktrees = paths(&["trees/other"]);
        let asked = paths(&["node_modules/x/index.js", "src/b.rs"]);
        let ignored = paths(&["node_modules/x/index.js"]);
        let input = UnsavedInput {
            buffers: &buffers,
            submodules: &submodules,
            worktrees: &worktrees,
            checked: (&asked, &ignored),
        };
        assert_eq!(ignore_candidates(&input), asked);
        assert_eq!(unsaved(&input), ["src/b.rs"]);
    }

    #[test]
    fn changes_unsaved__a_file_outside_the_repository_is_left_out() {
        let buffers = paths(&[
            "/home/ada/.config/gitardo/settings.json",
            "../next/a.rs",
            "C:/Users/ada/settings.json",
        ]);
        let input = UnsavedInput {
            buffers: &buffers,
            checked: (&buffers, &[]),
            ..UnsavedInput::default()
        };
        assert_eq!(ignore_candidates(&input), Vec::<String>::new());
        assert_eq!(unsaved(&input), Vec::<String>::new());
    }

    #[test]
    fn a_candidate_not_asked_about_yet_waits_and_the_result_is_sorted() {
        let buffers = paths(&["src/z.rs", "src/a.rs", "src/new.rs"]);
        let asked = paths(&["src/a.rs", "src/z.rs"]);
        let input = UnsavedInput {
            buffers: &buffers,
            checked: (&asked, &[]),
            ..UnsavedInput::default()
        };
        assert_eq!(unsaved(&input), ["src/a.rs", "src/z.rs"]);
    }
}
