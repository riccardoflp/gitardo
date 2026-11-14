//! Committing: the commit of what is staged, with the message of the commit box, and amending the last commit
//! (`docs/features/05-changes-and-commit.md`).

mod amend;
mod create;

pub use amend::amend;
pub use create::{MESSAGE, create};
