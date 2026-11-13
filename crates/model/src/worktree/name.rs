//! The name of a new worktree, or of a worktree renamed, as it is typed: what is wrong with it, and where its folder
//! goes.

use std::path::{Path, PathBuf};

use crate::branch::validate_name;
use crate::contract::Text;

use super::{folder_in, folder_part, slashed};

/// Where the folder of a worktree named in a box goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderBase {
    /// Inside the folder of the worktrees, at the path the name gives, part by part between its slashes.
    Worktrees(String),
    /// Beside a worktree that is elsewhere: in this folder, under the name with its slashes made `-`.
    Beside(PathBuf),
}

impl FolderBase {
    /// The folder a name gives.
    #[must_use]
    pub fn folder_for(&self, name: &str) -> PathBuf {
        match self {
            FolderBase::Worktrees(folder) => folder_in(folder, name),
            FolderBase::Beside(parent) => {
                let part = folder_part(&name.replace(['/', '\\'], "-"));
                let part = if part.is_empty() {
                    String::from("worktree")
                } else {
                    part
                };
                PathBuf::from(format!("{}/{part}", slashed(&parent.to_string_lossy())))
            }
        }
    }
}

/// What a name typed for a worktree must be (`worktree.create`, `worktree.create-from-branch`,
/// `worktree.create-at-commit`, `worktree.rename`): a name git takes for a branch when it names one, not a branch
/// already, nor one git cannot have beside a branch; and a folder no worktree has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeNameRule {
    /// The branches already there, by their short names, when the name is the name of a new branch too.
    pub branches: Option<Vec<String>>,
    /// The name taken when nothing is typed, as the short hash of a detached worktree; none when a name is needed.
    pub empty_name: Option<String>,
    /// Where the folder goes.
    pub base: FolderBase,
    /// The folders of the worktrees there are, which a new one cannot take.
    pub taken_folders: Vec<PathBuf>,
}

impl WorktreeNameRule {
    /// What is wrong with a name typed, or nothing when it can be taken.
    #[must_use]
    pub fn problem(&self, typed: &str) -> Option<Text> {
        let Some(name) = self.name_of(typed) else {
            return Some(Text::from("Enter a name"));
        };
        if let Some(branches) = self.branches.as_ref().filter(|_| !typed.is_empty()) {
            if let Some(problem) = validate_name(typed) {
                return Some(problem);
            }
            if branches.iter().any(|branch| branch == typed) {
                return Some(Text::from("A branch with that name already exists"));
            }
            if let Some(branch) = branches.iter().find(|branch| is_beside(branch, typed)) {
                return Some(Text::from(format!("Git cannot have it beside the branch `{branch}`")));
            }
        }
        let folder = self.base.folder_for(name);
        self.is_taken(&folder)
            .then(|| Text::from(format!("`{}` already exists", folder.display())))
    }

    /// Where the folder goes, said under the box while nothing is wrong with the name.
    #[must_use]
    pub fn hint(&self, typed: &str) -> Option<Text> {
        if self.problem(typed).is_some() {
            return None;
        }
        let name = self.name_of(typed)?;
        Some(Text::from(format!("In `{}`", self.base.folder_for(name).display())))
    }

    /// The name the folder is made of: what is typed, or the name taken when nothing is.
    fn name_of<'a>(&'a self, typed: &'a str) -> Option<&'a str> {
        if typed.trim().is_empty() {
            self.empty_name.as_deref()
        } else {
            Some(typed)
        }
    }

    /// Whether a folder is the folder of a worktree there is, compared without regard to case or to the slashes.
    fn is_taken(&self, folder: &Path) -> bool {
        is_listed(&self.taken_folders, folder)
    }
}

/// Whether a folder is among some, compared without regard to case or to the kind of slashes.
#[must_use]
pub fn is_listed(folders: &[PathBuf], folder: &Path) -> bool {
    let folder = slashed(&folder.to_string_lossy());
    folders
        .iter()
        .any(|taken| slashed(&taken.to_string_lossy()).eq_ignore_ascii_case(&folder))
}

/// Whether git cannot have two branches side by side: one is a folder of the other, as `feature` and `feature/login`.
fn is_beside(branch: &str, name: &str) -> bool {
    name.strip_prefix(branch).is_some_and(|rest| rest.starts_with('/'))
        || branch.strip_prefix(name).is_some_and(|rest| rest.starts_with('/'))
}
