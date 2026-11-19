//! `diff.unstage-lines`: takes only the lines selected back out of the index, the working tree unchanged (D78).

use model::contract::{Answers, Context, Outcome, Refusal};
use model::diff::DiffSide;
use text::diff::{LinesPatchInput, PatchDirection, lines_patch};

use super::change::{
    CHANGED, apply_plan, index_plan, is_where_shown, patch_refusal, read_versions, target_change, text_of, versions,
    with_file,
};
use super::lines::{counted, placed_in_index, rows_changed, union};

/// Said when no line selected differs between `HEAD` and the index.
const NOTHING_STAGED: &str = "Nothing of these lines is staged.";
/// Said when the page is not against `HEAD`, or not of the index or the working tree.
const NOT_AGAINST_HEAD: &str = "Only lines of the index or the working tree against HEAD can be unstaged.";
/// What takes it back, since Undo does not.
const TAKEN_BACK_BY: &str = "Stage lines on the same lines takes it back: the index has no reflog.";

/// Reads `HEAD`, the index and the working tree again, checks that the lines are where the page showed them, places
/// the rows selected in the index, and gives the patch of `diff.lines-patch` from the index towards `HEAD` to `git apply
/// --cached --unidiff-zero --whitespace=nowarn -`. A file `HEAD` does not have, all of whose lines are selected, is
/// unstaged whole, as `diff.unstage-change` does.
///
/// Feature: diff.unstage-lines
#[must_use]
pub fn unstage_lines(context: &Context, answers: &Answers) -> Outcome {
    let change = match target_change(context) {
        Ok(change) => change,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    if change.left != DiffSide::Head || !matches!(change.right, DiffSide::Index | DiffSide::WorkingTree) {
        return Outcome::Refuse(Refusal::saying(NOT_AGAINST_HEAD));
    }
    let repository = &context.repository;
    let Some(versions) = versions(answers) else {
        return read_versions(change, repository.head.commit.as_ref());
    };
    if !is_where_shown(change, versions) {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let Some(index) = versions.index.as_deref() else {
        return Outcome::Refuse(Refusal::saying(NOTHING_STAGED));
    };
    let head = versions.commit.as_deref();
    let (index_text, head_text) = (text_of(Some(index)), text_of(head));
    let index_lines = if change.right == DiffSide::Index {
        change.right_lines.clone()
    } else {
        let working_text = text_of(versions.working_tree.as_deref());
        union(
            &placed_in_index(&working_text, &index_text, &change.right_lines),
            &placed_in_index(&head_text, &index_text, &change.left_lines),
        )
    };
    let every_line = text::diff::lines(&index_text).len();
    if head.is_none() && index_lines.start == 0 && index_lines.end >= every_line {
        let folder = repository.folder.clone();
        let step = if repository.has_commit() {
            with_file(folder, &["restore", "--staged"], &change.path)
        } else {
            with_file(folder, &["rm", "--cached", "-r"], &change.path)
        };
        let title = format!("Unstage {}", change.path);
        return Outcome::Run(index_plan(title, vec![step], TAKEN_BACK_BY));
    }
    let patch = lines_patch(&LinesPatchInput {
        path: &change.path,
        index: &index_text,
        other: &head_text,
        direction: PatchDirection::Unstage,
        index_lines,
        other_lines: change.left_lines.clone(),
        new_file: None,
    });
    let Some(patch) = patch else {
        return Outcome::Refuse(Refusal::saying(NOTHING_STAGED));
    };
    if let Some(refusal) = patch_refusal(&patch, &[Some(index), head]) {
        return Outcome::Refuse(refusal);
    }
    let title = format!("Unstage {} in {}", counted(rows_changed(&patch)), change.path);
    Outcome::Run(apply_plan(context, title, (patch, TAKEN_BACK_BY)))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
