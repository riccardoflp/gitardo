//! `stash.push`: sets every change aside, staged or not, untracked files included.

use model::contract::{AnswerKey, Answers, Context, Outcome, Refusal};

use super::{message_question, push_plan, refuse_before_first_commit};

/// The optional message typed for a stash.
pub const MESSAGE: AnswerKey = AnswerKey("message");

/// Asks an optional message, then stashes every change with `git stash push --include-untracked`; refuses before the
/// first commit and when there is nothing to stash.
///
/// Feature: stash.push
#[must_use]
pub fn push(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = refuse_before_first_commit(context) {
        return refusal;
    }
    if context.repository.working_tree.is_clean() {
        return Outcome::Refuse(Refusal::saying("There are no changes to stash."));
    }
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Ask(message_question(
            String::from("Stash changes"),
            "Optional message. Untracked files are stashed too.",
        ));
    };
    Outcome::Run(push_plan(
        context,
        "Stash all changes",
        (&["--include-untracked"], message),
        &[],
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MESSAGE, push};
    use crate::testing::{context, lines, modified, plan, question, state};
    use model::contract::{Answer, Answers, Outcome, Target};

    fn changed() -> model::contract::Context {
        let mut state = state();
        state.working_tree.unstaged.push(modified("a.txt"));
        context(state, Target::Head)
    }

    #[test]
    fn stash_push__a_message_typed_is_the_message_of_the_stash() {
        let asked = question(push(&changed(), &Answers::new()));
        assert_eq!(asked.title.as_str(), "Stash changes");
        let answers = Answers::new().with(MESSAGE, Answer::Text(String::from("  before the merge ")));
        assert_eq!(
            lines(&plan(push(&changed(), &answers))),
            ["git stash push --include-untracked -m \"before the merge\""]
        );
        let none = Answers::new().with(MESSAGE, Answer::Text(String::new()));
        assert_eq!(
            lines(&plan(push(&changed(), &none))),
            ["git stash push --include-untracked"]
        );
    }

    #[test]
    fn stash_push__before_the_first_commit_it_is_not_offered() {
        let mut state = state();
        state.head.commit = None;
        state.working_tree.untracked.push(String::from("a.txt"));
        assert!(matches!(
            push(&context(state, Target::Head), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }
}
