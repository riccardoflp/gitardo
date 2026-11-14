//! `stash.apply`: brings the changes of a stash back into the files, and keeps the stash.

use model::contract::{Answers, Context, Outcome};

use super::bring_back;

/// Applies the stash it was invoked on with `git stash apply stash@{N}`, always asking first with what it would stop
/// on (`stash.preview`).
///
/// Feature: stash.apply
#[must_use]
pub fn apply(context: &Context, answers: &Answers) -> Outcome {
    bring_back(
        context,
        answers,
        ("apply", "Its changes go into the files, and it stays in the list."),
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::apply;
    use crate::stash::READ;
    use crate::testing::{context, lines, plan, stash, state};
    use model::contract::{Answer, Answers, Danger, Outcome, Query, Target, Text};
    use model::stash::{StashFiles, StashReading};

    #[test]
    fn stash_apply__asks_first_with_the_note_and_what_it_would_stop_on() {
        let mut state = state();
        state.stashes.push(stash(0, "On main: try this"));
        let context = context(state, Target::Stash(0));
        assert!(matches!(
            apply(&context, &Answers::new()),
            Outcome::Read(READ, Query::Stash { index: 0 })
        ));
        let reading = StashReading {
            files: StashFiles {
                changed: vec![String::from("a.txt")],
                untracked: Vec::new(),
            },
            untracked_there: Vec::new(),
            conflicts: Some(Vec::new()),
        };
        let plan = plan(apply(&context, &Answers::new().with(READ, Answer::Stash(reading))));
        assert_eq!(plan.title.as_str(), "Apply stash@{0}");
        assert_eq!(lines(&plan), ["git stash apply stash@{0}"]);
        assert_eq!(
            plan.danger,
            Danger::Confirm(Text::from(
                "Its changes go into the files, and it stays in the list.\n\nNo conflicts expected."
            ))
        );
    }
}
