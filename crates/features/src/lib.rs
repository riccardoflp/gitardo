//! Every operation of Gitardo Desktop, as a function that asks questions and returns a plan.
//!
//! Each operation is pure (rule R2.2): it is given the [`Context`] and the [`Answers`] so far, and says what it needs
//! next or what it has decided to do. Only the engine runs what it decides.

#![forbid(unsafe_code)]

pub mod branch;
pub mod changes;
pub mod cherry_pick;
pub mod commit;
pub mod commit_page;
pub mod commits;
pub mod diff;
mod history;
pub mod identity;
pub mod merge;
pub mod message;
pub mod remote;
pub mod reset;
pub mod revert;
pub mod search;
pub mod selection;
pub mod stash;
pub mod sync;
pub mod tag;
pub mod undo;

#[cfg(test)]
mod testing;

use model::contract::{FeatureId, Operation};

/// The operations of the catalog that are built, by identifier.
pub const OPERATIONS: [(FeatureId, Operation); 65] = [
    (FeatureId("branch.checkout"), branch::checkout),
    (FeatureId("branch.checkout-detached"), branch::checkout_detached),
    (FeatureId("branch.checkout-remote"), branch::checkout_remote),
    (
        FeatureId("branch.checkout-remote-and-rebase"),
        branch::checkout_remote_and_rebase,
    ),
    (FeatureId("branch.clean-up"), branch::clean_up),
    (FeatureId("branch.create"), branch::create),
    (FeatureId("branch.delete"), branch::delete),
    (FeatureId("branch.fast-forward"), branch::fast_forward),
    (FeatureId("branch.rename"), branch::rename),
    (FeatureId("changes.discard"), changes::discard),
    (FeatureId("changes.discard-all"), changes::discard_all),
    (FeatureId("changes.stage"), changes::stage),
    (FeatureId("changes.stage-all"), changes::stage_all),
    (FeatureId("changes.unstage"), changes::unstage),
    (FeatureId("changes.unstage-all"), changes::unstage_all),
    (FeatureId("commit-page.open"), commit_page::open),
    (FeatureId("commit.amend"), commit::amend),
    (FeatureId("commit.create"), commit::create),
    (FeatureId("commits.copy-hash"), commits::copy_hash),
    (FeatureId("commits.copy-message"), commits::copy_message),
    (FeatureId("identity.switch"), identity::switch),
    (FeatureId("identity.use-global"), identity::use_global),
    (FeatureId("search.go-to-commit"), search::go_to_commit),
    (FeatureId("sync.fetch"), sync::fetch),
    (FeatureId("tag.checkout"), tag::checkout),
    (FeatureId("tag.create"), tag::create),
    (FeatureId("tag.delete"), tag::delete),
    (FeatureId("tag.push"), tag::push),
    (FeatureId("undo.last"), undo::last),
    // Remotes, sync and stashes.
    (FeatureId("sync.pull"), sync::pull),
    (FeatureId("sync.pull-with"), sync::pull_with),
    (FeatureId("sync.push"), sync::push),
    (FeatureId("sync.push-force"), sync::push_force),
    (FeatureId("sync.push-force-with-lease"), sync::push_force_with_lease),
    (FeatureId("sync.sync"), sync::sync),
    (FeatureId("remote.add"), remote::add),
    (FeatureId("remote.delete-branch"), remote::delete_branch),
    (FeatureId("remote.fetch"), remote::fetch),
    (FeatureId("remote.manage"), remote::manage),
    (FeatureId("remote.remove"), remote::remove),
    (FeatureId("remote.rename"), remote::rename),
    (FeatureId("remote.set-url"), remote::set_url),
    (FeatureId("stash.apply"), stash::apply),
    (FeatureId("stash.branch"), stash::branch),
    (FeatureId("stash.drop"), stash::drop),
    (FeatureId("stash.pop"), stash::pop),
    (FeatureId("stash.push"), stash::push),
    (FeatureId("stash.push-file"), stash::push_file),
    (FeatureId("stash.push-staged"), stash::push_staged),
    (FeatureId("stash.undo-drop"), stash::undo_drop),
    // The Diff page.
    (FeatureId("diff.discard-file"), diff::discard_file),
    (FeatureId("diff.revert-change"), diff::revert_change),
    (FeatureId("diff.stage-change"), diff::stage_change),
    (FeatureId("diff.stage-file"), diff::stage_file),
    (FeatureId("diff.unstage-change"), diff::unstage_change),
    (FeatureId("diff.unstage-file"), diff::unstage_file),
    // Commits brought, taken back and reworded, and several selected at once.
    (FeatureId("cherry-pick.commit"), cherry_pick::commit),
    (FeatureId("cherry-pick.skip-empty"), cherry_pick::skip_empty),
    (FeatureId("merge.choose-way"), merge::choose_way),
    (FeatureId("merge.into-current"), merge::into_current),
    (FeatureId("merge.undo-squash"), merge::undo_squash),
    (FeatureId("message.edit"), message::edit),
    (FeatureId("reset.to-commit"), reset::to_commit),
    (FeatureId("revert.commit"), revert::commit),
    (FeatureId("selection.cherry-pick"), selection::cherry_pick),
];

/// The operation of an identifier, when it is built.
#[must_use]
pub fn operation(identifier: &str) -> Option<(FeatureId, Operation)> {
    OPERATIONS.iter().find(|(feature, _)| feature.0 == identifier).copied()
}
