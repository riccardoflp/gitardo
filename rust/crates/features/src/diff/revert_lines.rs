//! `diff.revert-lines`: puts back in the working tree what the left side of a Diff page has in place of the lines
//! selected (D78).

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Text, Undo};
use model::diff::{DiffChange, DiffSide};
use text::diff::{LinesPatchInput, PatchDirection, lines_taken};

use super::change::{CHANGED, is_where_shown, read_versions, side_text, target_change, text_of, versions};
use super::lines::counted;

/// Said when the right side of the page is not the working tree.
const NOT_OF_THE_WORKING_TREE: &str = "Only lines of the working tree can be reverted.";
/// Said when no line selected differs between the two sides.
const NOTHING_TO_REVERT: &str = "These lines are as the left side has them.";
/// Why Undo does not take it back.
const NOT_UNDONE: &str = "The file is written with the lines of the left side; Undo does not take it back.";

/// Reads the working tree and the left side again, checks that the lines are where the page showed them, replaces the
/// lines of the rows selected in the working tree by those the left side has on those rows, in the line endings of
/// the file, and writes the file: `write <file>`. The window does it as one edit of the working tree side, which
/// `Ctrl+Z` takes back, with `reverted_lines`.
///
/// Feature: diff.revert-lines
#[must_use]
pub fn revert_lines(context: &Context, answers: &Answers) -> Outcome {
    let change = match target_change(context) {
        Ok(change) => change,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    if change.right != DiffSide::WorkingTree {
        return Outcome::Refuse(Refusal::saying(NOT_OF_THE_WORKING_TREE));
    }
    let commit = match &change.left {
        DiffSide::Head => context.repository.head.commit.as_ref(),
        DiffSide::Commit(commit) => Some(commit),
        DiffSide::Index | DiffSide::WorkingTree | DiffSide::Empty => None,
    };
    let Some(versions) = versions(answers) else {
        return read_versions(change, commit);
    };
    let Some(working_tree) = versions.working_tree.as_deref() else {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    };
    if !is_where_shown(change, versions) {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let left = side_text(&change.left, versions).unwrap_or_default();
    let Some(contents) = reverted_lines(working_tree, left, change) else {
        return Outcome::Refuse(Refusal::saying(NOTHING_TO_REVERT));
    };
    let rows = change.left_lines.len().max(change.right_lines.len());
    Outcome::Run(Plan {
        title: Text::from(format!("Revert {} in {}", counted(rows), change.path)),
        steps: vec![Step::Write {
            path: context.repository.folder.join(&change.path),
            contents,
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_UNDONE)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The working tree with the lines of the rows of a selection replaced by those the left side has on them: of each
/// difference, a line of the working tree selected is taken out and a line of the left side selected put in, in the
/// line endings of the file. None when no line selected differs.
#[must_use]
pub fn reverted_lines(working_tree: &[u8], left: &[u8], selection: &DiffChange) -> Option<Vec<u8>> {
    let (working_text, left_text) = (text_of(Some(working_tree)), text_of(Some(left)));
    let taken = lines_taken(&LinesPatchInput {
        path: &selection.path,
        index: &working_text,
        other: &left_text,
        direction: PatchDirection::Stage,
        index_lines: selection.right_lines.clone(),
        other_lines: selection.left_lines.clone(),
        new_file: None,
    })?;
    Some(taken.into_bytes())
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
