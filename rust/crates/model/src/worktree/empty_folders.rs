//! `worktree.remove-empty-folders`: the folders that held only a worktree moved or removed, deleted after it.

use std::path::{Path, PathBuf};

use crate::contract::Step;

use super::slashed;

/// Where a worktree was, and the folder of the worktrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyFoldersInput<'a> {
    /// The folder of the worktrees, worked out from `worktrees.folder`.
    pub worktrees_folder: &'a str,
    /// The folder the worktree was in before it was moved or removed.
    pub worktree: &'a Path,
}

/// The steps that delete the folders that held the worktree, from the one that held it upwards, each only when it is
/// empty by then, inside the folder of the worktrees: never that folder itself, nor anything outside it.
///
/// Feature: worktree.remove-empty-folders
#[must_use]
pub fn remove_empty_folders(input: &EmptyFoldersInput<'_>) -> Vec<Step> {
    let base = slashed(input.worktrees_folder).to_lowercase();
    let mut steps = Vec::new();
    let mut folder = input.worktree.parent();
    while let Some(parent) = folder {
        let path = slashed(&parent.to_string_lossy());
        let is_inside = path
            .to_lowercase()
            .strip_prefix(&base)
            .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1);
        if !is_inside {
            break;
        }
        steps.push(Step::DeleteEmptyFolder {
            path: PathBuf::from(path),
        });
        folder = parent.parent();
    }
    steps
}
