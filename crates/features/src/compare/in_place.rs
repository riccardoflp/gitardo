//! `compare.with-current` and `compare.selected-before`: two commits compared in the inspector, in place of the commit
//! selected.

use model::compare::{InPlaceSides, NamedCommit, Side};
use model::contract::{Answers, Context, FollowUp, Outcome, Refusal, Target};

use super::{no_step, side_of_target};

/// Compares what a menu was opened on, first, with what is checked out, second, in the inspector: a commit by its
/// short hash, a branch, a remote branch or a tag by its name, and what is checked out by its branch, or `HEAD`.
///
/// Feature: compare.with-current
#[must_use]
pub fn with_current(context: &Context, _answers: &Answers) -> Outcome {
    let repository = &context.repository;
    let Some(head) = &repository.head.commit else {
        return Outcome::Refuse(Refusal::saying("There is no commit checked out to compare with."));
    };
    let from = side_of_target(context).and_then(|side| match side.side {
        Side::Commit(commit) => Some(NamedCommit {
            name: side.name,
            commit,
        }),
        Side::WorkingTree => None,
    });
    let Some(from) = from else {
        return Outcome::Refuse(Refusal::saying("Pick a commit, a branch or a tag to compare."));
    };
    if from.commit == *head {
        return Outcome::Refuse(Refusal::saying("This is the commit checked out."));
    }
    let to = NamedCommit {
        name: repository.checked_out_branch().unwrap_or("HEAD").to_owned(),
        commit: head.clone(),
    };
    in_place(InPlaceSides { from, to })
}

/// Compares the commit selected before with the one the menu was opened on, in the inspector, each named by its short
/// hash: the target holds the two, the one selected before first.
///
/// Feature: compare.selected-before
#[must_use]
pub fn selected_before(context: &Context, _answers: &Answers) -> Outcome {
    let Target::Commits(commits) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Select a commit, then open the menu of another."));
    };
    let [before, clicked] = commits.as_slice() else {
        return Outcome::Refuse(Refusal::saying("Select a commit, then open the menu of another."));
    };
    if before == clicked {
        return Outcome::Refuse(Refusal::saying("Choose another commit than the one selected."));
    }
    let named = |commit: &model::CommitId| NamedCommit {
        name: commit.short().to_owned(),
        commit: commit.clone(),
    };
    in_place(InPlaceSides {
        from: named(before),
        to: named(clicked),
    })
}

/// The plan that shows two commits compared in the inspector.
fn in_place(sides: InPlaceSides) -> Outcome {
    let title = format!("Compare {} with {}", sides.from.name, sides.to.name);
    Outcome::Run(no_step(title, vec![FollowUp::CompareInPlace(Box::new(sides))]))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::{InPlaceSides, NamedCommit};
    use model::contract::{Answers, FollowUp, Outcome, Target};

    use super::{selected_before, with_current};
    use crate::testing::{branch, commit, context, plan, state};

    /// The two sides a plan compares in the inspector.
    fn sides(outcome: Outcome) -> InPlaceSides {
        let plan = plan(outcome);
        let Some(FollowUp::CompareInPlace(sides)) = plan.after.first() else {
            panic!("compares in place: {plan:?}")
        };
        sides.as_ref().clone()
    }

    #[test]
    fn compare_with_current__on_feature_with_main_checked_out_compares_feature_with_main() {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        let context = context(state, Target::Branch(String::from("feature")));
        let compared = sides(with_current(&context, &Answers::new()));
        assert_eq!(
            (compared.from.name.as_str(), compared.to.name.as_str()),
            ("feature", "main")
        );
        assert_eq!(compared.from.commit, commit('b'));
    }

    #[test]
    fn compare_with_current__refused_on_the_commit_checked_out() {
        let context = context(state(), Target::Branch(String::from("main")));
        assert!(matches!(with_current(&context, &Answers::new()), Outcome::Refuse(_)));
    }

    #[test]
    fn compare_selected_before__compares_the_commit_selected_before_with_the_one_clicked() {
        let context = context(state(), Target::Commits(vec![commit('1'), commit('2')]));
        let compared = sides(selected_before(&context, &Answers::new()));
        let named = |digit| NamedCommit {
            name: commit(digit).short().to_owned(),
            commit: commit(digit),
        };
        assert_eq!(
            compared,
            InPlaceSides {
                from: named('1'),
                to: named('2')
            }
        );
    }

    #[test]
    fn compare_selected_before__refused_on_the_commit_selected_itself() {
        let context = context(state(), Target::Commits(vec![commit('1'), commit('1')]));
        assert!(matches!(selected_before(&context, &Answers::new()), Outcome::Refuse(_)));
    }
}
