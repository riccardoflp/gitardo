//! `stash.pop`: brings the changes of a stash back into the files, and drops the stash once they are in without
//! conflicts.

use model::contract::{Answers, Context, Outcome};

use super::bring_back;

/// Pops the stash it was invoked on with `git stash pop stash@{N}`, always asking first with what it would stop on
/// (`stash.preview`); git keeps the stash when it stops with conflicts.
///
/// Feature: stash.pop
#[must_use]
pub fn pop(context: &Context, answers: &Answers) -> Outcome {
    bring_back(
        context,
        answers,
        (
            "pop",
            "Its changes go into the files, and it is dropped once they are in without conflicts: with conflicts it \
             stays.",
        ),
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::pop;
    use crate::stash::READ;
    use crate::testing::{context, lines, plan, stash, state};
    use model::contract::{Answer, Answers, Danger, Target};
    use model::stash::StashReading;

    #[test]
    fn stash_pop__asks_first_whatever_command_preview_says() {
        let mut state = state();
        state.stashes.push(stash(0, "On main: one"));
        state.stashes.push(stash(1, "On main: two"));
        let answers = Answers::new().with(READ, Answer::Stash(StashReading::default()));
        let plan = plan(pop(&context(state, Target::Stash(1)), &answers));
        assert_eq!(plan.title.as_str(), "Pop stash@{1}");
        assert_eq!(lines(&plan), ["git stash pop stash@{1}"]);
        assert!(matches!(plan.danger, Danger::Confirm(_)));
    }
}
