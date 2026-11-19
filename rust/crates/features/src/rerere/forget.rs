//! `rerere.forget`: has git forget the resolution it recorded for the conflicts of a file, and marks them again.

use std::ffi::OsString;

use model::changes::pathspec;
use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Said when the file was not resolved as before during the operation under way.
const NOT_RESOLVED_AS_BEFORE: &str = "Git did not resolve this file as before.";

/// The warning of `rerere.forget` on a file.
#[must_use]
pub fn warning(path: &str) -> String {
    format!(
        "The resolution git recorded is forgotten, and what git wrote in {path}, with any change made to it since, is \
         lost: the conflicts are marked again."
    )
}

/// Runs `git rerere forget <path>`, then `git checkout --merge -- <path>`, which marks the conflicts again, also in a
/// file git staged. Offered on a file git resolved as before during the operation under way.
///
/// Feature: rerere.forget
#[must_use]
pub fn forget(context: &Context, _answers: &Answers) -> Outcome {
    let Target::File(path) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NOT_RESOLVED_AS_BEFORE));
    };
    let repository = &context.repository;
    if !repository.resolved_as_before.iter().any(|file| &file.path == path) {
        return Outcome::Refuse(Refusal::saying(NOT_RESOLVED_AS_BEFORE));
    }
    let folder = &repository.folder;
    let forget = vec![OsString::from("rerere"), "forget".into(), pathspec(path)];
    let mark = vec![
        OsString::from("checkout"),
        "--merge".into(),
        "--".into(),
        pathspec(path),
    ];
    Outcome::Run(Plan {
        title: Text::from(format!("Forget the resolution of {path}")),
        steps: vec![Step::git(folder.clone(), forget), Step::git(folder.clone(), mark)],
        danger: Danger::Destructive(Text::from(warning(path))),
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: the resolution recorded and the text of the file are not kept.",
        )),
        after: vec![FollowUp::Say(Text::from(format!(
            "{path} has its conflicts marked again: the resolution made next is recorded."
        )))],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
