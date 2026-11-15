//! `message.edit`: gives a commit a new message: the last one by amending it, an older one by an interactive rebase
//! that makes the commits after it again.

use std::ffi::OsString;

use model::CommitId;
use model::commit::Commit;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step, Text,
    Undo, Validation,
};
use model::rebase::{Reword, TodoAction, TodoStep, pushed_note};

use crate::history::{refuse_under_way, target_commit};
use crate::rebase::{Rewrite, rewrite_plan};

/// The new message, typed.
pub const MESSAGE: AnswerKey = AnswerKey("message");

/// The message the commit has.
const OLD: AnswerKey = AnswerKey("old-message");
/// The remote branches that have the commit.
const PUSHED: AnswerKey = AnswerKey("pushed");
/// Whether the commit is an ancestor of `HEAD`.
const BEFORE_HEAD: AnswerKey = AnswerKey("before-head");
/// The commit, read for its parents.
const EDITED: AnswerKey = AnswerKey("edited");
/// The commits after it up to `HEAD`.
const LINE: AnswerKey = AnswerKey("line");

/// Gives the commit it was invoked on the message typed, trimmed: the commit checked out with `git commit --amend
/// --only --allow-empty -F -`, which leaves out what is staged; an older one with an interactive rebase that rewords
/// it and makes the commits after it again, the changes not committed set aside and put back.
///
/// Feature: message.edit
#[must_use]
pub fn edit(context: &Context, answers: &Answers) -> Outcome {
    let commit = match target_commit(context) {
        Ok(commit) => commit,
        Err(outcome) => return *outcome,
    };
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    if context.repository.bisect.is_some() {
        // A new message makes the commit again, and the bisect would test commits that are no longer there.
        return Outcome::Refuse(Refusal::saying(crate::bisect::END_FIRST));
    }
    let Some(old) = answers.message(OLD) else {
        return Outcome::Read(OLD, Query::Message { commit: commit.clone() });
    };
    let is_head = context.repository.head.commit.as_ref() == Some(commit);
    let line = if is_head {
        Line::default()
    } else {
        match line(commit, answers) {
            Ok(line) => line,
            Err(outcome) => return *outcome,
        }
    };
    let Some(message) = answers.text(MESSAGE).map(str::trim) else {
        return Outcome::Ask(message_question(commit, old));
    };
    if message.is_empty() {
        return Outcome::Refuse(Refusal::saying("Enter a message."));
    }
    let Some(pushed) = answers.names(PUSHED) else {
        return Outcome::Read(PUSHED, Query::Pushed { commit: commit.clone() });
    };
    let note = pushed_note(commit, pushed);
    let edited = Edited { commit, message, old };
    if is_head {
        return Outcome::Run(amend_plan(context, &edited, note));
    }
    match rebase_plan(context, &edited, &line, note) {
        Ok(plan) => Outcome::Run(plan),
        Err(message) => Outcome::Refuse(Refusal { message, offer: None }),
    }
}

/// The commit edited, its new message and the one it has.
struct Edited<'a> {
    commit: &'a CommitId,
    message: &'a str,
    old: &'a str,
}

/// The line of commits after an older commit up to `HEAD`, newest first, and the commit before it, which the rebase
/// starts from; none for a root commit.
#[derive(Default)]
struct Line {
    after: Vec<CommitId>,
    base: Option<CommitId>,
}

/// The line after an older commit, once read, or what to read next, or the refusal when there is none.
fn line(commit: &CommitId, answers: &Answers) -> Result<Line, Box<Outcome>> {
    let read = |key, query| Box::new(Outcome::Read(key, query));
    let not_on_line = || {
        Box::new(Outcome::Refuse(Refusal::saying(&format!(
            "{} is not on the line of commits before HEAD without merges: its message cannot be changed by a rebase \
             from here.",
            commit.short()
        ))))
    };
    match answers.flag(BEFORE_HEAD) {
        None => {
            return Err(read(
                BEFORE_HEAD,
                Query::IsAncestor {
                    ancestor: commit.to_string(),
                    descendant: String::from("HEAD"),
                },
            ));
        }
        Some(false) => return Err(not_on_line()),
        Some(true) => {}
    }
    let Some(edited) = answers.commits(EDITED) else {
        return Err(read(
            EDITED,
            Query::Commits {
                revisions: vec![commit.to_string()],
            },
        ));
    };
    let Some(after) = answers.commits(LINE) else {
        return Err(read(LINE, Query::LineAfter { commit: commit.clone() }));
    };
    let parents = edited.first().map_or(&[][..], |edited| edited.parents.as_slice());
    if parents.len() > 1 || !is_single_line(after, commit) {
        return Err(not_on_line());
    }
    Ok(Line {
        after: after.iter().map(|commit| commit.id.clone()).collect(),
        base: parents.first().cloned(),
    })
}

/// Whether the commits read, newest first, follow one another down to the commit, each the only parent of the one
/// before.
fn is_single_line(after: &[Commit], commit: &CommitId) -> bool {
    let below = after
        .iter()
        .skip(1)
        .map(|commit| &commit.id)
        .chain(std::iter::once(commit));
    after
        .iter()
        .zip(below)
        .all(|(newer, older)| newer.parents.len() == 1 && newer.parents.first() == Some(older))
}

/// The box of the whole message, the message the commit has in it.
fn message_question(commit: &CommitId, old: &str) -> Question {
    Question {
        key: MESSAGE,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::NewMessage { old: old.to_owned() },
        },
        title: Text::from(format!("Edit the message of {}", commit.short())),
        detail: None,
        default: Some(model::contract::Answer::Text(old.to_owned())),
    }
}

/// `git commit --amend --only --allow-empty -F -`, the message on standard input.
fn amend_plan(context: &Context, edited: &Edited<'_>, note: Option<Text>) -> Plan {
    let arguments = ["commit", "--amend", "--only", "--allow-empty", "-F", "-"];
    Plan {
        title: Text::from(format!("Edit the message of {}", edited.commit.short())),
        steps: vec![Step::Git {
            arguments: arguments.iter().map(OsString::from).collect(),
            directory: context.repository.folder.clone(),
            input: Some(format!("{}\n", edited.message).into_bytes()),
        }],
        danger: note.map_or(Danger::None, Danger::Confirm),
        undo: Undo::ByReflog,
        after: vec![FollowUp::SelectHead],
        on_failure: Vec::new(),
    }
}

/// The interactive rebase that rewords an older commit and picks the commits after it, oldest first, with
/// `--autostash`: the new message and the todo list written to the folder of the files of the rebase, which is deleted
/// once the rebase ended, also when it stopped first on a commit picked after it; or why git could not read the list.
fn rebase_plan(context: &Context, edited: &Edited<'_>, line: &Line, note: Option<Text>) -> Result<Plan, Text> {
    let commit = edited.commit.to_string();
    let mut todo = vec![TodoStep {
        action: TodoAction::Reword,
        commit: &commit,
        reword: Some(Reword {
            message: edited.message,
            is_subject_only: false,
            original: edited.old,
        }),
    }];
    let picks: Vec<String> = line.after.iter().rev().map(ToString::to_string).collect();
    todo.extend(picks.iter().map(|pick| TodoStep {
        action: TodoAction::Pick,
        commit: pick,
        reword: None,
    }));
    rewrite_plan(
        context,
        Rewrite {
            steps: &todo,
            base: line.base.as_ref(),
            sets_aside: true,
            title: format!("Edit the message of {}", edited.commit.short()),
            note: rebase_note(context, edited.commit, line.after.len(), note).to_string(),
            after: vec![FollowUp::SelectBeforeHead(line.after.len())],
        },
    )
}

/// What the confirmation of the rebase says: the note of a commit pushed already, what is made again, and the changes
/// set aside.
fn rebase_note(context: &Context, commit: &CommitId, after: usize, pushed: Option<Text>) -> Text {
    let again = if after == 1 {
        String::from("the commit after it is made again on top of it, with new hashes.")
    } else {
        format!("the {after} commits after it are made again on top of it, with new hashes.")
    };
    let mut paragraphs: Vec<String> = pushed.into_iter().map(|note| note.as_str().to_owned()).collect();
    paragraphs.push(format!("{} gets the new message, and {again}", commit.short()));
    let working_tree = &context.repository.working_tree;
    if !(working_tree.staged.is_empty() && working_tree.unstaged.is_empty()) {
        paragraphs.push(String::from(
            "Changes that are not committed are set aside and put back.",
        ));
    }
    Text::from(paragraphs.join("\n\n"))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
