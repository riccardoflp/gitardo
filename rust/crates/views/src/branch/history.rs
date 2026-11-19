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
use crate::text::range_diff as range_diff_words;

/// The key under which `range-diff.compare` takes what a branch is compared with: `upstream` for its upstream.
const RANGE_DIFF_WITH: AnswerKey = AnswerKey("range-diff.with");

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
    entries.extend(range_diff_entries(repository, target));
    entries
}

/// **What its last rewrite changed** for a local branch, with **Compare with `<upstream>` commit by commit** when it
/// and its upstream each have commits the other has not, as of the last fetch; **What the force push changed** for a
/// remote branch (`range-diff.compare`).
pub(crate) fn range_diff_entries(repository: &RepositoryState, target: &Target) -> Vec<OperationEntry> {
    match target {
        Target::Branch(name) => {
            let mut entries = vec![OperationEntry::runs(
                range_diff_words::LAST_REWRITE,
                "range-diff.compare",
                target.clone(),
            )];
            entries.extend(compare_with_upstream(repository, name));
            entries
        }
        Target::RemoteBranch(_) => vec![OperationEntry::runs(
            range_diff_words::FORCE_PUSH,
            "range-diff.compare",
            target.clone(),
        )],
        _ => Vec::new(),
    }
}

/// **Compare with `<upstream>` commit by commit** for a local branch that has commits its upstream has not and the
/// other way round, as of the last fetch: what a force push would replace.
pub(crate) fn compare_with_upstream(repository: &RepositoryState, name: &str) -> Option<OperationEntry> {
    let branch = repository.branches.iter().find(|branch| branch.name == name)?;
    let upstream = branch
        .upstream
        .as_ref()
        .filter(|upstream| !upstream.is_gone && upstream.ahead > 0 && upstream.behind > 0)?;
    let shown = upstream
        .reference
        .strip_prefix("refs/remotes/")
        .unwrap_or(&upstream.reference);
    let answers = Answers::new().with(RANGE_DIFF_WITH, Answer::Choice(String::from("upstream")));
    Some(OperationEntry::runs_with(
        range_diff_words::compare_with_upstream(shown),
        "range-diff.compare",
        Target::Branch(name.to_owned()),
        answers,
    ))
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
