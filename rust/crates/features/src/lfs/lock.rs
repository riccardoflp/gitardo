//! `lfs.lock`: locks a file stored with Git LFS on the server, so that others know not to change it.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Target, Text, Undo};

/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: Unlock does.";

/// Runs `git lfs lock --json <path>`, the path from the top of the repository; the server's refusal, such as a file
/// locked by someone else meanwhile, is what Git LFS says. Refuses without Git LFS.
///
/// Feature: lfs.lock
#[must_use]
pub fn lock(context: &Context, _answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    let Target::File(path) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose a file stored with Git LFS to lock."));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Lock {path}")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["lfs", "lock", "--json", path.as_str()],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
