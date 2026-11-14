//! The identity commits are made as in a repository: switching it, and going back to the global one
//! (`docs/features/05-changes-and-commit.md`).

mod switch;
mod use_global;

pub use switch::switch;
pub use use_global::use_global;

use model::contract::{AnswerKey, Answers, IdentityKeys, Outcome, Query};

/// Where the identity read comes back (`identity.read`).
const IDENTITY: AnswerKey = AnswerKey("identity");

/// The identity of the repository as read, or the outcome that reads it first.
fn read_identity(answers: &Answers) -> Result<&IdentityKeys, Box<Outcome>> {
    answers
        .identity(IDENTITY)
        .ok_or_else(|| Box::new(Outcome::Read(IDENTITY, Query::Identity)))
}
