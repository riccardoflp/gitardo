//! `worktree.lock` and `worktree.unlock`: has git keep a worktree whatever happens to its folder, with the reason why,
//! and takes the lock away; and the ask of `worktree.locked` before **Remove…** and **Rename…** on a locked worktree.

use std::path::Path;

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text, Undo,
    Validation,
};
use model::state::Worktree;
use model::worktree::{LockedInput, locked};

use super::names::choice;
use super::{label, target_worktree};

/// Why the worktree is locked, which may stay empty.
pub const REASON: AnswerKey = AnswerKey("reason");
/// Given by **Unlock and remove** and **Unlock and rename**.
pub const UNLOCK: AnswerKey = AnswerKey("unlock");
/// The one choice of [`UNLOCK`].
pub const UNLOCK_CHOICE: &str = "unlock";

/// Locks the worktree invoked on with the reason typed: `git worktree lock --reason <reason> <folder>`, or without
/// `--reason` when it is empty. A missing worktree may be locked, which is what a lock is most for.
///
/// Feature: worktree.lock
#[must_use]
pub fn lock(context: &Context, answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if worktree.is_main {
        return Outcome::Refuse(Refusal::saying(
            "Git cannot lock the main worktree, the folder the repository was created in.",
        ));
    }
    if worktree.locked.is_some() {
        return Outcome::Refuse(Refusal::saying("This worktree is locked already."));
    }
    let name = label(context, &worktree.folder);
    let Some(reason) = answers.text(REASON) else {
        return Outcome::Ask(Question {
            key: REASON,
            kind: QuestionKind::Text {
                placeholder: Some(Text::from("On a drive that is not always plugged in")),
                validation: Validation::Anything,
            },
            title: Text::from(format!("Why {name} is locked")),
            detail: Some(Text::from(
                "Prune and Remove leave a locked worktree alone. The reason is shown beside it.",
            )),
            default: None,
        });
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Lock worktree {name}")),
        steps: vec![lock_step(context, &worktree.folder, reason)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: Unlock on the worktree does.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// Unlocks the worktree invoked on: `git worktree unlock <folder>`.
///
/// Feature: worktree.unlock
#[must_use]
pub fn unlock(context: &Context, _answers: &Answers) -> Outcome {
    let worktree = match target_worktree(context) {
        Ok(worktree) => worktree,
        Err(refusal) => return *refusal,
    };
    if worktree.locked.is_none() {
        return Outcome::Refuse(Refusal::saying("This worktree is not locked."));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Unlock worktree {}", label(context, &worktree.folder))),
        steps: vec![unlock_step(context, &worktree.folder)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Lock… on the worktree locks it again.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// `git worktree lock`, with the reason when there is one.
pub(super) fn lock_step(context: &Context, folder: &Path, reason: &str) -> Step {
    let folder = folder.to_string_lossy().into_owned();
    let reason = reason.trim();
    let mut arguments = vec!["worktree", "lock"];
    if !reason.is_empty() {
        arguments.extend(["--reason", reason]);
    }
    arguments.push(&folder);
    Step::git(context.repository.folder.clone(), arguments)
}

/// `git worktree unlock <folder>`.
pub(super) fn unlock_step(context: &Context, folder: &Path) -> Step {
    let folder = folder.to_string_lossy().into_owned();
    Step::git(
        context.repository.folder.clone(),
        ["worktree", "unlock", folder.as_str()],
    )
}

/// What **Remove…** and **Rename…** ask first on a locked worktree, **`<label>` is locked: `<reason>`.** with the one
/// choice `action` (**Unlock and remove**, **Unlock and rename**); none once it was answered, or when it is not locked.
pub(super) fn ask_unlock(
    context: &Context,
    worktree: &Worktree,
    (answers, action): (&Answers, &str),
) -> Option<Outcome> {
    if answers.choice(UNLOCK).is_some() {
        return None;
    }
    let name = label(context, &worktree.folder);
    let rules = locked(&LockedInput {
        label: &name,
        reason: worktree.locked.as_deref(),
        is_missing: worktree.is_missing,
    });
    let ask = rules.ask?;
    Some(Outcome::Ask(Question {
        key: UNLOCK,
        kind: QuestionKind::Pick {
            choices: vec![choice(UNLOCK_CHOICE, action, None)],
        },
        title: Text::from(ask),
        detail: None,
        default: Some(Answer::Choice(UNLOCK_CHOICE.to_owned())),
    }))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
