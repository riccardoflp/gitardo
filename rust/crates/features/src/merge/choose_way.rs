//! `merge.choose-way`: merges into what is checked out in the way picked: as git's configuration says, always with a
//! merge commit, only as a fast-forward, or as one change staged for the user to commit.

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step,
    Text, Undo, UndoRecord,
};

use super::{merged, predicted};
use crate::history::{Brought, conflicts_danger, current};

/// The way to merge, picked: `merge`, `no-ff`, `ff-only` or `squash`.
pub const WAY: AnswerKey = AnswerKey("way");

/// Whether `HEAD` is an ancestor of what is merged: a fast-forward is possible.
const CAN_FAST_FORWARD: AnswerKey = AnswerKey("can-fast-forward");

/// The commits each side has that the other has not, read to say why a fast-forward is not possible.
const COUNTS: AnswerKey = AnswerKey("counts");

/// The four ways, by identifier, with the option of `git merge` each adds, in the order of the pick.
const WAYS: [(&str, Option<&str>); 4] = [
    ("merge", None),
    ("no-ff", Some("--no-ff")),
    ("ff-only", Some("--ff-only")),
    ("squash", Some("--squash")),
];

/// Asks which of the four ways to merge, then merges that way: `git merge [--no-ff | --ff-only | --squash] <ref>`.
/// A fast-forward that is not possible is refused without running git; a squash merge leaves its changes staged, with
/// what Undo needs in the journal, since it moves no branch.
///
/// Feature: merge.choose-way
#[must_use]
pub fn choose_way(context: &Context, answers: &Answers) -> Outcome {
    let merged = match merged(context) {
        Ok(merged) => merged,
        Err(outcome) => return *outcome,
    };
    let Some(can_fast_forward) = answers.flag(CAN_FAST_FORWARD) else {
        return Outcome::Read(
            CAN_FAST_FORWARD,
            Query::IsAncestor {
                ancestor: String::from("HEAD"),
                descendant: merged.argument,
            },
        );
    };
    let Some(way) = answers.choice(WAY) else {
        return Outcome::Ask(way_question(context, &merged, can_fast_forward));
    };
    if way == "ff-only" {
        return fast_forward_only(context, answers, &merged, can_fast_forward);
    }
    let conflicts = match predicted(answers, &merged) {
        Ok(conflicts) => conflicts,
        Err(outcome) => return *outcome,
    };
    let option = WAYS
        .iter()
        .find(|(identifier, _)| *identifier == way)
        .and_then(|(_, option)| *option);
    let mut plan = merge_plan(context, &merged, option);
    plan.danger = conflicts_danger(None, conflicts);
    if way == "squash" {
        squash(context, &merged, &mut plan, conflicts);
    }
    Outcome::Run(plan)
}

/// The pick of the four ways, each with its command and what it does.
fn way_question(context: &Context, merged: &Brought, can_fast_forward: bool) -> Question {
    let into = current(context);
    let name = &merged.name;
    let fast_forward = if can_fast_forward {
        format!("{into} moves up to {name}, with no merge commit")
    } else {
        format!("Not possible: {into} has commits that {name} has not")
    };
    let said = [
        (
            "Merge",
            String::from("A fast-forward when one is possible, otherwise a merge commit"),
        ),
        (
            "Merge (no fast-forward)",
            String::from("Always a merge commit, even where a fast-forward would do"),
        ),
        ("Merge (fast-forward only)", fast_forward),
        (
            "Merge (squash)",
            format!("The changes of {name} staged as one, for you to commit, with no merge recorded"),
        ),
    ];
    let choices = WAYS
        .iter()
        .zip(said)
        .map(|((identifier, option), (label, what))| {
            let command = match option {
                Some(option) => format!("git merge {option} {}", merged.argument),
                None => format!("git merge {}", merged.argument),
            };
            Choice {
                identifier: (*identifier).to_owned(),
                label: Text::from(label),
                description: Some(Text::from(format!("{command} · {what}"))),
            }
        })
        .collect();
    Question {
        key: WAY,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("Merge {name} into {into}")),
        detail: None,
        default: None,
    }
}

/// `git merge [<option>] <ref>`, undone by the reflog.
fn merge_plan(context: &Context, merged: &Brought, option: Option<&str>) -> Plan {
    let mut arguments = vec!["merge"];
    arguments.extend(option);
    arguments.push(&merged.argument);
    let how = match option {
        Some("--no-ff") => " (no fast-forward)",
        Some("--ff-only") => " (fast-forward only)",
        Some("--squash") => " (squash)",
        _ => "", // The plain merge says nothing more.
    };
    Plan {
        title: Text::from(format!("Merge {} into {}{how}", merged.name, current(context))),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// A fast-forward only: run when possible, refused with why otherwise, once the counts are read.
fn fast_forward_only(context: &Context, answers: &Answers, merged: &Brought, can_fast_forward: bool) -> Outcome {
    if can_fast_forward {
        return Outcome::Run(merge_plan(context, merged, Some("--ff-only")));
    }
    let Some((_, theirs)) = answers.counts(COUNTS) else {
        return Outcome::Read(
            COUNTS,
            Query::Counts {
                left: String::from("HEAD"),
                right: merged.argument.clone(),
            },
        );
    };
    let into = current(context);
    let name = &merged.name;
    let message = if theirs == 0 {
        format!("{into} already has every commit of {name}.")
    } else {
        format!(
            "{into} cannot be fast-forwarded to {name}: it has commits that {name} has not. Merge it another way, or \
             rebase {into} onto it."
        )
    };
    Outcome::Refuse(Refusal::saying(&message))
}

/// What a squash merge adds to its plan: the note that asks first, the journal's Undo, and what follows it.
fn squash(context: &Context, merged: &Brought, plan: &mut Plan, conflicts: Option<&[String]>) {
    let into = current(context);
    let note = format!(
        "The changes of {} are staged in {into} as one change, for you to commit. No merge is recorded: {} will not \
         show as merged.",
        merged.name, merged.name
    );
    plan.danger = conflicts_danger(Some(&note), conflicts);
    plan.undo = Undo::Journal(UndoRecord::SquashMerge);
    plan.after = vec![
        FollowUp::Say(Text::from(format!(
            "The changes are staged, and nothing is committed yet. Commit them to make them one commit on {into}."
        ))),
        FollowUp::PreparedMessage,
    ];
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
