//! A file one side deleted and the other changed (`review.deleted-card`): what the side that kept it changed of the
//! base, as a diff with the lines between the changes left out.

use git::read::conflicts::ConflictFile;
use git::read::merge_view::{Stage, side_arguments};
use git::{Git, GitError};
use model::diff::LineChange;
use model::review::{DeletedFile, KeptChanges, KeptRow, Which};

use super::{LARGEST_FILE, changes_between};

/// How many lines alike are shown around each change.
const CONTEXT: usize = 3;
/// The most rows shown before the changes are too many.
const MOST_ROWS: usize = 400;

/// What the side that kept a file changed of it; none for a file that is not text or is larger than 2 MB.
pub(super) fn read(git: &Git, file: &ConflictFile, deleted_by: Which) -> Result<Option<DeletedFile>, GitError> {
    let kept = match deleted_by {
        Which::Current => Stage::Incoming,
        Which::Incoming => Stage::Current,
    };
    let base = git.run_read(&side_arguments(&file.path, Stage::Base))?;
    let side = git.run_read(&side_arguments(&file.path, kept))?;
    let is_text = |bytes: &[u8]| bytes.len() <= LARGEST_FILE && !bytes.contains(&0);
    if !is_text(&base) || !is_text(&side) {
        return Ok(None);
    }
    let (base, side) = (String::from_utf8_lossy(&base), String::from_utf8_lossy(&side));
    let base: Vec<&str> = text::diff::lines(&base);
    let side: Vec<&str> = text::diff::lines(&side);
    Ok(Some(DeletedFile {
        path: file.path.clone(),
        deleted_by,
        changes: kept_changes(&base, &side),
    }))
}

/// The changes from the base to the side that kept the file, with 3 lines alike around each and the others left out.
#[must_use]
pub fn kept_changes(base: &[&str], side: &[&str]) -> KeptChanges {
    let changes = changes_between(base, side);
    if changes.is_empty() {
        return KeptChanges::Nothing;
    }
    let mut rows = Vec::new();
    let mut shown_until = 0;
    for (index, change) in changes.iter().enumerate() {
        let from = change.right.start.saturating_sub(CONTEXT).max(shown_until);
        if from > shown_until {
            rows.push(KeptRow::LeftOut);
        }
        same(side, from..change.right.start, &mut rows);
        rows.extend(
            base.get(change.left.clone())
                .unwrap_or_default()
                .iter()
                .map(|line| KeptRow::Removed((*line).to_owned())),
        );
        rows.extend(
            change
                .right
                .clone()
                .map(|number| KeptRow::Added(number + 1, side.get(number).copied().unwrap_or_default().to_owned())),
        );
        let next = changes
            .get(index + 1)
            .map_or(side.len(), |next: &LineChange| next.right.start);
        let until = (change.right.end + CONTEXT).min(next);
        same(side, change.right.end..until, &mut rows);
        shown_until = until;
        if rows.len() > MOST_ROWS {
            return KeptChanges::TooMany;
        }
    }
    if shown_until < side.len() {
        rows.push(KeptRow::LeftOut);
    }
    KeptChanges::Rows(rows)
}

/// The lines alike of a stretch of the side, by their numbers.
fn same(side: &[&str], lines: std::ops::Range<usize>, rows: &mut Vec<KeptRow>) {
    rows.extend(
        lines.map(|number| KeptRow::Same(number + 1, side.get(number).copied().unwrap_or_default().to_owned())),
    );
}
