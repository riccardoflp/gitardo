//! The reflog: where `HEAD` has been, to go back there or recover a commit no branch reaches any more
//! (`docs/features/11-history-and-blame.md`).

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal,
    Target, Text, Undo,
};
use model::reflog::ReflogPosition;

/// The key of the positions of `HEAD` read.
pub const POSITIONS: AnswerKey = AnswerKey("reflog");
/// The key of the entry picked, by its selector: `HEAD@{3}`.
pub const ENTRY: AnswerKey = AnswerKey("reflog-entry");
/// The key of what is done with the entry picked.
pub const ACTION: AnswerKey = AnswerKey("reflog-action");

/// How many positions of `HEAD` are listed.
const LISTED: usize = 200;

/// What can be done with an entry: its identifier, the feature it calls, and its words; **Show in the timeline** calls
/// none.
const ACTIONS: [(&str, Option<&str>, &str); 5] = [
    ("show", None, "Show in the timeline"),
    ("branch", Some("branch.create"), "Create branch here\u{2026}"),
    ("reset", Some("reset.to-commit"), "Reset {branch} to here\u{2026}"),
    ("checkout", Some("branch.checkout-detached"), "Check out (detached)"),
    ("cherry-pick", Some("cherry-pick.commit"), "Cherry-pick"),
];

/// Lists the last 200 positions of `HEAD`, newest first, then what can be done with the one picked: show it in the
/// timeline, or call the feature that makes a branch there, resets to it, checks it out or cherry-picks it, with the
/// commit of the entry. Only the actions whose feature is built are offered.
///
/// Feature: reflog.browse
#[must_use]
pub fn browse(context: &Context, answers: &Answers) -> Outcome {
    let Some(positions) = answers.positions(POSITIONS) else {
        return Outcome::Read(
            POSITIONS,
            Query::ReflogPositions {
                reference: String::from("HEAD"),
                count: LISTED,
            },
        );
    };
    if positions.is_empty() {
        return Outcome::Refuse(Refusal::saying("HEAD has no reflog yet."));
    }
    let picked = answers
        .choice(ENTRY)
        .and_then(|selector| positions.iter().find(|position| position.selector == selector));
    let Some(position) = picked else {
        return Outcome::Ask(entries(positions));
    };
    match answers.choice(ACTION) {
        Some("show") => Outcome::Run(show(&position.commit)),
        Some(action) => match ACTIONS
            .iter()
            .find(|(identifier, ..)| *identifier == action)
            .and_then(|(_, feature, _)| crate::operation((*feature)?))
        {
            Some((_, operation)) => {
                let called = Context {
                    target: Target::Commit(position.commit.clone()),
                    ..context.clone()
                };
                operation(&called, answers)
            }
            None => Outcome::Ask(actions(context, position)),
        },
        None => Outcome::Ask(actions(context, position)),
    }
}

/// The first pick: each entry with what moved `HEAD` there, its short hash and its selector.
fn entries(positions: &[ReflogPosition]) -> Question {
    Question {
        key: ENTRY,
        kind: QuestionKind::Pick {
            choices: positions
                .iter()
                .take(LISTED)
                .map(|position| Choice {
                    identifier: position.selector.clone(),
                    label: Text::from(position.subject.as_str()),
                    description: Some(Text::from(format!(
                        "{} \u{b7} {}",
                        position.commit.short(),
                        position.selector
                    ))),
                })
                .collect(),
        },
        title: Text::from("Reflog: where HEAD has been"),
        detail: Some(Text::from("Pick an entry to go back to it or recover it")),
        default: None,
    }
}

/// The second pick: what to do with the entry, among the actions whose feature is built.
fn actions(context: &Context, position: &ReflogPosition) -> Question {
    let branch = context.repository.checked_out_branch().unwrap_or("HEAD");
    Question {
        key: ACTION,
        kind: QuestionKind::Pick {
            choices: ACTIONS
                .iter()
                .filter(|(_, feature, _)| feature.is_none_or(|feature| crate::operation(feature).is_some()))
                .map(|(identifier, _, label)| Choice {
                    identifier: (*identifier).to_owned(),
                    label: Text::from(label.replace("{branch}", branch)),
                    description: None,
                })
                .collect(),
        },
        title: Text::from(format!("{}: {}", position.commit.short(), position.subject)),
        detail: None,
        default: None,
    }
}

/// **Show in the timeline**: selects the commit, changing nothing in the repository.
fn show(commit: &CommitId) -> Plan {
    Plan {
        title: Text::from(format!("Show {} in the timeline", commit.short())),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Showing a commit changes nothing to undo.")),
        after: vec![FollowUp::SelectCommit(commit.clone())],
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::Timestamp;
    use model::contract::{Answer, Answers, Outcome, Query, QuestionKind, Target};
    use model::reflog::ReflogPosition;

    use super::{ACTION, ENTRY, POSITIONS, browse};
    use crate::testing::{commit, context, plan, question, state};

    /// Positions of `HEAD` at the commits of some digits, newest first.
    fn positions(digits: &[char]) -> Answers {
        let positions = digits
            .iter()
            .enumerate()
            .map(|(place, &digit)| ReflogPosition {
                commit: commit(digit),
                subject: format!("commit: Add {digit}"),
                selector: format!("HEAD@{{{place}}}"),
                time: Timestamp(0),
            })
            .collect();
        Answers::new().with(POSITIONS, Answer::Positions(positions))
    }

    #[test]
    fn reflog_browse__lists_at_most_200_entries_of_head_newest_first() {
        let context = context(state(), Target::Head);
        assert_eq!(
            browse(&context, &Answers::new()),
            Outcome::Read(
                POSITIONS,
                Query::ReflogPositions {
                    reference: String::from("HEAD"),
                    count: 200
                }
            )
        );
        let asked = question(browse(&context, &positions(&['3', '2', '1'])));
        assert_eq!(asked.title.as_str(), "Reflog: where HEAD has been");
        let QuestionKind::Pick { choices } = asked.kind else {
            panic!("a pick")
        };
        let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
        assert_eq!(labels, ["commit: Add 3", "commit: Add 2", "commit: Add 1"]);
        let description = choices
            .first()
            .and_then(|choice| choice.description.clone())
            .unwrap_or_default();
        assert_eq!(
            description.as_str(),
            format!("{} \u{b7} HEAD@{{0}}", commit('3').short())
        );
    }

    #[test]
    fn reflog_browse__create_branch_here_calls_branch_create_with_the_commit_of_the_entry() {
        let context = context(state(), Target::Head);
        let picked = positions(&['3', '2']).with(ENTRY, Answer::Choice(String::from("HEAD@{1}")));
        let asked = question(browse(&context, &picked));
        assert_eq!(asked.title.as_str(), format!("{}: commit: Add 2", commit('2').short()));
        let chosen = picked.with(ACTION, Answer::Choice(String::from("branch")));
        let called = crate::testing::context(state(), Target::Commit(commit('2')));
        assert_eq!(
            browse(&context, &chosen),
            crate::branch::create(&called, &chosen),
            "the question of branch.create, at the commit"
        );
    }

    #[test]
    fn reflog_browse__show_in_the_timeline_selects_the_commit() {
        let context = context(state(), Target::Head);
        let chosen = positions(&['3', '2'])
            .with(ENTRY, Answer::Choice(String::from("HEAD@{1}")))
            .with(ACTION, Answer::Choice(String::from("show")));
        let plan = plan(browse(&context, &chosen));
        assert_eq!(plan.steps, []);
        assert_eq!(plan.after, [model::contract::FollowUp::SelectCommit(commit('2'))]);
    }

    #[test]
    fn reflog_browse__closing_either_pick_runs_nothing() {
        let context = context(state(), Target::Head);
        assert!(matches!(browse(&context, &positions(&['3', '2'])), Outcome::Ask(_)));
        let picked = positions(&['3', '2']).with(ENTRY, Answer::Choice(String::from("HEAD@{0}")));
        assert!(matches!(browse(&context, &picked), Outcome::Ask(_)));
    }
}
