//! The long runs of unchanged rows of a diff folded into a bar, so that the changes are seen together
//! (`diff.collapse-unchanged`).

use std::collections::BTreeSet;
use std::ops::Range;

use super::AlignedDiff;

/// How many unchanged lines stay in sight next to a change.
pub const FOLD_KEEPS: usize = 3;
/// The fewest lines a fold hides: a shorter run stays in sight.
pub const FOLD_LEAST: usize = 4;

/// The runs of unchanged rows to fold, in order: each run keeps 3 rows in sight next to each change, none on the side
/// away from the changes at the start and the end of the file, and is folded only when that hides 4 rows or more. A
/// diff with no change folds nothing. The two sides fold the same rows.
#[must_use]
pub fn unchanged_folds(diff: &AlignedDiff) -> Vec<Range<usize>> {
    if diff.changes.is_empty() {
        return Vec::new();
    }
    let mut folds = Vec::new();
    let mut start = 0;
    for change in &diff.changes {
        let is_first = start == 0;
        let keep_before = if is_first { 0 } else { FOLD_KEEPS };
        push_fold(
            &mut folds,
            (start + keep_before)..change.rows.start.saturating_sub(FOLD_KEEPS),
        );
        start = change.rows.end;
    }
    push_fold(&mut folds, (start + FOLD_KEEPS)..diff.rows.len());
    folds
}

/// Adds a run to fold, when it hides enough rows.
fn push_fold(folds: &mut Vec<Range<usize>>, run: Range<usize>) {
    if run.end >= run.start + FOLD_LEAST {
        folds.push(run);
    }
}

/// A row of the page as it is shown: a row of the diff, or the bar of a fold.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShownRow {
    /// A row of the diff, by its place.
    Row(usize),
    /// The bar of a folded run of rows: **⋯ 12 unchanged lines**.
    Folded(Range<usize>),
}

/// The rows shown, the folds left folded taking one row each: those of `folds` but the ones whose first row is in
/// `unfolded`.
#[must_use]
pub fn shown_rows(row_count: usize, folds: &[Range<usize>], unfolded: &BTreeSet<usize>) -> Vec<ShownRow> {
    let mut shown = Vec::with_capacity(row_count.min(4096));
    let mut row = 0;
    for fold in folds.iter().filter(|fold| !unfolded.contains(&fold.start)) {
        shown.extend((row..fold.start).map(ShownRow::Row));
        shown.push(ShownRow::Folded(fold.clone()));
        row = fold.end;
    }
    shown.extend((row..row_count).map(ShownRow::Row));
    shown
}
