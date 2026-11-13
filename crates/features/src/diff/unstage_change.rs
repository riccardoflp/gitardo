//! `diff.unstage-change`: takes one change back out of the index, from the **−** button of its band in the gutter of a
//! Diff page.

use model::contract::{Answers, Context, Outcome, Refusal};
use model::diff::DiffSide;
use text::diff::{ChangeLines, ChangePatchInput, PatchDirection, change_patch};

use super::change::{
    CHANGED, apply_plan, index_plan, is_where_shown, patch_refusal, read_versions, target_change, text_of, versions,
    with_file,
};

/// Said when nothing of the change is in the index.
const NOTHING_STAGED: &str = "Nothing of this change is staged.";
/// Said when the page is not against `HEAD`, or not of the index or the working tree.
const NOT_AGAINST_HEAD: &str = "Only a change of the index or the working tree against HEAD can be unstaged.";
/// What takes it back, since Undo does not.
const TAKEN_BACK_BY: &str = "Stage this change takes it back: the index has no reflog.";

/// Reads `HEAD` and the index again, checks that the change is where the page showed it, and gives the patch that takes
/// it out of the index towards `HEAD` (`diff.change-patch`) to `git apply --cached --unidiff-zero --whitespace=nowarn
/// -`. A file in the index and not in `HEAD` is one change: `git restore --staged -- <file>`, or
/// `git rm --cached -r -- <file>` before the first commit.
///
/// Feature: diff.unstage-change
#[must_use]
pub fn unstage_change(context: &Context, answers: &Answers) -> Outcome {
    let change = match target_change(context) {
        Ok(change) => change,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let in_index = match change.right {
        DiffSide::Index => ChangeLines::Index(change.right_lines.clone()),
        DiffSide::WorkingTree => ChangeLines::Other(change.left_lines.clone()),
        DiffSide::Head | DiffSide::Commit(_) | DiffSide::Empty => {
            return Outcome::Refuse(Refusal::saying(NOT_AGAINST_HEAD));
        }
    };
    if change.left != DiffSide::Head {
        return Outcome::Refuse(Refusal::saying(NOT_AGAINST_HEAD));
    }
    let repository = &context.repository;
    let Some(versions) = versions(answers) else {
        return read_versions(change, repository.head.commit.as_ref());
    };
    if !is_where_shown(change, versions) {
        return Outcome::Refuse(Refusal::saying(CHANGED));
    }
    let title = format!("Unstage change in {}", change.path);
    let folder = repository.folder.clone();
    let (Some(index), head) = (versions.index.as_deref(), versions.commit.as_deref()) else {
        return Outcome::Refuse(Refusal::saying(NOTHING_STAGED));
    };
    let Some(head) = head else {
        let step = if repository.has_commit() {
            with_file(folder, &["restore", "--staged"], &change.path)
        } else {
            with_file(folder, &["rm", "--cached", "-r"], &change.path)
        };
        return Outcome::Run(index_plan(title, vec![step], TAKEN_BACK_BY));
    };
    let (index_text, head_text) = (text_of(Some(index)), text_of(Some(head)));
    let patch = change_patch(&ChangePatchInput {
        path: &change.path,
        index: &index_text,
        other: &head_text,
        direction: PatchDirection::Unstage,
        change: in_index,
    });
    let Some(patch) = patch else {
        return Outcome::Refuse(Refusal::saying(NOTHING_STAGED));
    };
    if let Some(refusal) = patch_refusal(&patch, &[Some(index), Some(head)]) {
        return Outcome::Refuse(refusal);
    }
    Outcome::Run(apply_plan(context, title, (patch, TAKEN_BACK_BY)))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
