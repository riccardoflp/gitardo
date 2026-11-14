//! `compare.with`, which compares what a menu was opened on with a side asked for, and `compare.change-side`, which
//! puts another side in place of one side of a comparison page.

use model::compare::{ComparisonPart, ComparisonTarget};
use model::contract::{Answers, Context, Outcome, Refusal, Target};

use super::{Picked, SideKeys, SideQuestion, comparison, opens, side, side_of_target};

/// Compares a commit, a branch, a remote branch or a tag with the side asked for, the one the menu was opened on
/// first: a commit is named by its short hash and stays that commit, a branch or a tag follows its name.
///
/// Feature: compare.with
#[must_use]
pub fn with(context: &Context, answers: &Answers) -> Outcome {
    let Some(first) = side_of_target(context) else {
        return Outcome::Refuse(Refusal::saying("Pick a commit, a branch or a tag to compare."));
    };
    let question = SideQuestion {
        title: format!("Compare {} with", first.name),
        placeholder: "The branch, tag or commit to compare it with",
        chosen: Some(&first),
        may_offer_working_tree: true,
        recent: None,
    };
    match side(context, answers, SideKeys::SECOND, &question) {
        Ok(Picked::Side(second)) => opens(comparison(first, second, None)),
        Ok(Picked::Recent(_)) => Outcome::Refuse(Refusal::saying("Choose a branch, tag or commit.")),
        Err(outcome) => *outcome,
    }
}

/// Puts the side asked for in place of one side of a comparison, the other left out of the choices and the working
/// tree offered unless the other side is it; the page then compares the new pair in the mode `compare.initial-mode`
/// gives, with the path it had.
///
/// Feature: compare.change-side
#[must_use]
pub fn change_side(context: &Context, answers: &Answers) -> Outcome {
    let Target::Comparison(target) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose a side of a comparison."));
    };
    let is_first = match target.part {
        ComparisonPart::FirstSide => true,
        ComparisonPart::SecondSide => false,
        ComparisonPart::Whole | ComparisonPart::File(_) => {
            return Outcome::Refuse(Refusal::saying("Choose a side of a comparison."));
        }
    };
    let (changed, other) = if is_first {
        (&target.first, &target.second)
    } else {
        (&target.second, &target.first)
    };
    let title = if is_first {
        format!("Compare with {}", target.second.name)
    } else {
        format!("Compare {} with", target.first.name)
    };
    let placeholder = format!("The branch, tag or commit to put in place of {}", changed.name);
    let question = SideQuestion {
        title,
        placeholder: &placeholder,
        chosen: Some(other),
        may_offer_working_tree: !other.is_working_tree(),
        recent: None,
    };
    let keys = if is_first { SideKeys::FIRST } else { SideKeys::SECOND };
    let new = match side(context, answers, keys, &question) {
        Ok(Picked::Side(new)) => new,
        Ok(Picked::Recent(_)) => return Outcome::Refuse(Refusal::saying("Choose a branch, tag or commit.")),
        Err(outcome) => return *outcome,
    };
    let ComparisonTarget {
        first, second, path, ..
    } = target.as_ref().clone();
    let (first, second) = if is_first { (new, second) } else { (first, new) };
    opens(comparison(first, second, path))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::ComparisonPart;
    use model::contract::{Answers, FollowUp, Outcome, QuestionKind, Target};

    use super::{change_side, with};
    use crate::compare::SECOND;
    use crate::compare::testing::{branch as side, compared, picked};
    use crate::testing::{branch, commit, context, plan, question, state};

    /// A repository with `main` checked out at `a`, `feature` at `b` and `release` at `c`.
    fn branches() -> model::state::RepositoryState {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        state.branches.push(branch("release", 'c', false));
        state
    }

    /// The names of the sides a plan opens, and whether the first is followed.
    fn opened(outcome: Outcome) -> (String, String, bool, Option<String>) {
        let plan = plan(outcome);
        let Some(FollowUp::OpenComparison(target)) = plan.after.first() else {
            panic!("opens a comparison: {plan:?}")
        };
        (
            target.first.name.clone(),
            target.second.name.clone(),
            target.first.is_followed,
            target.path.clone(),
        )
    }

    /// The identifiers of the choices of a question.
    fn choices(outcome: Outcome) -> Vec<String> {
        match question(outcome).kind {
            QuestionKind::Pick { choices } => choices.into_iter().map(|choice| choice.identifier).collect(),
            other => panic!("a pick, not {other:?}"),
        }
    }

    #[test]
    fn compare_with__on_feature_choosing_main_opens_feature_with_main() {
        let context = context(branches(), Target::Branch(String::from("feature")));
        let (first, second, is_followed, _) = opened(with(&context, &picked(&[(SECOND, "ref:main")])));
        assert_eq!(
            (first.as_str(), second.as_str(), is_followed),
            ("feature", "main", true)
        );
    }

    #[test]
    fn compare_with__a_commit_is_named_by_its_short_hash_and_fixed() {
        let context = context(branches(), Target::Commit(commit('b')));
        let (first, _, is_followed, _) = opened(with(&context, &picked(&[(SECOND, "ref:main")])));
        assert_eq!((first.as_str(), is_followed), (commit('b').short(), false));
    }

    #[test]
    fn compare_with__the_side_itself_is_not_among_the_choices() {
        let context = context(branches(), Target::Branch(String::from("feature")));
        let offered = choices(with(&context, &Answers::new()));
        assert!(!offered.contains(&String::from("ref:feature")), "{offered:?}");
        assert!(offered.contains(&String::from("ref:main")), "{offered:?}");
    }

    #[test]
    fn compare_change_side__release_in_place_of_main_compares_release_with_the_second_side() {
        let target = compared(side("main", 'a'), side("feature", 'b'), ComparisonPart::FirstSide);
        let context = context(branches(), Target::Comparison(Box::new(target)));
        let asked = question(change_side(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Compare with feature");
        let answers = picked(&[(crate::compare::FIRST, "ref:release")]);
        let (first, second, _, _) = opened(change_side(&context, &answers));
        assert_eq!((first.as_str(), second.as_str()), ("release", "feature"));
    }

    #[test]
    fn compare_change_side__the_choices_are_those_of_the_repository_of_the_page_the_other_side_left_out() {
        let target = compared(side("main", 'a'), side("feature", 'b'), ComparisonPart::SecondSide);
        let context = context(branches(), Target::Comparison(Box::new(target)));
        let offered = choices(change_side(&context, &Answers::new()));
        assert!(!offered.contains(&String::from("ref:main")), "{offered:?}");
        assert!(offered.contains(&String::from("ref:release")), "{offered:?}");
    }

    #[test]
    fn compare_change_side__nothing_picked_leaves_the_page_as_it_was() {
        let target = compared(side("main", 'a'), side("feature", 'b'), ComparisonPart::SecondSide);
        let context = context(branches(), Target::Comparison(Box::new(target)));
        assert!(matches!(change_side(&context, &Answers::new()), Outcome::Ask(_)));
    }
}
