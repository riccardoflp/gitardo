//! Cherry-picking a commit onto what is checked out, and getting past a commit of a cherry-pick or a revert that
//! changes nothing (`docs/features/07-history-rewriting.md`).

mod commit;
mod skip_empty;

pub use commit::commit;
pub use skip_empty::skip_empty;

use model::contract::{FeatureId, FollowUp};

/// What a cherry-pick or a revert does when one of its steps fails: it gets past a commit that changes nothing, when
/// that is why it stopped.
pub(crate) fn skip_when_empty() -> Vec<FollowUp> {
    vec![FollowUp::Recover(FeatureId("cherry-pick.skip-empty"))]
}
