//! Every operation of Gitardo Desktop, as a function that asks questions and returns a plan.
//!
//! Each operation is pure (rule R2.2): it is given the [`Context`] and the [`Answers`] so far, and says what it needs
//! next or what it has decided to do. Only the engine runs what it decides.

#![forbid(unsafe_code)]

pub mod branch;
pub mod commit;
pub mod search;
pub mod sync;
pub mod undo;

#[cfg(test)]
mod testing;

use model::contract::{FeatureId, Operation};

/// The operations of the catalog that are built, by identifier.
pub const OPERATIONS: [(FeatureId, Operation); 5] = [
    (FeatureId("branch.create"), branch::create),
    (FeatureId("commit.create"), commit::create),
    (FeatureId("search.go-to-commit"), search::go_to_commit),
    (FeatureId("sync.fetch"), sync::fetch),
    (FeatureId("undo.last"), undo::last),
];

/// The operation of an identifier, when it is built.
#[must_use]
pub fn operation(identifier: &str) -> Option<(FeatureId, Operation)> {
    OPERATIONS.iter().find(|(feature, _)| feature.0 == identifier).copied()
}
