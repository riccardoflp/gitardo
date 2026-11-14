//! The rows of a file of a Diff page kept in step with the text of its working tree side while it is edited, until the
//! diff is computed again (`diff.edit`): the lines typed show at once where they are, the lines after them move down
//! or up, and what the diff said of the lines edited waits for it to be computed again.

use std::ops::Range;

use model::diff::{AlignedRow, LineChange, RowLine};
use text::diff::line_ranges;
use views::diff::DiffText;

/// Where each row of a file went once its right side followed an edit: its new place, none for a row taken out.
pub(super) type RowMap = Vec<Option<usize>>;

/// The lines of the right side that an edit replaced: from the first line that differs, so many lines that were there
/// and so many that are there now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Replaced {
    /// The first line that differs.
    start: usize,
    /// How many lines were there.
    removed: usize,
    /// How many lines are there now.
    added: usize,
}

/// Makes the right side of a file the text given, as an edit left it, and moves its rows to follow: the lines that
/// differ take the rows of the lines they replaced, with no marks, those added get rows of their own, those taken out
/// leave filler rows, and the lines after them move. Says where each row went, none when the text is the same.
pub(super) fn follow_text(text: &mut DiffText, right: String) -> Option<RowMap> {
    if text.right == right {
        return None;
    }
    let ranges = line_ranges(&right);
    let replaced = replaced(text, (&right, &ranges));
    text.right = right;
    text.right_lines = ranges;
    let (rows, map) = moved_rows(&text.aligned.rows, replaced);
    let blank = vec![Vec::new(); replaced.added];
    let start = replaced.start.min(text.right_colors.len());
    let end = (replaced.start + replaced.removed).min(text.right_colors.len());
    let _old: Vec<_> = text.right_colors.splice(start..end, blank).collect();
    text.aligned.rows = rows;
    changes_again(text);
    text.folds = moved_folds(&text.folds, &map);
    Some(map)
}

/// The lines that differ between the right side as it was and the text given: what comes before them and after them
/// is the same.
fn replaced(text: &DiffText, (right, ranges): (&str, &[Range<usize>])) -> Replaced {
    let line = |source: &str, range: &Range<usize>| source.get(range.clone()).unwrap_or_default().to_owned();
    let old: Vec<String> = text.right_lines.iter().map(|range| line(&text.right, range)).collect();
    let new: Vec<String> = ranges.iter().map(|range| line(right, range)).collect();
    let start = old.iter().zip(&new).take_while(|(one, other)| one == other).count();
    let most = old.len().min(new.len()) - start;
    let end = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(most)
        .take_while(|(one, other)| one == other)
        .count();
    Replaced {
        start,
        removed: old.len() - start - end,
        added: new.len() - start - end,
    }
}

/// The rows once the lines replaced are in place, and where each row went.
fn moved_rows(rows: &[AlignedRow], replaced: Replaced) -> (Vec<AlignedRow>, RowMap) {
    let end = replaced.start + replaced.removed;
    let in_block = |row: &AlignedRow| {
        row.right
            .as_ref()
            .is_some_and(|line| (replaced.start..end).contains(&line.line))
    };
    let last_in_block = rows.iter().rposition(in_block);
    let insert_at = last_in_block.map_or_else(
        || {
            rows.iter()
                .position(|row| row.right.as_ref().is_some_and(|line| line.line >= end))
                .unwrap_or(rows.len())
        },
        |last| last + 1,
    );
    let mut moved = Vec::with_capacity(rows.len() + replaced.added);
    let mut map = Vec::with_capacity(rows.len());
    let mut taken = 0;
    for (index, row) in rows.iter().enumerate() {
        if index == insert_at {
            push_added(&mut moved, (rows, index), (replaced, taken));
        }
        let mut row = row.clone();
        if in_block(&row) {
            row.right = (taken < replaced.added).then(|| RowLine {
                line: replaced.start + taken,
                marks: Vec::new(),
            });
            row.left.iter_mut().for_each(|line| line.marks.clear());
            taken += 1;
        } else if let Some(line) = row.right.as_mut().filter(|line| line.line >= end) {
            line.line = line.line + replaced.added - replaced.removed;
        }
        if row.left.is_none() && row.right.is_none() {
            map.push(None);
            continue;
        }
        map.push(Some(moved.len()));
        moved.push(row);
    }
    if insert_at >= rows.len() {
        push_added(&mut moved, (rows, rows.len()), (replaced, taken));
    }
    (moved, map)
}

/// The rows of the lines added past those that took the rows of the lines replaced, before a row: of the change of the
/// rows around them when both are of the same change, of none otherwise.
fn push_added(
    moved: &mut Vec<AlignedRow>,
    (rows, before): (&[AlignedRow], usize),
    (replaced, taken): (Replaced, usize),
) {
    let change_before = before
        .checked_sub(1)
        .and_then(|index| rows.get(index))
        .and_then(|row| row.change);
    let change_after = rows.get(before).and_then(|row| row.change);
    let change = change_before.filter(|_| change_before == change_after);
    for line in taken..replaced.added {
        moved.push(AlignedRow {
            left: None,
            right: Some(RowLine {
                line: replaced.start + line,
                marks: Vec::new(),
            }),
            change,
        });
    }
}

/// The changes made again from the rows: the rows of each, its lines on the right where its rows have them, a change
/// left with no rows taken out with what is said of it.
fn changes_again(text: &mut DiffText) {
    let count = text.aligned.changes.len();
    let mut kept = vec![None; count];
    let mut next = 0;
    for (change, slot) in kept.iter_mut().enumerate() {
        if text.aligned.rows.iter().any(|row| row.change == Some(change)) {
            *slot = Some(next);
            next += 1;
        }
    }
    for row in &mut text.aligned.rows {
        row.change = row.change.and_then(|change| kept.get(change).copied().flatten());
    }
    let keep = |index: usize| kept.get(index).copied().flatten().is_some();
    let mut index = 0;
    text.aligned.changes.retain(|_| {
        index += 1;
        keep(index - 1)
    });
    index = 0;
    text.changes.retain(|_| {
        index += 1;
        keep(index - 1)
    });
    if text.staged.len() == count {
        index = 0;
        text.staged.retain(|_| {
            index += 1;
            keep(index - 1)
        });
    }
    let rows = &text.aligned.rows;
    for (change, (place, lines)) in text.aligned.changes.iter_mut().zip(&mut text.changes).enumerate() {
        let of_change: Vec<usize> = (0..rows.len())
            .filter(|row| rows.get(*row).is_some_and(|aligned| aligned.change == Some(change)))
            .collect();
        let (Some(first), Some(last)) = (of_change.first(), of_change.last()) else {
            continue;
        };
        place.rows = *first..last + 1;
        *lines = LineChange {
            left: lines.left.clone(),
            right: right_lines(rows, place.rows.clone()),
        };
    }
}

/// The lines of the right side that some rows have, or the place after the last line before them when they have none.
fn right_lines(rows: &[AlignedRow], of: Range<usize>) -> Range<usize> {
    let lines: Vec<usize> = rows
        .get(of.clone())
        .unwrap_or_default()
        .iter()
        .filter_map(|row| row.right.as_ref().map(|line| line.line))
        .collect();
    if let (Some(first), Some(last)) = (lines.first(), lines.last()) {
        return *first..last + 1;
    }
    let before = rows
        .get(..of.start)
        .unwrap_or_default()
        .iter()
        .rev()
        .find_map(|row| row.right.as_ref().map(|line| line.line + 1))
        .unwrap_or(0);
    before..before
}

/// The folds once the rows moved: a fold whose rows all moved keeps them in their new places; one that lost a row is
/// gone, until the diff is computed again.
fn moved_folds(folds: &[Range<usize>], map: &RowMap) -> Vec<Range<usize>> {
    folds
        .iter()
        .filter_map(|fold| {
            let rows: Option<Vec<usize>> = map.get(fold.clone())?.iter().copied().collect();
            let rows = rows?;
            let (first, last) = (rows.first()?, rows.last()?);
            (last + 1 - first == fold.len()).then(|| *first..last + 1)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::diff::{AlignInput, align};

    /// The rows of two texts, as the worker computes them, with no word marks.
    fn diff_of(left: &str, right: &str) -> DiffText {
        let left_lines = line_ranges(left);
        let right_lines = line_ranges(right);
        let lines = |text: &str, ranges: &[Range<usize>]| -> Vec<String> {
            ranges.iter().map(|range| text[range.clone()].to_owned()).collect()
        };
        let (left_text, right_text) = (lines(left, &left_lines), lines(right, &right_lines));
        let left_refs: Vec<&str> = left_text.iter().map(String::as_str).collect();
        let right_refs: Vec<&str> = right_text.iter().map(String::as_str).collect();
        let found = text::diff::line_diff(&text::diff::LineDiffInput {
            left: &left_refs,
            right: &right_refs,
            keep_going: &|| true,
        })
        .unwrap();
        let changes: Vec<LineChange> = found
            .iter()
            .map(|change| LineChange {
                left: change.left.lines(),
                right: change.right.lines(),
            })
            .collect();
        let aligned = align(&AlignInput {
            left: &left_refs,
            right: &right_refs,
            changes: &changes,
            word_marks: &|_, _| (Vec::new(), Vec::new()),
        });
        DiffText {
            left: left.to_owned(),
            right: right.to_owned(),
            sides: (true, true),
            left_lines,
            right_lines,
            changes,
            aligned,
            ..DiffText::default()
        }
    }

    /// The text of each row, left and right, with `~` for a filler row.
    fn shown(text: &DiffText) -> Vec<(String, String)> {
        text.aligned
            .rows
            .iter()
            .map(|row| {
                let side = |line: Option<&RowLine>, of: fn(&DiffText, usize) -> &str| {
                    line.map_or_else(|| String::from("~"), |line| of(text, line.line).to_owned())
                };
                (
                    side(row.left.as_ref(), DiffText::left_line),
                    side(row.right.as_ref(), DiffText::right_line),
                )
            })
            .collect()
    }

    fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
        rows.iter()
            .map(|(left, right)| ((*left).to_owned(), (*right).to_owned()))
            .collect()
    }

    #[test]
    fn a_line_typed_gets_a_row_of_its_own_and_the_lines_after_it_move_down() {
        let mut text = diff_of("a\nb\nc\n", "a\nb\nc\n");
        let map = follow_text(&mut text, String::from("a\nnew\nb\nc\n")).unwrap();
        assert_eq!(shown(&text), pairs(&[("a", "a"), ("~", "new"), ("b", "b"), ("c", "c")]));
        assert_eq!(map, [Some(0), Some(2), Some(3)]);
        assert_eq!(
            text.aligned.changes.len(),
            0,
            "the line waits for the diff to be colored"
        );
    }

    #[test]
    fn a_line_changed_keeps_its_row_and_its_change() {
        let mut text = diff_of("a\nb\nc\n", "a\nB\nc\n");
        let _ = follow_text(&mut text, String::from("a\nBee\nc\n")).unwrap();
        assert_eq!(shown(&text), pairs(&[("a", "a"), ("b", "Bee"), ("c", "c")]));
        assert_eq!(text.aligned.changes.len(), 1);
        assert_eq!(text.aligned.changes[0].rows, 1..2);
        assert_eq!(text.changes[0].right, 1..2);
    }

    #[test]
    fn a_line_taken_out_leaves_a_filler_row_where_the_left_side_has_a_line() {
        let mut text = diff_of("a\nb\nc\n", "a\nb\nc\n");
        let _ = follow_text(&mut text, String::from("a\nc\n")).unwrap();
        assert_eq!(shown(&text), pairs(&[("a", "a"), ("b", "~"), ("c", "c")]));
    }

    #[test]
    fn an_added_line_taken_out_takes_its_row_and_its_change_with_it() {
        let mut text = diff_of("a\nc\n", "a\nb\nc\n");
        assert_eq!(text.aligned.changes.len(), 1);
        let _ = follow_text(&mut text, String::from("a\nc\n")).unwrap();
        assert_eq!(shown(&text), pairs(&[("a", "a"), ("c", "c")]));
        assert_eq!(text.aligned.changes.len(), 0);
        assert_eq!(text.changes.len(), 0);
    }

    #[test]
    fn lines_typed_at_the_end_go_after_the_last_row() {
        let mut text = diff_of("a\n", "a\n");
        let _ = follow_text(&mut text, String::from("a\nb\nc")).unwrap();
        assert_eq!(shown(&text), pairs(&[("a", "a"), ("~", "b"), ("~", "c")]));
    }

    #[test]
    fn the_same_text_moves_nothing() {
        let mut text = diff_of("a\n", "a\n");
        assert_eq!(follow_text(&mut text, String::from("a\n")), None);
    }
}
