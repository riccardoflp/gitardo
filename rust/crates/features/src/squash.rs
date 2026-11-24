//! Squashing: several commits that follow one another before `HEAD` made into one, by an interactive rebase
//! (`docs/features/07-history-rewriting.md`).

mod commits;
mod into_previous;

use std::fmt::Write as _;

pub use commits::commits;
pub use into_previous::into_previous;

use model::CommitId;
use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Question, QuestionKind, Text, Validation};
use model::rebase::{LineCommit, Reword, TodoAction, TodoStep, commits_after, pushed_note, subject_of};
use model::squash::join_messages;

use crate::rebase::{LINE, Rewrite, line_holding, pushed, refuse, refuse_under_way, rewrite_note, rewrite_plan};

/// The subject of the commit they become.
pub const SUBJECT: AnswerKey = AnswerKey("subject");

/// Said when the commits are not the oldest of a line up to `HEAD` and those right after it.
const NOT_FOLLOWING: &str =
    "Only commits that follow one another on the line before HEAD, without merges, can be squashed into one.";

/// Squashes the commits selected, which must follow one another on the line before `HEAD`: the oldest is reworded
/// with the messages of them all under the subject answered, the others are fixups of it, and the commits after them
/// are made again on top, what is not committed set aside meanwhile.
fn squash(context: &Context, answers: &Answers, selected: &[CommitId]) -> Outcome {
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    if selected.len() < 2 {
        return refuse("Select two commits or more to squash them into one.");
    }
    let line = match line_holding(answers, selected) {
        Ok(read) => match read.commits.as_deref() {
            Some(line) => line,
            None => return refuse(NOT_FOLLOWING),
        },
        Err(outcome) => return *outcome,
    };
    let (squashed, rest) = line.split_at(selected.len().min(line.len()));
    if squashed.len() != selected.len() || !squashed.iter().all(|commit| selected.contains(&commit.id)) {
        return refuse(NOT_FOLLOWING);
    }
    let Some(oldest) = squashed.first() else {
        return refuse(NOT_FOLLOWING);
    };
    let Some(subject) = answers.text(SUBJECT) else {
        return Outcome::Ask(subject_question(squashed.len(), &oldest.message));
    };
    let names = match pushed(answers, &oldest.id) {
        Ok(names) => names,
        Err(read) => return *read,
    };
    let messages: Vec<&str> = squashed.iter().map(|commit| commit.message.as_str()).collect();
    let message = join_messages(subject, &messages);
    let mut steps = vec![TodoStep {
        action: TodoAction::Reword,
        commit: oldest.id.as_str(),
        reword: Some(Reword {
            message: &message,
            is_subject_only: false,
            original: &oldest.message,
        }),
    }];
    steps.extend(squashed.iter().skip(1).map(|commit| step(TodoAction::Fixup, commit)));
    steps.extend(rest.iter().map(|commit| step(TodoAction::Pick, commit)));
    let rewrite = Rewrite {
        steps: &steps,
        base: line_base(answers),
        sets_aside: true,
        title: format!("Squash {} commits", squashed.len()),
        note: format!(
            "{}\n\nThe message of the commit they become:\n\n{}",
            note(context, (squashed, rest), names),
            message.trim_end()
        ),
        after: vec![FollowUp::SelectBeforeHead(rest.len())],
    };
    match rewrite_plan(context, answers, rewrite) {
        Ok(plan) => Outcome::Run(plan),
        Err(outcome) => *outcome,
    }
}

/// What the confirmation of a squash says: the note of a pushed commit, the commits squashed, those made again on top,
/// and the changes set aside.
fn note(context: &Context, (squashed, rest): (&[LineCommit], &[LineCommit]), pushed: &[String]) -> String {
    let shorts: Vec<&str> = squashed.iter().map(|commit| commit.id.short()).collect();
    let mut middle = format!("{} become one commit, with the changes of them all.", shorts.join(", "));
    if !rest.is_empty() {
        let after = commits_after(rest.len(), "them");
        let _ = write!(middle, " {} made again on top of it.", capitalized(&after));
    }
    let oldest = squashed.first().map(|commit| pushed_note(&commit.id, pushed));
    rewrite_note(
        context,
        oldest.flatten(),
        &middle,
        "Changes that are not committed are set aside and put back.",
    )
}

/// The commit the rebase starts after: the parent of the oldest.
fn line_base(answers: &Answers) -> Option<&CommitId> {
    answers.line(LINE).and_then(model::rebase::LineToHead::base)
}

/// A step of a commit of the line.
fn step(action: TodoAction, commit: &LineCommit) -> TodoStep<'_> {
    TodoStep {
        action,
        commit: commit.id.as_str(),
        reword: None,
    }
}

/// A sentence that starts with a capital.
fn capitalized(words: &str) -> String {
    let mut characters = words.chars();
    characters
        .next()
        .map_or_else(String::new, |first| first.to_uppercase().chain(characters).collect())
}

/// The box of the subject of the commit they become, filled with the subject of the oldest.
fn subject_question(count: usize, oldest_message: &str) -> Question {
    Question {
        key: SUBJECT,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Required(Text::from("Enter a subject")),
        },
        title: Text::from(format!("Squash {count} commits")),
        detail: Some(Text::from(
            "Subject of the commit they become. What their messages say goes under it.",
        )),
        default: Some(model::contract::Answer::Text(subject_of(oldest_message).to_owned())),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
