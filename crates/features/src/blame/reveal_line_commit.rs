//! `blame.reveal-line-commit`: shows the commit that last changed the line under the cursor in the timeline.

use model::contract::{Answers, Context, FollowUp, Outcome};

use super::{line_commit, no_step};

/// Selects in the timeline the commit that last changed the line under the cursor of a File page: the window loads
/// the history up to it, and takes away the narrowing that leaves it out, as going to a commit does.
///
/// Feature: blame.reveal-line-commit
#[must_use]
pub fn reveal_line_commit(context: &Context, answers: &Answers) -> Outcome {
    match line_commit(context, answers) {
        Ok(commit) => no_step(
            format!("Show commit {} in the timeline", commit.id.short()),
            FollowUp::SelectCommit(commit.id.clone()),
        ),
        Err(outcome) => *outcome,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answers, FollowUp, Outcome};

    use super::reveal_line_commit;
    use model::blame::ShownBlame;

    use crate::blame::testing::{blamed, on_line};
    use crate::testing::{commit, context, plan, state};

    #[test]
    fn blame_reveal_line_commit__selects_the_commit_of_the_line() {
        let shown = on_line(1, ShownBlame::Line(Some(blamed('c', None))));
        let plan = plan(reveal_line_commit(&shown, &Answers::new()));
        assert_eq!(plan.after, [FollowUp::SelectCommit(commit('c'))]);
        let nothing = context(state(), model::contract::Target::Head);
        let Outcome::Refuse(refusal) = reveal_line_commit(&nothing, &Answers::new()) else {
            panic!("refuses with no File page");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Put the cursor on a line of a file of a repository."
        );
    }
}
