//! `rebase.branch-onto`: rebases a local branch that is not checked out onto the commit it is dropped on; git checks
//! it out first.

use model::branch::other_worktree;
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Target, Text,
    Undo,
};

use super::refuse;

/// The local branch to rebase, by its short name: what was dropped on the commit.
pub const BRANCH: AnswerKey = AnswerKey("branch");

/// Rebases the local branch answered under [`BRANCH`] onto the commit it was invoked on: `git rebase <full hash>
/// <branch>`, which checks the branch out first. A branch checked out here or in another worktree is refused.
///
/// Feature: rebase.branch-onto
#[must_use]
pub fn branch_onto(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(onto) = &context.target else {
        return refuse("Choose the commit to rebase the branch onto.");
    };
    let state = &context.repository;
    let Some(name) = answers.choice(BRANCH) else {
        let choices: Vec<Choice> = state
            .branches
            .iter()
            .filter(|branch| !branch.is_checked_out && branch.commit != *onto)
            .filter(|branch| other_worktree(state, &branch.name).is_none())
            .map(|branch| Choice {
                identifier: branch.name.clone(),
                label: Text::from(branch.name.as_str()),
                description: None,
            })
            .collect();
        if choices.is_empty() {
            return refuse(&format!("No other local branch can be rebased onto {}.", onto.short()));
        }
        return Outcome::Ask(Question {
            key: BRANCH,
            kind: QuestionKind::Pick { choices },
            title: Text::from(format!("Rebase a branch onto {}", onto.short())),
            detail: None,
            default: None,
        });
    };
    let Some(branch) = state.branches.iter().find(|branch| branch.name == name) else {
        return refuse(&format!("There is no branch {name}."));
    };
    if branch.is_checked_out {
        return refuse(&format!(
            "{name} is checked out: rebase it onto {} instead.",
            onto.short()
        ));
    }
    if other_worktree(state, name).is_some() {
        return refuse(&format!("{name} is checked out in another worktree: rebase it there."));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Rebase {name} onto {}", onto.short())),
        steps: vec![Step::git(state.folder.clone(), ["rebase", onto.as_str(), name])],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// The answers of a branch dropped on a commit: the branch to rebase.
#[must_use]
pub fn dropped(name: &str) -> Answers {
    Answers::new().with(BRANCH, Answer::Choice(name.to_owned()))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{branch_onto, dropped};
    use crate::testing::{branch, commit, context, lines, plan, question, state};
    use model::contract::{Answers, Outcome, QuestionKind, Target};
    use model::state::Worktree;

    #[test]
    fn rebase_branch_onto__runs_rebase_with_the_full_hash_and_the_branch() {
        let mut state = state();
        state.branches.push(branch("feature", 'c', false));
        let context = context(state, Target::Commit(commit('b')));
        let plan = plan(branch_onto(&context, &dropped("feature")));
        assert_eq!(lines(&plan), [format!("git rebase {} feature", commit('b'))]);
        assert_eq!(plan.title.as_str(), "Rebase feature onto bbbbbbb");
        let QuestionKind::Pick { choices } = question(branch_onto(&context, &Answers::new())).kind else {
            panic!("a pick")
        };
        assert_eq!(choices.len(), 1);
    }

    #[test]
    fn rebase_branch_onto__refuses_a_branch_another_worktree_has() {
        let mut state = state();
        state.branches.push(branch("feature", 'c', false));
        state.worktrees.push(Worktree {
            folder: std::path::PathBuf::from("elsewhere"),
            commit: Some(commit('c')),
            branch: Some(String::from("refs/heads/feature")),
            is_main: false,
            is_current: false,
            is_missing: false,
            uncommitted: Some(0),
        });
        let context = context(state, Target::Commit(commit('b')));
        assert!(matches!(branch_onto(&context, &dropped("feature")), Outcome::Refuse(_)));
        assert!(matches!(branch_onto(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
