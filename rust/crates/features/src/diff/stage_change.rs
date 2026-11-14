//! `diff.stage-change`: puts one change of the working tree into the index, from the **+** button of its band in the
//! gutter of a Diff page.

use model::contract::{Answers, Context, Outcome, Refusal};
use model::diff::DiffSide;
use text::diff::{ChangeLines, ChangePatchInput, PatchDirection, change_patch};

use super::change::{
    CHANGED, apply_plan, index_plan, is_where_shown, patch_refusal, read_versions, target_change, text_of, versions,
    with_file,
};

/// Said when the change has nothing left to stage.
const STAGED_ALREADY: &str = "This change is staged already.";
/// Said when the page is not of the working tree against `HEAD` or the index.
const NOT_OF_THE_WORKING_TREE: &str = "Only a change of the working tree against HEAD or the index can be staged.";
/// What takes it back, since Undo does not.
const TAKEN_BACK_BY: &str = "Unstage this change takes it back: the index has no reflog.";

/// Reads the index and the working tree again, checks that the change is where the page showed it, and gives its patch
/// (`diff.change-patch`) to `git apply --cached --unidiff-zero --whitespace=nowarn -`. A file not in the index has no
/// index to patch: it is added whole with `git add -- <file>`.
///
/// Feature: diff.stage-change
#[must_use]
pub fn stage_change(context: &Context, answers: &Answers) -> Outcome {
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
    if !is_where_shown(change, versions) || versions.working_tree.is_none() {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let title = format!("Stage change in {}", change.path);
    let folder = context.repository.folder.clone();
    let Some(index) = versions.index.as_deref() else {
        let step = with_file(folder, &["add"], &change.path);
        return Outcome::Run(index_plan(title, vec![step], TAKEN_BACK_BY));
    };
    let working_tree = versions.working_tree.as_deref();
    let (index_text, working_text) = (text_of(Some(index)), text_of(working_tree));
    let patch = change_patch(&ChangePatchInput {
        path: &change.path,
        index: &index_text,
        other: &working_text,
        direction: PatchDirection::Stage,
        change: ChangeLines::Other(change.right_lines.clone()),
    });
    let Some(patch) = patch else {
        return Outcome::Refuse(Refusal::saying(STAGED_ALREADY));
    };
    if let Some(refusal) = patch_refusal(&patch, &[Some(index), working_tree]) {
        return Outcome::Refuse(refusal);
    }
    Outcome::Run(apply_plan(context, title, (patch, TAKEN_BACK_BY)))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
