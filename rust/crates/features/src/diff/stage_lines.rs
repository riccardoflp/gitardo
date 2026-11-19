//! `diff.stage-lines`: puts into the index only the lines selected of the changes of the working tree (D78).

use model::contract::{Answers, Context, Outcome, Refusal};
use model::diff::DiffSide;
use text::diff::{LinesPatchInput, PatchDirection, lines_patch};

use super::change::{
    CHANGED, apply_plan, is_where_shown, patch_refusal, read_versions, target_change, text_of, versions,
};
use super::lines::{counted, placed_in_index, rows_changed, union};

/// Said when no line selected differs between the index and the working tree.
const STAGED_ALREADY: &str = "These lines are staged already.";
/// Said when the page is not of the working tree against `HEAD` or the index.
const NOT_OF_THE_WORKING_TREE: &str = "Only lines of the working tree against HEAD or the index can be staged.";
/// What takes it back, since Undo does not.
const TAKEN_BACK_BY: &str = "Unstage lines on the same lines takes it back: the index has no reflog.";

/// Reads `HEAD`, the index and the working tree again, checks that the lines are where the page showed them, places
/// the rows selected in the index, and gives the patch of `diff.lines-patch` to `git apply --cached --unidiff-zero
/// --whitespace=nowarn -`. A file not in the index is created there with the lines selected alone.
///
/// Feature: diff.stage-lines
#[must_use]
pub fn stage_lines(context: &Context, answers: &Answers) -> Outcome {
    let change = match target_change(context) {
        Ok(change) => change,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    if change.right != DiffSide::WorkingTree || !matches!(change.left, DiffSide::Head | DiffSide::Index) {
        return Outcome::Refuse(Refusal::saying(NOT_OF_THE_WORKING_TREE));
    }
    let Some(versions) = versions(answers) else {
        return read_versions(change, context.repository.head.commit.as_ref());
    };
    let Some(working_tree) = versions.working_tree.as_deref() else {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    };
    if !is_where_shown(change, versions) {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let working_text = text_of(Some(working_tree));
    let index = versions.index.as_deref();
    let index_text = text_of(index);
    let index_lines = match (index, &change.left) {
        (None, _) => 0..0,
        (Some(_), DiffSide::Index) => change.left_lines.clone(),
        (Some(_), _) => {
            let head_text = text_of(versions.commit.as_deref());
            union(
                &placed_in_index(&head_text, &index_text, &change.left_lines),
                &placed_in_index(&working_text, &index_text, &change.right_lines),
            )
        }
    };
    let patch = lines_patch(&LinesPatchInput {
        path: &change.path,
        index: &index_text,
        other: &working_text,
        direction: PatchDirection::Stage,
        index_lines,
        other_lines: change.right_lines.clone(),
        new_file: index.is_none().then_some("100644"),
    });
    let Some(patch) = patch else {
        return Outcome::Refuse(Refusal::saying(STAGED_ALREADY));
    };
    if let Some(refusal) = patch_refusal(&patch, &[index, Some(working_tree)]) {
        return Outcome::Refuse(refusal);
    }
    let title = format!("Stage {} in {}", counted(rows_changed(&patch)), change.path);
    Outcome::Run(apply_plan(context, title, (patch, TAKEN_BACK_BY)))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
