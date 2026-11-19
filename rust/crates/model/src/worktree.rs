//! Worktrees: where a new one goes, how the views name one, what a name typed for one must be, what is done when
//! Windows holds its folder, the folders left empty after it, a branch another worktree has, and what a lock changes.

mod branch_of;
mod empty_folders;
mod held;
mod locked;
mod name;

use std::path::{Path, PathBuf};

pub use branch_of::{OtherWorktreeInput, WorktreeBranch, branch_in_another_worktree};
pub use empty_folders::{EmptyFoldersInput, remove_empty_folders};
pub use held::{
    DELETE_TRIES, DELETING_LEFT, HeldInput, MOVE_TRIES, MOVE_WAIT, Recovery, TRIED_AGAIN, delete_wait,
    recover_held_folder,
};
pub use locked::{LOCKED, LockedInput, LockedRules, MISSING_LOCKED, has_prunable, locked};
pub use name::{FolderBase, WorktreeNameRule, is_listed};

/// The answer of `worktree.remove` that says whether the branch goes too: [`KEEP_BRANCH`] or [`DELETE_BRANCH`]. The
/// entries **Remove worktree…** and **Remove worktree and delete branch…** of the menu of a branch give it.
pub const REMOVE_BRANCH: crate::contract::AnswerKey = crate::contract::AnswerKey("branch");
/// The choice that keeps the branch of a worktree removed.
pub const KEEP_BRANCH: &str = "keep";
/// The choice that deletes the branch of a worktree removed, if it is merged.
pub const DELETE_BRANCH: &str = "delete";

/// The folder of the worktrees when `worktrees.folder` is empty.
pub const DEFAULT_FOLDER: &str = "${repo}.worktrees";

/// The most characters a part of the folder of a worktree keeps.
const PART_LENGTH: usize = 60;

/// What [`folder_path`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderInput<'a> {
    /// `worktrees.folder`, as the setting has it.
    pub setting: &'a str,
    /// The folder of the main worktree, whichever worktree asks.
    pub main_worktree: &'a Path,
    /// The home folder of the user, for `~`.
    pub home: &'a Path,
    /// The name of the worktree: the name of its branch, slashes and all.
    pub name: &'a str,
}

/// A path with forward slashes and none at its end.
fn slashed(path: &str) -> String {
    let path = path.replace('\\', "/");
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() && path.starts_with('/') {
        String::from("/")
    } else {
        trimmed.to_owned()
    }
}

/// Whether a path, with forward slashes, is absolute: `/srv`, `D:/trees`, `//server/share`.
fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/') || matches!(bytes, [letter, b':', ..] if letter.is_ascii_alphabetic())
}

/// The folder of the worktrees: the setting, or its default, with `${repo}` and `~` filled, from the folder that holds
/// the main worktree when it is relative.
#[must_use]
pub fn worktrees_folder(setting: &str, main_worktree: &Path, home: &Path) -> String {
    let setting = if setting.trim().is_empty() {
        DEFAULT_FOLDER
    } else {
        setting.trim()
    };
    let main = slashed(&main_worktree.to_string_lossy());
    let (parent, repo) = main.rsplit_once('/').unwrap_or(("", main.as_str()));
    let filled = slashed(&setting.replace("${repo}", repo));
    let folder = if let Some(rest) = filled.strip_prefix('~') {
        format!("{}{rest}", slashed(&home.to_string_lossy()))
    } else if is_absolute(&filled) {
        filled
    } else {
        format!("{parent}/{filled}")
    };
    normalized(&folder)
}

/// A path without `.` and with each `..` taking away the part before it.
fn normalized(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." => {}
            ".." if parts.last().is_some_and(|last| !last.is_empty() && *last != "..") => {
                let _ = parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// The part of a folder made of a part of a name: runs of other characters than ASCII letters, digits, `_`, `.` and `-`
/// become one `-`, `-` and `.` are dropped from both ends, and it is cut to 60 characters.
fn folder_part(part: &str) -> String {
    let mut folder = String::new();
    for character in part.chars() {
        if character.is_ascii_alphanumeric() || "_.-".contains(character) {
            folder.push(character);
        } else if !folder.ends_with('-') {
            folder.push('-');
        }
    }
    let folder: String = folder.trim_matches(['-', '.']).chars().take(PART_LENGTH).collect();
    folder.trim_end_matches(['-', '.']).to_owned()
}

/// The folder of a new worktree: the folder of the worktrees, then the name part by part between slashes, each made a
/// safe folder name; `worktree` when nothing is left.
///
/// Feature: worktree.folder-path
#[must_use]
pub fn folder_path(input: &FolderInput<'_>) -> PathBuf {
    folder_in(
        &worktrees_folder(input.setting, input.main_worktree, input.home),
        input.name,
    )
}

/// The folder a name gives inside the folder of the worktrees, already worked out from the setting.
#[must_use]
pub fn folder_in(folder: &str, name: &str) -> PathBuf {
    let parts: Vec<String> = name
        .replace('\\', "/")
        .split('/')
        .map(folder_part)
        .filter(|part| !part.is_empty())
        .collect();
    let inside = if parts.is_empty() {
        String::from("worktree")
    } else {
        parts.join("/")
    };
    PathBuf::from(format!("{folder}/{inside}"))
}

/// How the views name a worktree: its path from the folder of the worktrees when it is inside it, compared without
/// regard to case, and the name of its folder otherwise.
#[must_use]
pub fn worktree_label(worktrees_folder: &str, worktree: &Path) -> String {
    let path = slashed(&worktree.to_string_lossy());
    let folder = slashed(worktrees_folder);
    let inside = path
        .get(..folder.len())
        .filter(|start| start.eq_ignore_ascii_case(&folder))
        .and_then(|_| path.get(folder.len()..))
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|rest| !rest.is_empty());
    match inside {
        Some(rest) => rest.to_owned(),
        None => path.rsplit('/').next().unwrap_or_default().to_owned(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
