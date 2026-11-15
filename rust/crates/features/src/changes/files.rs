//! What the operations on the changes share: the file they were invoked on, how they name files to git, and the shape
//! of their plans.

use std::ffi::OsString;
use std::path::PathBuf;

use model::changes::pathspec;
use model::contract::{Context, Danger, Plan, Refusal, Step, Target, Text, Undo};
use model::state::{ChangeKind, WorkingTree};

/// Said when an operation on one file is given none.
pub(super) const CHOOSE_A_FILE: &str = "Choose a file of the Changes page.";

/// The file an operation was invoked on, by its path from the top of the working tree; the one of the files ticked,
/// when only one is.
pub(super) fn target_file(context: &Context) -> Result<&str, Refusal> {
    match &context.target {
        Target::File(path) => Ok(path),
        Target::Files(paths) if paths.len() == 1 => paths
            .first()
            .map(String::as_str)
            .ok_or_else(|| Refusal::saying(CHOOSE_A_FILE)),
        _ => Err(Refusal::saying(CHOOSE_A_FILE)),
    }
}

/// The files an operation was invoked on: the one file of a row, or the files ticked (`Target::Files`), each by its
/// path from the top of the working tree.
pub(super) fn target_files(context: &Context) -> Result<Vec<&str>, Refusal> {
    match &context.target {
        Target::File(path) => Ok(vec![path.as_str()]),
        Target::Files(paths) if !paths.is_empty() => Ok(paths.iter().map(String::as_str).collect()),
        _ => Err(Refusal::saying(CHOOSE_A_FILE)),
    }
}

/// The files ticked, without the old name of a staged rename whose new name is ticked too: a host that lists a rename
/// as one row may send both its names, which are one file to the user.
pub(super) fn without_old_names<'a>(tree: &WorkingTree, paths: &[&'a str]) -> Vec<&'a str> {
    paths
        .iter()
        .copied()
        .filter(|path| {
            !tree.staged.iter().any(|change| {
                change.kind == ChangeKind::Renamed
                    && change.original_path.as_deref() == Some(*path)
                    && paths.contains(&change.path.as_str())
            })
        })
        .collect()
}

/// What a plan on files names them by: the path of the one file, or how many they are.
pub(super) fn named(paths: &[&str]) -> String {
    match paths {
        [path] => (*path).to_owned(),
        _ => format!("{} files", paths.len()),
    }
}

/// A git command with the files after `--`, each named as itself (`changes.pathspec`).
pub(super) fn with_files<'a>(folder: PathBuf, arguments: &[&str], paths: impl IntoIterator<Item = &'a str>) -> Step {
    let mut all: Vec<OsString> = arguments.iter().map(OsString::from).collect();
    all.push(OsString::from("--"));
    all.extend(paths.into_iter().map(pathspec));
    Step::git(folder, all)
}

/// A plan of the changes, which Undo does not take back: `by` says what does.
pub(super) fn plan(title: String, steps: Vec<Step>, danger: Danger, by: &str) -> Plan {
    Plan {
        title: Text::from(title),
        steps,
        danger,
        undo: Undo::Impossible(Text::from(by)),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// The paths a staged change names: its own, and its old one for a rename, so that the old name comes back with it.
pub(super) fn staged_names<'a>(tree: &'a WorkingTree, path: &'a str) -> Vec<&'a str> {
    let mut names = vec![path];
    let renamed = tree
        .staged
        .iter()
        .find(|change| change.path == path && change.kind == ChangeKind::Renamed)
        .and_then(|change| change.original_path.as_deref());
    names.extend(renamed);
    names
}

/// Whether a file has a change in the index.
pub(super) fn is_staged(tree: &WorkingTree, path: &str) -> bool {
    tree.staged.iter().any(|change| change.path == path)
}

/// Whether git does not track a file.
pub(super) fn is_untracked(tree: &WorkingTree, path: &str) -> bool {
    tree.untracked.iter().any(|untracked| untracked == path)
}

/// Whether a file has a change in the working tree that is not staged.
pub(super) fn is_unstaged(tree: &WorkingTree, path: &str) -> bool {
    tree.unstaged.iter().any(|change| change.path == path)
}

/// "One untracked file will be deleted from disk.", or as many as there are.
pub(super) fn untracked_warning(count: usize) -> String {
    if count == 1 {
        String::from("One untracked file will be deleted from disk.")
    } else {
        format!("{count} untracked files will be deleted from disk.")
    }
}

/// "One file goes back to the last commit, …", or as many as there are.
pub(super) fn back_to_last_commit_warning(count: usize) -> String {
    let said = "to the last commit, staged changes included: they will be lost, and a file added since is deleted \
                from disk.";
    if count == 1 {
        format!("One file goes back {said}")
    } else {
        format!("{count} files go back {said}")
    }
}

/// The warning of changes to tracked files that are not staged.
pub(super) const UNSTAGED_WARNING: &str = "Unstaged changes to tracked files will be lost.";

/// Why discarding cannot be taken back.
pub(super) const NEVER_COMMITTED: &str = "Changes that were never committed are in no reflog.";

/// Said when there is nothing to discard.
pub(super) const NOTHING_TO_DISCARD: &str = "There is nothing to discard.";
