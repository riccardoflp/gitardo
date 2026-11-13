//! Comparisons: the comparison page of two sides (`compare.page`) with its parts, the sides swapped (`compare.swap`),
//! the mode (`compare.set-mode`), the box that narrows it (`compare.narrow-to-path`) and the commits only in each side
//! (`compare.commits`); and the comparison of two commits in the inspector (`compare.in-place`).

mod in_place;
mod page;
mod parts;
#[cfg(test)]
mod tests;

pub use in_place::{InPlaceRead, InPlaceState, in_place};
pub use page::page;
pub use parts::{commits, narrow_to_path, set_mode, swap};

use model::CommitId;
use model::compare::{CompareMode, CompareSide, CompareView, ComparisonTarget, InPlaceSides, NamedCommit, Side};
use model::state::FileChange;

use crate::diff::{DiffTarget, DiffVersion, listed_file};

/// The number of a comparison page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComparePageId(pub u64);

/// How far a comparison is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareRead<'a> {
    /// Not read yet: **Comparing…**.
    Comparing,
    /// Read.
    Read(&'a CompareView),
    /// Git could not compare the two.
    Failed,
}

/// What the comparison page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompareState<'a> {
    /// The page.
    pub id: ComparePageId,
    /// Its title in the trail.
    pub title: &'a str,
    /// The two sides, how they are compared and the path it is narrowed to.
    pub target: &'a ComparisonTarget,
    /// The comparison, as far as it is read.
    pub read: CompareRead<'a>,
    /// Whether it is being compared again after a change.
    pub is_reading: bool,
    /// What is typed in the box that narrows it.
    pub typed_path: &'a str,
    /// How many commits only in the first and in the second side the comparison counted before it was narrowed.
    pub whole_counts: Option<(usize, usize)>,
    /// Whether **Show all N commits…** was clicked.
    pub is_all_commits_shown: bool,
    /// When the repository was last read, in seconds since 1970: how long ago each commit was made is counted from it.
    pub now: i64,
}

/// What is asked of a comparison page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompareIntent {
    /// A click on a side: another in its place (`compare.change-side`), the first when true.
    ChangeSide(ComparePageId, bool),
    /// The arrows between the sides (`compare.swap`).
    Swap(ComparePageId),
    /// A mode button (`compare.set-mode`).
    SetMode(ComparePageId, CompareMode),
    /// Text typed in the box that narrows it.
    TypePath(ComparePageId, String),
    /// `Enter` in the box: narrows to what is typed.
    Narrow(ComparePageId),
    /// `Escape` in the box: the path in use is put back.
    ResetPath(ComparePageId),
    /// **×**: every file again.
    ClearPath(ComparePageId),
    /// A click on a short hash or a commit: its page.
    OpenCommit(ComparePageId, CommitId),
    /// **Show all N commits…**.
    ShowAllCommits(ComparePageId),
    /// A click on a file: its Diff page between the two sides, by its place in the list.
    OpenFile(ComparePageId, usize),
    /// **Open in the Diff page**: every file listed.
    OpenAll(ComparePageId),
    /// The arrow of a file (`compare.take-file`), by its place in the list.
    TakeFile(ComparePageId, usize),
    /// **Copy as patch** (`compare.copy-patch`).
    CopyPatch(ComparePageId),
    /// The close button of the comparison in the inspector.
    CloseInPlace,
    /// **Swap** in the inspector.
    SwapInPlace,
    /// **Open as a page** in the inspector.
    InPlaceAsPage,
    /// A click on a file of the comparison in the inspector, by its place.
    InPlaceFile(usize),
    /// The button that opens every file of the comparison in the inspector in one Diff page.
    InPlaceAll,
}

/// The path a comparison is narrowed to from what is typed: spaces around it dropped, `\` made `/`, a leading `./` or
/// `/` and a trailing `/` dropped; none when nothing is left or only `.`.
#[must_use]
pub fn narrowed_path(typed: &str) -> Option<String> {
    let slashed = typed.trim().replace('\\', "/");
    let mut path = slashed.as_str();
    loop {
        if let Some(rest) = path.strip_prefix("./") {
            path = rest;
        } else if let Some(rest) = path.strip_prefix('/') {
            path = rest;
        } else {
            break;
        }
    }
    let path = path.trim_end_matches('/');
    (!path.is_empty() && path != ".").then(|| path.to_owned())
}

/// The version of a side in the Diff page: its commit under its name, or the working tree.
fn version(side: &CompareSide) -> DiffVersion {
    match &side.side {
        Side::WorkingTree => DiffVersion::WorkingTree,
        Side::Commit(commit) => DiffVersion::Commit {
            commit: commit.clone(),
            name: side.name.clone(),
        },
    }
}

/// The Diff page of files of a comparison between its two sides as the mode says: from where they diverged to the
/// second side for **Since they diverged**, when they did, from the first side to the second otherwise. A page of
/// several files is titled as the comparison page.
#[must_use]
pub fn diff_target(target: &ComparisonTarget, merge_base: Option<&CommitId>, files: &[FileChange]) -> DiffTarget {
    let left = match (target.mode, merge_base, &target.first.side) {
        (CompareMode::SinceDiverged, Some(base), Side::Commit(_)) => DiffVersion::commit(base.clone()),
        _ => version(&target.first),
    };
    let title = (files.len() != 1)
        .then(|| crate::text::compare::title(&target.first.name, &target.second.name, target.path.as_deref()));
    DiffTarget {
        left,
        right: version(&target.second),
        files: files.iter().map(listed_file).collect(),
        title,
    }
}

/// The Diff page of files of the comparison in the inspector, from its first commit to its second, titled
/// `<from> ↔ <to>` when it has several.
#[must_use]
pub fn in_place_diff_target(sides: &InPlaceSides, files: &[FileChange]) -> DiffTarget {
    let named = |side: &NamedCommit| DiffVersion::Commit {
        commit: side.commit.clone(),
        name: side.name.clone(),
    };
    let title = (files.len() != 1).then(|| crate::text::compare::title(&sides.from.name, &sides.to.name, None));
    DiffTarget {
        left: named(&sides.from),
        right: named(&sides.to),
        files: files.iter().map(listed_file).collect(),
        title,
    }
}
