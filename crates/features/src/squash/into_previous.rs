//! `squash.into-previous`: squashes a commit into the commit before it.

use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Target};

use crate::rebase::refuse;

/// The parents of the commit, read before the line of its parent.
pub const PARENTS: AnswerKey = AnswerKey("parents");

/// Squashes the commit it was invoked on into its parent, as `squash.commits` does with the two. The first commit has
/// nothing before it to be squashed into; a merge is not on a line of first parents.
///
/// Feature: squash.into-previous
#[must_use]
pub fn into_previous(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(commit) = &context.target else {
        return refuse("Choose a commit first.");
    };
    let Some(read) = answers.line(PARENTS) else {
        return Outcome::Read(PARENTS, Query::Line { commit: commit.clone() });
    };
    let parent = match read.parents.as_slice() {
        [] => {
            return refuse(&format!(
                "{} is the first commit: there is nothing before it to squash it into.",
                commit.short()
            ));
        }
        [parent] => parent.clone(),
        _ => {
            return refuse(&format!(
                "{} is a merge: it cannot be squashed into one parent.",
                commit.short()
            ));
        }
    };
    super::squash(context, answers, &[parent, commit.clone()])
}
