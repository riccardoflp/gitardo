//! The entries of the operations in the menu of a local branch, a remote branch, a tag or a commit, each with the
//! operation it runs, or greyed out while its operation comes in a later milestone.

use model::CommitId;
use model::branch::{OpenBranchInput, OpenedReference, can_fast_forward, local_name, open, rebase_onto};
use model::contract::{Answers, FeatureId, OpenIn, Target};
use model::state::{Branch, RemoteBranch, RepositoryState, Tag};
use model::worktree::{OtherWorktreeInput, WorktreeBranch, branch_in_another_worktree};
use ui::{MenuEntry, MenuItem};

use super::ReferenceMenus;
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::branch as words;
use crate::text::history as history_words;
use crate::text::rail as rail_words;
use crate::text::worktree as worktree_words;

/// An entry of a menu that runs an operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperationEntry {
    /// What it says.
    pub(crate) label: String,
    /// The operation and what it runs on; none while the operation comes in a later milestone, and it is greyed out.
    pub(crate) run: Option<(FeatureId, Target)>,
    /// The answers the entry gives the operation before it asks, as **Start bisect: this commit is bad** does.
    pub(crate) answers: Answers,
}

impl OperationEntry {
    /// An entry that runs an operation.
    pub(crate) fn runs(label: impl Into<String>, feature: &'static str, target: Target) -> Self {
        Self::runs_with(label, feature, target, Answers::new())
    }

    /// An entry that runs an operation with some of its questions answered.
    pub(crate) fn runs_with(label: impl Into<String>, feature: &'static str, target: Target, answers: Answers) -> Self {
        OperationEntry {
            label: label.into(),
            run: Some((FeatureId(feature), target)),
            answers,
        }
    }
}

/// The operations of a local branch. The one checked out here can be renamed or branched from; one another worktree
/// has gets the entries of that worktree and no **Check out**; any other is checked out, or opened in a worktree, as
/// `branches.openIn` puts first, fast-forwarded when it is only behind its upstream, renamed and deleted.
pub(crate) fn branch_entries(menus: ReferenceMenus<'_>, branch: &Branch) -> Vec<OperationEntry> {
    let state = menus.repository;
    let name = branch.name.as_str();
    let target = || Target::Branch(name.to_owned());
    let create = OperationEntry::runs(words::CREATE_BRANCH_HERE, "branch.create", target());
    if branch.is_checked_out {
        return vec![
            create,
            OperationEntry::runs(rail_words::RENAME, "branch.rename", target()),
        ];
    }
    let fast_forward = branch
        .upstream
        .as_ref()
        .filter(|_| can_fast_forward(state, branch))
        .map(|upstream| {
            let short = upstream
                .reference
                .strip_prefix("refs/remotes/")
                .unwrap_or(&upstream.reference);
            OperationEntry::runs(words::fast_forward_to(short), "branch.fast-forward", target())
        });
    if let Some(found) = in_another_worktree(menus, name) {
        let (mut entries, changing) = crate::worktree::other_branch_entries(menus, found);
        entries.push(create);
        entries.extend(fast_forward);
        entries.extend(merge_entries(state, &branch.commit, target));
        entries.extend(crate::rebase::rebase_onto_entry(state, &branch.commit, target()));
        entries.push(OperationEntry::runs(words::RENAME_BRANCH, "branch.rename", target()));
        entries.extend(changing);
        return entries;
    }
    let mut entries = opening(
        menus.open_in,
        OperationEntry::runs(rail_words::check_out(name), "branch.checkout", target()),
        target(),
    );
    entries.push(create);
    entries.push(OperationEntry::runs(
        worktree_words::CREATE_WORKTREE,
        "worktree.create-from-branch",
        target(),
    ));
    entries.extend(fast_forward);
    entries.extend(merge_entries(state, &branch.commit, target));
    entries.extend(crate::rebase::rebase_onto_entry(state, &branch.commit, target()));
    entries.push(OperationEntry::runs(rail_words::RENAME, "branch.rename", target()));
    entries.push(OperationEntry::runs(rail_words::DELETE, "branch.delete", target()));
    entries
}

/// The worktree other than the one the views show that has a local branch, and whether it is renamed and removed from
/// here (`worktree.branch-in-another-worktree`).
fn in_another_worktree<'a>(menus: ReferenceMenus<'a>, branch: &'a str) -> Option<WorktreeBranch<'a>> {
    branch_in_another_worktree(&OtherWorktreeInput {
        state: menus.repository,
        branch,
        window_folder: menus.window.folder,
    })
}

/// **Check out** and **Open in worktree**, the one a double click does first.
fn opening(open_in: OpenIn, check_out: OperationEntry, target: Target) -> Vec<OperationEntry> {
    let in_worktree = OperationEntry::runs(words::OPEN_IN_WORKTREE, "worktree.open-in-worktree", target);
    match open_in {
        OpenIn::Checkout => vec![check_out, in_worktree],
        OpenIn::Worktree => vec![in_worktree, check_out],
    }
}

/// The operations of a remote branch: checked out as the local branch of its name, or as a new one; checked out and
/// rebased onto it when its local branch has commits of its own and commits to take; branched from; and deleted from
/// its remote (`remote.delete-branch`).
pub(crate) fn remote_entries(menus: ReferenceMenus<'_>, remote: &RemoteBranch) -> Vec<OperationEntry> {
    let state = menus.repository;
    let name = local_name(&remote.name, &state.remotes);
    let target = || Target::RemoteBranch(remote.name.clone());
    let mut entries = if let Some(found) = in_another_worktree(menus, name) {
        crate::worktree::other_branch_entries(menus, found).0
    } else {
        let label = if state.branches.iter().any(|branch| branch.name == name) {
            words::check_out_local(name)
        } else {
            rail_words::CHECK_OUT_AS_LOCAL.to_owned()
        };
        opening(
            menus.open_in,
            OperationEntry::runs(label, "branch.checkout-remote", target()),
            target(),
        )
    };
    if let Some(local) = rebase_onto(state, &remote.name) {
        entries.push(OperationEntry::runs(
            words::check_out_and_rebase(&local.name),
            "branch.checkout-remote-and-rebase",
            target(),
        ));
    }
    entries.extend(crate::rebase::rebase_onto_entry(state, &remote.commit, target()));
    entries.push(OperationEntry::runs(
        words::CREATE_BRANCH_HERE,
        "branch.create",
        target(),
    ));
    entries.extend(merge_entries(state, &remote.commit, target));
    entries.push(OperationEntry::runs(
        rail_words::DELETE_FROM_REMOTE,
        "remote.delete-branch",
        target(),
    ));
    entries
}

/// The operations of a tag: checked out on a detached `HEAD` unless its commit is, branched from, pushed when the
/// repository has a remote, and deleted.
pub(crate) fn tag_entries(menus: ReferenceMenus<'_>, tag: &Tag) -> Vec<OperationEntry> {
    let state = menus.repository;
    let target = || Target::Tag(tag.name.clone());
    let mut entries = Vec::new();
    if state.head.commit.as_ref() != Some(&tag.commit) {
        entries.push(OperationEntry::runs(words::CHECK_OUT_TAG, "tag.checkout", target()));
    }
    entries.push(OperationEntry::runs(
        words::CREATE_BRANCH_HERE,
        "branch.create",
        target(),
    ));
    if !state.remotes.is_empty() {
        entries.push(OperationEntry::runs(words::PUSH_TAG, "tag.push", target()));
    }
    entries.push(OperationEntry::runs(rail_words::DELETE, "tag.delete", target()));
    entries
}

/// The operations of a commit of the timeline: a branch or a tag made there; the commit checked out on a detached
/// `HEAD`, merged or cherry-picked unless it is the one checked out; reverted; the branch checked out reset to it; and
/// its message edited while no operation is under way.
pub(crate) fn commit_entries(menus: ReferenceMenus<'_>, commit: &CommitId) -> Vec<OperationEntry> {
    let state = menus.repository;
    let target = || Target::Commit(commit.clone());
    let is_checked_out = state.head.commit.as_ref() == Some(commit);
    let mut entries = vec![
        OperationEntry::runs(words::CREATE_BRANCH_HERE, "branch.create", target()),
        OperationEntry::runs(words::CREATE_TAG_HERE, "tag.create", target()),
    ];
    if !is_checked_out {
        entries.push(OperationEntry::runs(
            words::CHECK_OUT_COMMIT,
            "branch.checkout-detached",
            target(),
        ));
    }
    entries.push(OperationEntry::runs(
        worktree_words::CREATE_WORKTREE_HERE,
        "worktree.create-at-commit",
        target(),
    ));
    entries.extend(merge_entries(state, commit, target));
    if !is_checked_out {
        entries.push(OperationEntry::runs(
            history_words::cherry_pick(commit.short()),
            "cherry-pick.commit",
            target(),
        ));
    }
    entries.push(OperationEntry::runs(
        history_words::revert(commit.short()),
        "revert.commit",
        target(),
    ));
    entries.push(OperationEntry::runs(
        history_words::reset_to_here(current(state)),
        "reset.to-commit",
        target(),
    ));
    if state.operation.is_none() {
        entries.push(OperationEntry::runs(
            history_words::EDIT_MESSAGE,
            "message.edit",
            target(),
        ));
    }
    entries.push(OperationEntry::runs(words::COPY_HASH, "commits.copy-hash", target()));
    entries.push(OperationEntry::runs(
        words::COPY_MESSAGE,
        "commits.copy-message",
        target(),
    ));
    entries
}

/// **Merge into `<current>`** and **Merge into `<current>`…**, which asks how, for a commit or the tip of a branch:
/// none before the first commit, nor for the commit checked out.
fn merge_entries(state: &RepositoryState, tip: &CommitId, target: impl Fn() -> Target) -> Vec<OperationEntry> {
    if state.head.commit.as_ref().is_none_or(|head| head == tip) {
        return Vec::new();
    }
    let current = current(state);
    vec![
        OperationEntry::runs(history_words::merge_into(current), "merge.into-current", target()),
        OperationEntry::runs(
            history_words::merge_into_choosing(current),
            "merge.choose-way",
            target(),
        ),
    ]
}

/// What is checked out, as the entries say it: the branch, or `HEAD` when detached.
fn current(state: &RepositoryState) -> &str {
    state.checked_out_branch().unwrap_or("HEAD")
}

/// The items of the entries, each carrying the value of its place among them, greyed out while its operation comes
/// later.
pub(crate) fn menu_items<Value>(
    entries: &[OperationEntry],
    value: impl Fn(usize) -> Value,
) -> Vec<MenuEntry<'_, Value>> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let item = MenuItem::new(entry.label.as_str(), value(index));
            MenuEntry::Item(if entry.run.is_some() { item } else { item.disabled() })
        })
        .collect()
}

/// What the entry at a place asks: its operation, run on its reference.
pub(crate) fn chosen(entries: &[OperationEntry], index: usize) -> Option<Intent> {
    let entry = entries.get(index)?;
    let (feature, target) = entry.run.clone()?;
    if entry.answers == Answers::new() {
        Some(Intent::Feature(FeatureIntent::Run(feature, target)))
    } else {
        Some(Intent::Feature(FeatureIntent::RunWith(
            feature,
            target,
            entry.answers.clone(),
        )))
    }
}

/// What a double click on a branch or a remote branch asks, as `branch.open` decides.
pub(crate) fn open_intent(menus: ReferenceMenus<'_>, reference: OpenedReference<'_>) -> Option<Intent> {
    let input = OpenBranchInput {
        state: menus.repository,
        reference,
        open_in: menus.open_in,
    };
    open(&input).map(|(feature, target)| Intent::Feature(FeatureIntent::Run(feature, target)))
}
