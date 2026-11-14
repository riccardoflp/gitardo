//! `split.continue`: what **Continue** does while a rebase is stopped to split a commit.

use std::ffi::OsString;

use model::changes::pathspec;
use model::contract::{AnswerKey, Answers, Choice, Context, Outcome, Query, Question, QuestionKind, Step, Text};
use model::rebase::StopKind;
use model::state::FileChange;

use crate::rebase::{continue_plan, refuse};

/// The files of the commit split, both names of one renamed.
pub const FILES: AnswerKey = AnswerKey("files");

/// What to do with the changes of the commit not committed yet: commit them as one more commit, or go back.
pub const LEFT: AnswerKey = AnswerKey("left");

/// The message of the commit split.
pub const MESSAGE: AnswerKey = AnswerKey("message");

/// The choice of [`LEFT`] that commits what is left.
const COMMIT_LEFT: &str = "commit";

/// Goes on with the rebase once the changes of the commit split are committed: `git -c core.editor=true rebase
/// --continue`. When some of its files are still changed, or untracked once more after the commit added them, it asks
/// whether to commit them as one more commit with the message the commit had; other changes not committed keep it from
/// going on, untracked files left out.
///
/// Feature: split.continue
#[must_use]
pub fn continue_split(context: &Context, answers: &Answers) -> Outcome {
    let state = &context.repository;
    let Some(stop) = state.rebase_stop.as_ref().filter(|stop| stop.kind == StopKind::Split) else {
        return refuse("No commit is being split.");
    };
    if !state.working_tree.conflicted.is_empty() {
        return refuse("Resolve the conflicts and stage the files first.");
    }
    let Some(files) = answers.names(FILES) else {
        return Outcome::Read(
            FILES,
            Query::CommitFiles {
                commit: stop.commit.clone(),
            },
        );
    };
    let (mut left, others) = sorted_changes(&state.working_tree.staged, &state.working_tree.unstaged, files);
    // A file the commit added is untracked once its changes are taken out of it: it is left to commit too.
    for path in &state.working_tree.untracked {
        if files.contains(path) && !left.contains(path) {
            left.push(path.clone());
        }
    }
    if let Some(refusal) = refuse_others(&others) {
        return refusal;
    }
    let title = format!("Continue the split of {}", stop.commit.short());
    if left.is_empty() {
        return Outcome::Run(continue_plan(context, title, Vec::new()));
    }
    match answers.choice(LEFT) {
        None => return Outcome::Ask(left_question(&stop.subject, left.len())),
        Some(COMMIT_LEFT) => {}
        Some(_) => return refuse("Commit the rest of its changes, then Continue."),
    }
    let Some(message) = answers.messages(MESSAGE).and_then(<[String]>::first) else {
        return Outcome::Read(
            MESSAGE,
            Query::Messages {
                commits: vec![stop.commit.clone()],
            },
        );
    };
    let folder = state.folder.clone();
    let mut add: Vec<OsString> = ["add", "--all", "--"].iter().map(OsString::from).collect();
    add.extend(left.iter().map(|path| pathspec(path)));
    let steps = vec![
        Step::git(folder.clone(), add),
        Step::git(folder, ["commit", "--quiet", "-m", message.trim()]),
    ];
    Outcome::Run(continue_plan(context, title, steps))
}

/// The refusal while files that are not part of the split have changes not committed, naming the one there is.
fn refuse_others(others: &[String]) -> Option<Outcome> {
    match others {
        [] => None,
        [file] => Some(refuse(&format!(
            "{file} has changes that are not committed and are not part of the split: commit or stash them, then \
             continue."
        ))),
        many => Some(refuse(&format!(
            "{} files have changes that are not committed and are not part of the split: commit or stash them, \
             then continue.",
            many.len()
        ))),
    }
}

/// The paths of the changes not committed that belong to the commit split, by their path or their old path, and those
/// of the others, each once.
fn sorted_changes(staged: &[FileChange], unstaged: &[FileChange], files: &[String]) -> (Vec<String>, Vec<String>) {
    let mut left: Vec<String> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    for change in staged.iter().chain(unstaged) {
        let is_of_commit = files.contains(&change.path)
            || change
                .original_path
                .as_ref()
                .is_some_and(|original| files.contains(original));
        let list = if is_of_commit { &mut left } else { &mut others };
        if !list.contains(&change.path) {
            list.push(change.path.clone());
        }
    }
    (left, others)
}

/// The dialog that asks what to do with the files of the commit still changed.
fn left_question(subject: &str, count: usize) -> Question {
    let still = if count == 1 {
        String::from("1 file of it is still changed")
    } else {
        format!("{count} files of it are still changed")
    };
    Question {
        key: LEFT,
        kind: QuestionKind::Pick {
            choices: vec![
                Choice {
                    identifier: String::from(COMMIT_LEFT),
                    label: Text::from("Commit what is left"),
                    description: None,
                },
                Choice {
                    identifier: String::from("back"),
                    label: Text::from("Go back to committing"),
                    description: None,
                },
            ],
        },
        title: Text::from(format!("\"{subject}\" is not all committed yet: {still}.")),
        detail: Some(Text::from(
            "Commit what is left as one more commit, with the message the commit had, and continue? Or go back to \
             committing it yourself.",
        )),
        default: Some(model::contract::Answer::Choice(String::from(COMMIT_LEFT))),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
