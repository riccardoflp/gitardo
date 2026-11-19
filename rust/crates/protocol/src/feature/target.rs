//! The JSON of what a feature runs on: [`TargetJson`], read into the engine's `Target` and written back for an
//! operation offered.

use std::path::PathBuf;

use model::CommitId;
use model::contract::Target;
use serde::{Deserialize, Serialize};

use model::compare::{CompareSide, ComparisonPart, ComparisonTarget, Side};

use super::diff_change::DiffChangeJson;
use super::resolutions::{FileResolutionsJson, MergeResultJson};
use crate::reads::{CompareModeJson, ComparisonSideJson};
use crate::state::FileChangeJson;

/// What a feature was invoked on (`Target` of `model`). The kinds with a structure of their own come with the areas that
/// use them: the resolutions of the review and the file of a Merge page with the conflicts (1.3), a comparison page as a
/// whole with AI and GitHub (1.6), a file of a comparison and a change of a diff with stage 10 (1.7); files saved, a
/// file at a commit, the settings of new worktrees, lines, a line of a file and a file of a list with theirs.
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
    /// A comparison page, as a whole (1.6, `compare.open-on-github`).
    Comparison {
        /// The first side.
        first: CompareSideJson,
        /// The second side.
        second: CompareSideJson,
        /// How they are compared.
        mode: CompareModeJson,
        /// The file or folder it is narrowed to; none for the whole repository.
        #[serde(default)]
        path: Option<String>,
        /// Where the two diverged, as the page last read it.
        #[serde(default)]
        merge_base: Option<String>,
        /// The file of the page it was invoked on, as the page lists it (1.7, `compare.take-file`); none for the page as
        /// a whole.
        #[serde(default)]
        file: Option<FileChangeJson>,
    },
    /// A change of a diff (1.7, `diff.stage-change` and `diff.unstage-change`).
    DiffChange(DiffChangeJson),
}

/// A side of a comparison page: its name, where it is now, and whether it follows its name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSideJson {
    /// Its name: a branch, a tag, a revision, a hash, or **Working tree**.
    pub name: String,
    /// Where it is now.
    pub side: ComparisonSideJson,
    /// Whether it follows its name as it moves; a side given by its hash is fixed.
    pub is_followed: bool,
    /// Whether its name no longer stands for a commit.
    pub is_gone: bool,
}

impl TryFrom<CompareSideJson> for CompareSide {
    type Error = NotACommit;

    fn try_from(side: CompareSideJson) -> Result<CompareSide, NotACommit> {
        Ok(CompareSide {
            side: Side::try_from(&side.side)?,
            name: side.name,
            is_followed: side.is_followed,
            is_gone: side.is_gone,
        })
    }
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
                recorded: Vec::new(),
            })),
            TargetJson::MergeResult(result) => Target::MergeResult(Box::new(result.into())),
            TargetJson::Comparison {
                first,
                second,
                mode,
                path,
                merge_base,
                file,
            } => Target::Comparison(Box::new(ComparisonTarget {
                first: first.try_into()?,
                second: second.try_into()?,
                mode: mode.into(),
                path,
                merge_base: merge_base.as_deref().map(commit).transpose()?,
                part: file
                    .as_ref()
                    .map_or(ComparisonPart::Whole, |file| ComparisonPart::File(file.into())),
            })),
            TargetJson::DiffChange(change) => Target::DiffChange(Box::new(change.try_into()?)),
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
