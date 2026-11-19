//! `lfs.unlock`: takes a lock away, one's own, or, after a warning, someone else's (Q1196).

use model::contract::{
    Answers, Context, Danger, FailureOffer, FollowUp, Outcome, Plan, Refusal, Step, Target, Text, Undo,
};
use model::lfs::LfsLock;

/// Words of what Git LFS says when it refuses to unlock a file with changes not committed.
pub const UNCOMMITTED: &str = "uncommitted changes";
/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: Lock takes it again, while no one else has.";

/// Runs `git lfs unlock <path>` for a lock of this user, offering **Unlock anyway** with `--force` when Git LFS refuses
/// for changes not committed; `git lfs unlock --force --id=<id>` for a lock of someone else, asked first since they
/// are not told. Refuses without Git LFS.
///
/// Feature: lfs.unlock
#[must_use]
pub fn unlock(context: &Context, _answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    let Target::LfsLock(lock) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose a locked file to unlock."));
    };
    let folder = context.repository.folder.clone();
    let id = format!("--id={}", lock.id);
    let forced = Step::git(folder.clone(), ["lfs", "unlock", "--force", id.as_str()]);
    if lock.is_ours == Some(false) {
        return Outcome::Run(Plan {
            danger: Danger::Confirm(Text::from(warning(lock))),
            ..plan(format!("Unlock {}, locked by {}", lock.path, lock.owner), forced)
        });
    }
    let anyway = plan(String::from("Unlock anyway"), forced);
    Outcome::Run(Plan {
        on_failure: vec![FollowUp::OfferWhenFailed(Box::new(FailureOffer {
            when: String::from(UNCOMMITTED),
            plan: anyway,
        }))],
        ..plan(
            format!("Unlock {}", lock.path),
            Step::git(folder, ["lfs", "unlock", lock.path.as_str()]),
        )
    })
}

/// A plan of one step that asks nothing.
fn plan(title: String, step: Step) -> Plan {
    Plan {
        title: Text::from(title),
        steps: vec![step],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// The warning before a lock of someone else is taken away.
#[must_use]
pub fn warning(lock: &LfsLock) -> String {
    format!(
        "The lock is {}'s, and they are not told: they may be changing the file.",
        lock.owner
    )
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
