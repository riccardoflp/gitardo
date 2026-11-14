//! `stash.push-staged`: sets aside only what is staged; the rest stays where it is.

use model::contract::{Answers, Context, Outcome, Refusal};

use super::push::MESSAGE;
use super::{message_question, push_plan, refuse_before_first_commit};

/// Asks an optional message, then stashes what is staged with `git stash push --staged`; refuses before the first
/// commit and while nothing is staged. Git refuses `--staged` before 2.35, in its own words.
///
/// Feature: stash.push-staged
#[must_use]
pub fn push_staged(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = refuse_before_first_commit(context) {
        return refusal;
    }
    if context.repository.working_tree.staged.is_empty() {
        return Outcome::Refuse(Refusal::saying("Nothing is staged."));
    }
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Ask(message_question(
            String::from("Stash staged changes"),
            "Optional message. Only what is staged is stashed; the rest stays where it is.",
        ));
    };
    Outcome::Run(push_plan(
        context,
        "Stash the staged changes",
        (&["--staged"], message),
        &[],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::push_staged;
    use crate::stash::MESSAGE;
    use crate::testing::{context, lines, modified, plan, state};
    use model::contract::{Answer, Answers, Outcome, Target};

    #[test]
    fn stash_push_staged__stashes_only_what_is_staged() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        state.working_tree.unstaged.push(modified("b.txt"));
        let answers = Answers::new().with(MESSAGE, Answer::Text(String::new()));
        assert_eq!(
            lines(&plan(push_staged(&context(state, Target::Head), &answers))),
            ["git stash push --staged"]
        );
    }

    #[test]
    fn stash_push_staged__is_not_offered_while_nothing_is_staged() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("b.txt"));
        assert!(matches!(
            push_staged(&context(state, Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
