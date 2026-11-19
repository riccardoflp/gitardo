//! `sync.deepen` and `sync.unshallow`: more of the history of a shallow clone fetched, a number of commits further back
//! or all of it.

use model::contract::{Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal};
use model::contract::{Step, Text, Undo, Validation, count_of};

use super::target::no_remote;

/// How many commits further back to fetch.
pub const COUNT: AnswerKey = AnswerKey("count");
/// How many commits further back are asked for when nothing else is typed (Q1063).
const DEFAULT_COUNT: &str = "100";

/// Asks how many commits further back to fetch, then runs `git fetch --deepen=<n> <remote>` from the remote of the
/// upstream of the branch checked out, or from the default one; refuses in a repository that is not shallow, and in one
/// with no remote, offering to add one.
///
/// Feature: sync.deepen
#[must_use]
pub fn deepen(context: &Context, answers: &Answers) -> Outcome {
    let remote = match shallow_remote(context) {
        Ok(remote) => remote,
        Err(refused) => return *refused,
    };
    let Some(count) = answers.text(COUNT) else {
        return Outcome::Ask(count_question());
    };
    let Some(count) = count_of(count) else {
        return Outcome::Ask(count_question());
    };
    fetch_plan(
        context,
        Text::from(format!("Load {count} more commits of history from {remote}")),
        [String::from("fetch"), format!("--deepen={count}"), remote],
    )
}

/// Runs `git fetch --unshallow <remote>`, from the remote `sync.deepen` fetches from; refuses as it does.
///
/// Feature: sync.unshallow
#[must_use]
pub fn unshallow(context: &Context, _answers: &Answers) -> Outcome {
    let remote = match shallow_remote(context) {
        Ok(remote) => remote,
        Err(refused) => return *refused,
    };
    fetch_plan(
        context,
        Text::from(format!("Load the whole history from {remote}")),
        [String::from("fetch"), String::from("--unshallow"), remote],
    )
}

/// The remote more history is fetched from: the remote of the upstream of the branch checked out, else the default one
/// (`remote.default`). Refused in a repository that is not shallow, or that has no remote.
fn shallow_remote(context: &Context) -> Result<String, Box<Outcome>> {
    let repository = &context.repository;
    if !repository.shallow.is_shallow() {
        return Err(Box::new(Outcome::Refuse(Refusal::saying(
            "This repository has its whole history already.",
        ))));
    }
    let upstream = repository.checked_out_branch().and_then(|name| {
        let branch = repository.branches.iter().find(|branch| branch.name == name)?;
        branch.upstream.as_ref().map(|upstream| upstream.remote.clone())
    });
    upstream
        .filter(|remote| repository.remotes.contains(remote))
        .or_else(|| model::remote::default(&repository.remotes))
        .ok_or_else(|| Box::new(no_remote()))
}

/// The question of how many commits further back to fetch.
fn count_question() -> Question {
    Question {
        key: COUNT,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Count,
        },
        title: Text::from("How many commits further back"),
        detail: None,
        default: Some(Answer::Text(String::from(DEFAULT_COUNT))),
    }
}

/// A fetch of more history, which Undo does not take back.
fn fetch_plan(context: &Context, title: Text, arguments: [String; 3]) -> Outcome {
    Outcome::Run(Plan {
        title,
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A fetch only adds commits, which Undo does not take back.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[path = "deepen_tests.rs"]
mod tests;
