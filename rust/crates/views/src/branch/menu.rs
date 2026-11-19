//! The entries of the operations in the menu of a local branch, a remote branch, a tag or a commit, each with the
//! operation it runs, or greyed out while its operation comes in a later milestone.

use model::CommitId;
use model::branch::{OpenBranchInput, OpenedReference, can_fast_forward, local_name, open, rebase_onto};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, OpenIn, Target};
use model::state::{Branch, RemoteBranch, RepositoryState, Tag};
use model::worktree::{OtherWorktreeInput, WorktreeBranch, branch_in_another_worktree};
use ui::MenuEntry;

use super::ReferenceMenus;
use crate::feature::FeatureIntent;
use crate::icons;
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
    /// The address it opens in the browser in place of an operation: a pull request (`github.open-pull-request`).
    pub(crate) url: Option<String>,
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
            url: None,
        }
    }

    /// An entry that opens the pull request of a branch, or GitHub's page to create one (`github.open-pull-request`).
    pub(crate) fn opens(link: model::github::PullLink) -> Self {
        OperationEntry {
            label: link.label.as_str().to_owned(),
            run: None,
            answers: Answers::new(),
            url: Some(link.url),
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
    let link = menus.github.link_of_branch(branch).map(OperationEntry::opens);
    if branch.is_checked_out {
        let mut entries = vec![create];
        entries.extend(link);
        entries.extend(upstream_entries(state, branch));
        entries.push(OperationEntry::runs(rail_words::RENAME, "branch.rename", target()));
        return entries;
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
        entries.extend(link);
        entries.extend(upstream_entries(state, branch));
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
    entries.extend(link);
    entries.extend(upstream_entries(state, branch));
    entries.push(OperationEntry::runs(rail_words::RENAME, "branch.rename", target()));
    entries.push(OperationEntry::runs(rail_words::DELETE, "branch.delete", target()));
    entries
}

/// **Set upstream…** for a branch that follows nothing, or **Change upstream…** and **Stop tracking `<upstream>`** for
/// one that follows a remote branch, gone or not; none while the repository has no remote branch to follow.
fn upstream_entries(state: &model::state::RepositoryState, branch: &Branch) -> Vec<OperationEntry> {
    let target = || Target::Branch(branch.name.clone());
    let Some(upstream) = &branch.upstream else {
        if state.remote_branches.is_empty() {
            return Vec::new();
        }
        return vec![OperationEntry::runs(
            rail_words::SET_UPSTREAM,
            "branch.set-upstream",
            target(),
        )];
    };
    let short = upstream
        .reference
        .strip_prefix("refs/remotes/")
        .unwrap_or(&upstream.reference);
    let mut entries = Vec::new();
    if !state.remote_branches.is_empty() {
        entries.push(OperationEntry::runs(
            rail_words::CHANGE_UPSTREAM,
            "branch.set-upstream",
            target(),
        ));
    }
    entries.push(OperationEntry::runs(
        rail_words::stop_tracking(short),
        "branch.unset-upstream",
        target(),
    ));
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
    entries.extend(menus.github.link_of_remote_branch(remote).map(OperationEntry::opens));
    entries.extend(check_out_pull_request(menus, remote, target()));
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
    entries.extend(delete_remote_entries(&state.remotes, target()));
    entries
}

/// **Delete from `<remote>`** and **Delete here and from `<remote>`** for the remote of `remote.default`, or, with
/// several remotes, **Delete from a remote…** and **Delete here and from a remote…**, which ask which; none without a
/// remote (`tag.delete-remote`).
fn delete_remote_entries(remotes: &[String], target: Target) -> Vec<OperationEntry> {
    let Some(remote) = model::remote::default(remotes) else {
        return Vec::new();
    };
    let (from, here_and_from) = if remotes.len() == 1 {
        (words::delete_from(&remote), words::delete_here_and_from(&remote))
    } else {
        (
            String::from(words::DELETE_FROM_A_REMOTE),
            String::from(words::DELETE_HERE_AND_FROM_A_REMOTE),
        )
    };
    let here = Answers::new().with(DELETE_HERE, model::contract::Answer::Flag(true));
    vec![
        OperationEntry::runs(from, "tag.delete-remote", target.clone()),
        OperationEntry::runs_with(here_and_from, "tag.delete-remote", target, here),
    ]
}

/// The answer of `tag.delete-remote` that deletes the tag here too.
const DELETE_HERE: model::contract::AnswerKey = model::contract::AnswerKey("here");

/// The operations of a commit of the timeline: a branch or a tag made there; the commit checked out on a detached
/// `HEAD`, merged or cherry-picked unless it is the one checked out; reverted; the branch checked out reset to it; and
/// its message edited while no operation and no bisect is under way.
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
    if state.operation.is_none() && state.bisect.is_none() {
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

/// **Check out pull request #12** (`hosting.checkout-pull-request`) on a remote branch whose label shows a pull
/// request, with its number given, as the feature's answer `number`.
fn check_out_pull_request(menus: ReferenceMenus<'_>, remote: &RemoteBranch, target: Target) -> Option<OperationEntry> {
    let pull_request = menus.github.of_remote_branch(menus.repository, remote)?;
    let service = menus.github.service();
    let _kept_by_the_service = service.pull_request_reference(pull_request.number)?;
    let number = Answer::Text(pull_request.number.to_string());
    Some(OperationEntry::runs_with(
        crate::text::github::check_out_this_pull_request(service, pull_request.number),
        "hosting.checkout-pull-request",
        target,
        Answers::new().with(AnswerKey("number"), number),
    ))
}

/// The items of the entries, each carrying the value of its place among them and the icon of its operation, greyed out
/// while its operation comes later.
pub(crate) fn menu_items<Value>(
    entries: &[OperationEntry],
    value: impl Fn(usize) -> Value,
) -> Vec<MenuEntry<'_, Value>> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let feature = match (&entry.run, &entry.url) {
                (Some((FeatureId(feature), _)), _) => feature,
                (None, Some(_)) => "github.open-pull-request",
                (None, None) => "",
            };
            let item = icons::operation_item(entry.label.as_str(), feature, value(index));
            MenuEntry::Item(if entry.run.is_some() || entry.url.is_some() {
                item
            } else {
                item.disabled()
            })
        })
        .collect()
}

/// What the entry at a place asks: its operation, run on its reference.
pub(crate) fn chosen(entries: &[OperationEntry], index: usize) -> Option<Intent> {
    let entry = entries.get(index)?;
    if let Some(url) = &entry.url {
        return Some(Intent::OpenUrl(url.clone()));
    }
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
