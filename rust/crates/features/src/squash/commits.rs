//! `squash.commits`: makes one commit of several that follow one another before `HEAD`.

use model::contract::{Answers, Context, Outcome, Target};

use crate::rebase::refuse;

/// Squashes the commits selected into one, as an interactive rebase that rewords the oldest with the messages of them
/// all under the subject answered and makes the others fixups of it; the commits after them are made again on top,
/// and what is not committed is set aside and put back.
///
/// Feature: squash.commits
#[must_use]
pub fn commits(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commits(selected) = &context.target else {
        return refuse("Select two commits or more to squash them into one.");
    };
    let mut unique = Vec::with_capacity(selected.len());
    for commit in selected {
        if !unique.contains(commit) {
            unique.push(commit.clone());
        }
    }
    super::squash(context, answers, &unique)
}
