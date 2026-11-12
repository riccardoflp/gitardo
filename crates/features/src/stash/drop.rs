//! `stash.drop`: deletes a stash and the changes it holds.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Step, Text, Undo, UndoRecord};

use super::target_stash;

/// Drops the stash it was invoked on with `git stash drop stash@{N}`, warning first; the journal keeps its message and
/// its commit, so that Undo can store it again (`stash.undo-drop`).
///
/// Feature: stash.drop
#[must_use]
pub fn drop(context: &Context, _answers: &Answers) -> Outcome {
    let stash = match target_stash(context) {
        Ok(stash) => stash,
        Err(refusal) => return *refusal,
    };
    let name = format!("stash@{{{}}}", stash.index);
    Outcome::Run(Plan {
        title: Text::from(format!("Drop {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["stash", "drop", &name])],
        danger: Danger::Destructive(Text::from("The stashed changes will be deleted.")),
        undo: Undo::Journal(UndoRecord::StashDropped {
            message: stash.message.clone(),
            commit: stash.commit.clone(),
        }),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::drop;
    use crate::testing::{commit, context, lines, plan, stash, state};
    use model::contract::{Answers, Danger, Target, Undo, UndoRecord};

    #[test]
    fn stash_drop__asks_with_a_warning_and_keeps_what_undo_needs() {
        let mut state = state();
        state.stashes.push(stash(0, "On main: try this"));
        let plan = plan(drop(&context(state, Target::Stash(0)), &Answers::new()));
        assert_eq!(lines(&plan), ["git stash drop stash@{0}"]);
        assert!(matches!(plan.danger, Danger::Destructive(_)));
        assert_eq!(
            plan.undo,
            Undo::Journal(UndoRecord::StashDropped {
                message: String::from("On main: try this"),
                commit: commit('5'),
            })
        );
    }
}
