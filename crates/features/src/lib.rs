//! Every operation of Gitardo Desktop, as a function that asks questions and returns a plan.
//!
//! Each operation is pure (rule R2.2): it is given the [`Context`] and the [`Answers`] so far, and says what it needs
//! next or what it has decided to do. Only the engine runs what it decides.

#![forbid(unsafe_code)]

pub mod branch;
pub mod changes;
pub mod commit;
pub mod commit_page;
pub mod identity;
pub mod search;
pub mod sync;
pub mod tag;
pub mod undo;

#[cfg(test)]
mod testing;

use model::contract::{FeatureId, Operation};

/// The operations of the catalog that are built, by identifier.
pub const OPERATIONS: [(FeatureId, Operation); 27] = [
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
    (FeatureId("identity.switch"), identity::switch),
    (FeatureId("identity.use-global"), identity::use_global),
    (FeatureId("search.go-to-commit"), search::go_to_commit),
    (FeatureId("sync.fetch"), sync::fetch),
    (FeatureId("tag.checkout"), tag::checkout),
    (FeatureId("tag.create"), tag::create),
    (FeatureId("tag.delete"), tag::delete),
    (FeatureId("tag.push"), tag::push),
    (FeatureId("undo.last"), undo::last),
];

/// The operation of an identifier, when it is built.
#[must_use]
pub fn operation(identifier: &str) -> Option<(FeatureId, Operation)> {
    OPERATIONS.iter().find(|(feature, _)| feature.0 == identifier).copied()
}
