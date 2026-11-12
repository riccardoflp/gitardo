//! Which changes of the working tree against `HEAD` are in the index already (`diff.staged-changes`).

use std::ops::RangeInclusive;

use super::LineChange;

/// The changes of the working tree against `HEAD`, and the differences left between the index and the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedInput<'a> {
    /// The changes of the diff of the working tree against `HEAD`, in order.
    pub changes: &'a [LineChange],
    /// The changes between the index and the working tree; none when the file is not in the index, as an untracked
    /// file is not.
    pub unstaged: Option<&'a [LineChange]>,
}

/// For each change of the working tree against `HEAD`, whether it is staged: no difference between the index and the
/// working tree touches its lines in the working tree. Lines are placed as hunk headers place them: an insertion sits
/// between two lines, so two insertions at the same place touch, and a change of lines also touches the place just
/// before its first line. A file not in the index has no change staged.
///
/// Feature: diff.staged-changes
#[must_use]
pub fn staged_changes(input: &StagedInput<'_>) -> Vec<bool> {
    let Some(unstaged) = input.unstaged else {
        return vec![false; input.changes.len()];
    };
    let left: Vec<RangeInclusive<usize>> = unstaged.iter().map(place).collect();
    input
        .changes
        .iter()
        .map(|change| {
            let here = place(change);
            !left
                .iter()
                .any(|other| other.start() <= here.end() && here.start() <= other.end())
        })
        .collect()
}

/// Where a change is in the working tree, in halves of a line: line `n` (from 1) is `2n`, and the place between lines
/// `n` and `n + 1` is `2n + 1`. A change of lines covers from the place before its first line to its last line; an
/// insertion covers the place it sits at.
fn place(change: &LineChange) -> RangeInclusive<usize> {
    let lines = &change.right;
    let before = 2 * lines.start + 1;
    if lines.is_empty() {
        before..=before
    } else {
        before..=2 * lines.end
    }
}
