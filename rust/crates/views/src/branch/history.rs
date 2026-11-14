//! The entries of the menus that compare a commit, a branch, a remote branch or a tag with something else, copy or save
//! a commit as a patch, and start or go on with a bisect.

use model::CommitId;
use model::commit::Commit;
use model::contract::{Answer, AnswerKey, Answers, Target};
use model::state::RepositoryState;

/// The key under which `bisect.start` and `bisect.mark` take the mark the entry chosen gives: `good` or `bad`.
const MARK: AnswerKey = AnswerKey("mark");

use super::ReferenceMenus;
use super::menu::OperationEntry;
use crate::text::bisect as words;
use crate::text::compare as compare_words;
use crate::text::patch as patch_words;

/// **Compare with…** and, when it is not at the commit checked out, **Compare with `<current branch>`** (**Compare with
/// HEAD** when no branch is checked out), for a commit, a branch, a remote branch or a tag at a commit.
pub(crate) fn compare_entries(menus: ReferenceMenus<'_>, target: &Target, commit: &CommitId) -> Vec<OperationEntry> {
    let repository = menus.repository;
    let mut entries = vec![OperationEntry::runs(
        compare_words::COMPARE_WITH,
        "compare.with",
        target.clone(),
    )];
    if let Some(head) = &repository.head.commit
        && head != commit
    {
        let current = repository.checked_out_branch().unwrap_or("HEAD");
        entries.push(OperationEntry::runs(
            compare_words::compare_with_current(current),
            "compare.with-current",
            target.clone(),
        ));
    }
    entries
}

/// The entries of a commit of the timeline beyond those of `branch.`: the comparisons, with the commit selected before
/// when another one is; **Copy as patch** and **Save as patch…** unless it is a merge; and the bisect, started or
/// marked.
pub(crate) fn history_entries(
    menus: ReferenceMenus<'_>,
    commit: &Commit,
    selected_before: Option<&CommitId>,
) -> Vec<OperationEntry> {
    let target = Target::Commit(commit.id.clone());
    let mut entries = compare_entries(menus, &target, &commit.id);
    if let Some(before) = selected_before.filter(|before| **before != commit.id) {
        entries.push(OperationEntry::runs(
            compare_words::compare_with_selected(before.short()),
            "compare.selected-before",
            Target::Commits(vec![before.clone(), commit.id.clone()]),
        ));
    }
    if !commit.is_merge() {
        entries.push(OperationEntry::runs(
            patch_words::COPY_AS_PATCH,
            "patch.copy",
            target.clone(),
        ));
        entries.push(OperationEntry::runs(
            patch_words::SAVE_AS_PATCH,
            "patch.save",
            target.clone(),
        ));
    }
    entries.extend(bisect_entries(menus.repository, &target));
    entries
}

/// **Start bisect: this commit is bad** and **…good** while no bisect nor other operation is under way, **Bisect: mark
/// as good** and **…bad** while a bisect is.
fn bisect_entries(repository: &RepositoryState, target: &Target) -> Vec<OperationEntry> {
    if repository.operation.is_some() {
        return Vec::new();
    }
    let marked = |label: &str, feature: &'static str, mark: &str| {
        let answers = Answers::new().with(MARK, Answer::Choice(mark.to_owned()));
        OperationEntry::runs_with(label, feature, target.clone(), answers)
    };
    if repository.bisect.is_some() {
        vec![
            marked(words::MARK_GOOD, "bisect.mark", "good"),
            marked(words::MARK_BAD, "bisect.mark", "bad"),
        ]
    } else {
        vec![
            marked(words::START_BAD, "bisect.start", "bad"),
            marked(words::START_GOOD, "bisect.start", "good"),
        ]
    }
}
