//! The lines typed in a page that shows the blame of its file, marked not committed at once (`blame.mark-edited-lines`),
//! so that every line keeps something beside it until git is asked again.

use model::blame::{LineEdit, mark_edited_lines};
use ui::editor::Change;
use views::file_view::FilePageId;

use crate::store::state::State;

/// An edit of the text of a page: the range it replaced, from its start to its end, each a line and a column from 0,
/// and the text put in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineChange {
    /// Where the range started.
    pub start: (usize, usize),
    /// Where it ended.
    pub end: (usize, usize),
    /// What was put in its place.
    pub text: String,
}

/// The edits some changes made, from the text before them and the text after: each change is taken in its turn, in the
/// text as the changes before it left it.
#[must_use]
pub fn line_changes(before: &str, changes: &[Change], after: &str) -> Vec<LineChange> {
    let mut text = before.to_owned();
    let mut edits = Vec::with_capacity(changes.len());
    for change in changes {
        let end = (change.at + change.removed).min(text.len());
        let start = change.at.min(end);
        let put = after
            .get(change.at..change.at + change.inserted)
            .unwrap_or_default()
            .to_owned();
        edits.push(LineChange {
            start: position(&text, start),
            end: position(&text, end),
            text: put.clone(),
        });
        if text.is_char_boundary(start) && text.is_char_boundary(end) {
            text.replace_range(start..end, &put);
        }
    }
    edits
}

/// The line and the column, from 0, of a byte of a text.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count();
    let column = before
        .rfind('\n')
        .map_or(before.len(), |found| before.len() - found - 1);
    (line, column)
}

/// Marks the lines some edits touched in a page that shows the blame of its file as not committed yet.
pub fn mark_edits(state: &mut State, id: FilePageId, changes: &[LineChange]) {
    if changes.is_empty() {
        return;
    }
    let Some(lines) = state.blames.pages.get_mut(&id).and_then(|page| page.lines.as_mut()) else {
        return;
    };
    let edits: Vec<LineEdit<'_>> = changes
        .iter()
        .map(|change| LineEdit {
            start: change.start,
            end: change.end,
            text: &change.text,
        })
        .collect();
    *lines = mark_edited_lines(lines, &edits);
}

#[cfg(test)]
mod tests {
    use ui::editor::{Change, LinesChanged};

    use super::{LineChange, line_changes};

    #[test]
    fn a_line_deleted_whole_is_from_the_start_of_it_to_the_start_of_the_next() {
        let before = "a\nb\nc\n";
        let change = Change {
            at: 2,
            removed: 2,
            inserted: 0,
            lines: LinesChanged {
                start: 1,
                removed: 2,
                added: 1,
            },
        };
        let edits = line_changes(before, &[change], "a\nc\n");
        assert_eq!(
            edits,
            [LineChange {
                start: (1, 0),
                end: (2, 0),
                text: String::new(),
            }]
        );
    }
}
