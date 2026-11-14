//! Merging a commit, a branch or a remote branch into what is checked out, in one of the four ways, and taking a squash
//! merge back (`docs/features/07-history-rewriting.md`).

mod choose_way;
mod into_current;
mod undo_squash;

pub use choose_way::{WAY, choose_way};
pub use into_current::into_current;
pub use undo_squash::undo_squash;
pub(crate) use undo_squash::undo_squash_plan;

use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Refusal};

use crate::history::{Brought, brought};

/// The files the merge would leave in conflict, read before it runs.
pub const CONFLICTS: AnswerKey = AnswerKey("conflicts");

/// What is merged, once the operation may run: something checked out, and what is merged not the commit checked out
/// nor the branch checked out.
fn merged(context: &Context) -> Result<Brought, Box<Outcome>> {
    let repository = &context.repository;
    let Some(head) = &repository.head.commit else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(
            "There is nothing to merge into before the first commit.",
        ))));
    };
    let merged = brought(context)?;
    if &merged.commit == head {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(&format!(
            "{} is the commit checked out already.",
            merged.name
        )))));
    }
    Ok(merged)
}

/// The files merging would leave in conflict, or the query that reads them.
fn predicted<'a>(answers: &'a Answers, merged: &Brought) -> Result<Option<&'a [String]>, Box<Outcome>> {
    answers.conflicts(CONFLICTS).ok_or_else(|| {
        Box::new(Outcome::Read(
            CONFLICTS,
            Query::PredictedConflicts {
                ours: None,
                theirs: merged.argument.clone(),
                base: None,
            },
        ))
    })
}
