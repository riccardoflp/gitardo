//! The rows of a Diff page made from the diffs of its files: the header of each file, its rows with the unchanged ones
//! folded, or its note; and where each change starts among them.

use model::diff::{ShownRow, shown_rows};
use views::diff::{ChangePlace, FileDiff, PageLine};

use super::DiffPage;

/// Makes the rows of a page and the places of its changes again, after a file was read or a fold changed; the current
/// change is kept while there are as many.
pub(super) fn rebuild(page: &mut DiffPage) {
    page.lines.clear();
    page.changes.clear();
    for (file, diff) in page.diffs.iter().enumerate() {
        page.lines.push(PageLine::Header(file));
        let FileDiff::Text(text) = diff else {
            page.lines.push(PageLine::Note(file));
            continue;
        };
        if text.aligned.rows.is_empty() {
            page.lines.push(PageLine::Note(file));
            continue;
        }
        let folds = if page.is_collapsed { text.folds.as_slice() } else { &[] };
        let unfolded = page.unfolded.get(file).cloned().unwrap_or_default();
        let mut starts = text.aligned.changes.iter().enumerate().peekable();
        for shown in shown_rows(text.aligned.rows.len(), folds, &unfolded) {
            match shown {
                ShownRow::Row(row) => {
                    if let Some((change, _)) = starts.next_if(|(_, change)| change.rows.start == row) {
                        page.changes.push(ChangePlace {
                            file,
                            change,
                            line: page.lines.len(),
                        });
                    }
                    page.lines.push(PageLine::Row(file, row));
                }
                ShownRow::Folded(rows) => page.lines.push(PageLine::Folded(file, rows)),
            }
        }
    }
    if page.current.is_some_and(|current| current >= page.changes.len()) {
        page.current = None;
    }
}
