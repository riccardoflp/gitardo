//! The JSON of what a feature runs on: [`TargetJson`], read into the engine's `Target` and written back for an
//! operation offered.

use std::path::PathBuf;

use model::CommitId;
use model::contract::Target;
use serde::{Deserialize, Serialize};

use super::resolutions::{FileResolutionsJson, MergeResultJson};

/// What a feature was invoked on (`Target` of `model`). The kinds with a structure of their own come with the areas that
/// use them: the resolutions of the review and the file of a Merge page with the conflicts (1.3); a change of a diff, a
/// comparison, files saved, a file at a commit, the settings of new worktrees, lines, a line of a file and a file of a
/// list with theirs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TargetJson {
    /// What is checked out: the toolbar and the palette.
    Head,
    /// A commit, by its full id.
    Commit {
        /// Its id.
        commit: String,
    },
    /// A local branch.
    Branch {
        /// Its short name.
        name: String,
    },
    /// A remote branch, `origin/main`.
    RemoteBranch {
        /// Its short name.
        name: String,
    },
    /// A tag.
    Tag {
        /// Its name.
        name: String,
    },
    /// A stash, by its index.
    Stash {
        /// `N` of `stash@{N}`.
        index: usize,
    },
    /// A remote.
    Remote {
        /// Its name.
        name: String,
    },
    /// Files of the working tree.
    Files {
        /// Their paths, from the top of the working tree.
        paths: Vec<String>,
    },
    /// A file of the working tree.
    File {
        /// Its path, from the top of the working tree.
        path: String,
    },
    /// The uncommitted changes.
    Changes,
    /// Commits selected together.
    Commits {
        /// Their ids.
        commits: Vec<String>,
    },
    /// A submodule.
    Submodule {
        /// Its folder, from the top of the working tree.
        path: String,
    },
    /// A worktree.
    Worktree {
        /// Its folder.
        folder: String,
    },
    /// A file or a folder.
    Path {
        /// Where it is.
        path: String,
    },
    /// The resolutions accepted on the review of the conflicts.
    Resolutions {
        /// The files of the review, in its order, with or without any accepted.
        files: Vec<FileResolutionsJson>,
    },
    /// The file of a Merge page.
    MergeResult(MergeResultJson),
}

/// A commit id that is not one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotACommit(pub String);

impl std::fmt::Display for NotACommit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} is not the full id of a commit.", self.0)
    }
}

fn commit(text: &str) -> Result<CommitId, NotACommit> {
    CommitId::parse(text).ok_or_else(|| NotACommit(text.to_owned()))
}

impl TryFrom<TargetJson> for Target {
    type Error = NotACommit;

    fn try_from(target: TargetJson) -> Result<Target, NotACommit> {
        Ok(match target {
            TargetJson::Head => Target::Head,
            TargetJson::Commit { commit: id } => Target::Commit(commit(&id)?),
            TargetJson::Branch { name } => Target::Branch(name),
            TargetJson::RemoteBranch { name } => Target::RemoteBranch(name),
            TargetJson::Tag { name } => Target::Tag(name),
            TargetJson::Stash { index } => Target::Stash(index),
            TargetJson::Remote { name } => Target::Remote(name),
            TargetJson::Files { paths } => Target::Files(paths),
            TargetJson::File { path } => Target::File(path),
            TargetJson::Changes => Target::Changes,
            TargetJson::Commits { commits } => {
                Target::Commits(commits.iter().map(|id| commit(id)).collect::<Result<_, _>>()?)
            }
            TargetJson::Submodule { path } => Target::Submodule(path),
            TargetJson::Worktree { folder } => Target::Worktree(PathBuf::from(folder)),
            TargetJson::Path { path } => Target::Path(PathBuf::from(path)),
            TargetJson::Resolutions { files } => Target::Resolutions(Box::new(model::review::Resolutions {
                files: files.into_iter().map(Into::into).collect(),
            })),
            TargetJson::MergeResult(result) => Target::MergeResult(Box::new(result.into())),
        })
    }
}

impl TargetJson {
    /// The JSON of a target, for an operation the extension is offered to run; none for the kinds that come with the
    /// areas that use them.
    #[must_use]
    pub fn of(target: &Target) -> Option<TargetJson> {
        let path = |path: &std::path::Path| path.to_string_lossy().into_owned();
        Some(match target {
            Target::Head => TargetJson::Head,
            Target::Commit(commit) => TargetJson::Commit {
                commit: commit.as_str().to_owned(),
            },
            Target::Branch(name) => TargetJson::Branch { name: name.clone() },
            Target::RemoteBranch(name) => TargetJson::RemoteBranch { name: name.clone() },
            Target::Tag(name) => TargetJson::Tag { name: name.clone() },
            Target::Stash(index) => TargetJson::Stash { index: *index },
            Target::Remote(name) => TargetJson::Remote { name: name.clone() },
            Target::Files(paths) => TargetJson::Files { paths: paths.clone() },
            Target::File(file) => TargetJson::File { path: file.clone() },
            Target::Changes => TargetJson::Changes,
            Target::Commits(commits) => TargetJson::Commits {
                commits: commits.iter().map(|commit| commit.as_str().to_owned()).collect(),
            },
            Target::Submodule(folder) => TargetJson::Submodule { path: folder.clone() },
            Target::Worktree(folder) => TargetJson::Worktree { folder: path(folder) },
            Target::Path(where_it_is) => TargetJson::Path {
                path: path(where_it_is),
            },
            _ => return None,
        })
    }
}
