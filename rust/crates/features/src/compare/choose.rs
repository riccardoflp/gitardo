//! `compare.choose`: the two sides of a comparison asked one after the other, or a comparison opened before.

use model::compare::{CompareSide, RecentComparison, RecentSide, Side, SideInput, follow_sides, reopened};
use model::contract::{AnswerKey, Answers, Context, Outcome, Query, Refusal};

use super::{Picked, RECENT, SideKeys, SideQuestion, comparison, opens, side};

/// The keys of what the names of a comparison opened before resolve to now.
const REOPENED: [AnswerKey; 2] = [AnswerKey("recent-first"), AnswerKey("recent-second")];

/// Asks for the first side, among the comparisons opened before, the branches, the tags, what is checked out, the
/// working tree or a revision typed; then for the second, the first left out; and opens the comparison page of the
/// two. A comparison opened before opens again where its names are now, narrowed as it was, without a second question.
///
/// Feature: compare.choose
#[must_use]
pub fn choose(context: &Context, answers: &Answers) -> Outcome {
    let recent = answers.recent_comparisons(RECENT);
    let first = SideQuestion {
        title: String::from("Compare"),
        placeholder: "The branch, tag or commit to start from, or a comparison opened before",
        chosen: None,
        may_offer_working_tree: true,
        recent,
    };
    let first = match side(context, answers, SideKeys::FIRST, &first) {
        Ok(Picked::Side(first)) => first,
        Ok(Picked::Recent(place)) => {
            return match recent.and_then(|recent| recent.get(place)) {
                Some(opened) => reopen(answers, opened),
                None => Outcome::Refuse(Refusal::saying("This comparison is no longer in the list.")),
            };
        }
        Err(outcome) => return *outcome,
    };
    let second = SideQuestion {
        title: format!("Compare {} with", first.name),
        placeholder: "The branch, tag or commit to compare it with",
        chosen: Some(&first),
        may_offer_working_tree: !first.is_working_tree(),
        recent: None,
    };
    match side(context, answers, SideKeys::SECOND, &second) {
        Ok(Picked::Side(second)) => opens(comparison(first, second, None)),
        Ok(Picked::Recent(_)) => Outcome::Refuse(Refusal::saying("Choose a branch, tag or commit.")),
        Err(outcome) => *outcome,
    }
}

/// A comparison opened before, opened again: each name resolved where it is now.
fn reopen(answers: &Answers, opened: &RecentComparison) -> Outcome {
    let mut sides = Vec::new();
    for (side, key) in [&opened.first, &opened.second].into_iter().zip(REOPENED) {
        if side.commit.is_none() {
            sides.push(CompareSide::working_tree());
            continue;
        }
        let Some(resolved) = answers.commit(key) else {
            return Outcome::Read(
                key,
                Query::Revision {
                    revision: side.name.clone(),
                },
            );
        };
        sides.push(again(side, resolved));
    }
    let mut sides = sides.into_iter();
    match (sides.next(), sides.next()) {
        (Some(first), Some(second)) => opens(comparison(first, second, opened.path.clone())),
        _ => Outcome::Refuse(Refusal::saying("This comparison cannot be opened again.")),
    }
}

/// A side opened again: where `compare.recent` says it is now, followed unless it is named by its hash.
fn again(side: &RecentSide, resolved: Option<&model::CommitId>) -> CompareSide {
    let now = reopened(side, resolved);
    let Side::Commit(commit) = &now else {
        return CompareSide::working_tree();
    };
    let is_own_hash = !side.name.is_empty() && commit.as_str().starts_with(&side.name.to_lowercase());
    let remembered = side.commit.clone().unwrap_or_else(|| commit.clone());
    follow_sides(&SideInput {
        name: &side.name,
        chosen: &Side::Commit(remembered),
        is_followed: Some(!is_own_hash),
        resolved,
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::compare::{RecentComparison, RecentSide};
    use model::contract::{Answer, Answers, Context, FollowUp, Outcome, Query, QuestionKind, Target};

    use super::{REOPENED, choose};
    use crate::compare::testing::picked;
    use crate::compare::{FIRST, RECENT, SECOND, SideKeys};
    use crate::testing::{branch, commit, context, plan, question, state};

    /// A repository with `main` checked out at `a` and `feature` at `b`.
    fn two_branches() -> Context {
        let mut state = state();
        state.branches.push(branch("feature", 'b', false));
        context(state, Target::Head)
    }

    /// The comparison a plan opens: the names of its sides and its path.
    fn opened(outcome: Outcome) -> (String, String, Option<String>) {
        let plan = plan(outcome);
        let Some(FollowUp::OpenComparison(target)) = plan.after.first() else {
            panic!("opens a comparison: {plan:?}")
        };
        (
            target.first.name.clone(),
            target.second.name.clone(),
            target.path.clone(),
        )
    }

    #[test]
    fn compare_choose__main_then_feature_opens_main_with_feature() {
        let context = two_branches();
        let asked = question(choose(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Compare");
        let second = question(choose(&context, &picked(&[(FIRST, "ref:main")])));
        assert_eq!(second.title.as_str(), "Compare main with");
        let both = picked(&[(FIRST, "ref:main"), (SECOND, "ref:feature")]);
        assert_eq!(
            opened(choose(&context, &both)),
            (String::from("main"), String::from("feature"), None)
        );
    }

    #[test]
    fn compare_choose__the_working_tree_as_the_second_side() {
        let both = picked(&[(FIRST, "ref:main"), (SECOND, "working-tree")]);
        assert_eq!(
            opened(choose(&two_branches(), &both)),
            (String::from("main"), String::from("Working tree"), None)
        );
    }

    #[test]
    fn compare_choose__a_revision_typed_is_named_as_typed() {
        let context = two_branches();
        let keys = SideKeys::FIRST;
        let typed = picked(&[(FIRST, "revision")]);
        assert!(matches!(
            question(choose(&context, &typed)).kind,
            QuestionKind::Text { .. }
        ));
        let typed = typed.with(keys.typed, Answer::Text(String::from("HEAD~3")));
        assert_eq!(
            choose(&context, &typed),
            Outcome::Read(
                keys.commit,
                Query::Revision {
                    revision: String::from("HEAD~3")
                }
            )
        );
        let resolved = typed
            .with(keys.commit, Answer::Commit(Some(commit('c'))))
            .with(SECOND, Answer::Choice(String::from("ref:main")));
        assert_eq!(opened(choose(&context, &resolved)).0, "HEAD~3");
    }

    #[test]
    fn compare_choose__a_name_git_does_not_know_is_refused() {
        let keys = SideKeys::FIRST;
        let typed = picked(&[(FIRST, "revision")])
            .with(keys.typed, Answer::Text(String::from("nothing")))
            .with(keys.commit, Answer::Commit(None));
        let Outcome::Refuse(refusal) = choose(&two_branches(), &typed) else {
            panic!("refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "\"nothing\" is not a commit of this repository."
        );
    }

    #[test]
    fn compare_choose__an_entry_opened_before_opens_narrowed_as_it_was_without_a_second_question() {
        let recent = vec![RecentComparison {
            first: RecentSide {
                name: String::from("main"),
                commit: Some(commit('a')),
            },
            second: RecentSide {
                name: String::from("feature"),
                commit: Some(commit('b')),
            },
            path: Some(String::from("src")),
        }];
        let answers = Answers::new()
            .with(RECENT, Answer::Recent(recent))
            .with(FIRST, Answer::Choice(String::from("recent:0")))
            .with(REOPENED[0], Answer::Commit(Some(commit('a'))))
            .with(REOPENED[1], Answer::Commit(Some(commit('c'))));
        let (first, second, path) = opened(choose(&two_branches(), &answers));
        assert_eq!(
            (first.as_str(), second.as_str(), path.as_deref()),
            ("main", "feature", Some("src"))
        );
    }

    #[test]
    fn compare_choose__nothing_picked_opens_nothing() {
        let asked = question(choose(&two_branches(), &Answers::new()));
        assert!(matches!(asked.kind, QuestionKind::Pick { .. }));
    }
}
