//! `changes.stop-tracking`: takes a file, or the files of a folder, out of the index, so that the next commit deletes it
//! from the repository while it stays on disk.

use std::path::PathBuf;

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FailureOffer, FollowUp, Outcome, Query, Refusal, Step, Target, Text,
};
use model::worktree_setup::Tracked;

use super::files::{plan, with_files};

/// The key of what git says of the path: whether it tracks it, and whether it is a folder.
pub(super) const PATH_STATUS: AnswerKey = AnswerKey("stop-tracking-path-status");

/// Words of what git says when the index has a version that is neither the last commit nor the file on disk.
pub(super) const STAGED_DIFFERENT: &str = "staged content different from both";

/// How the deletion is taken back while it is not committed.
const UNSTAGE_TAKES_IT_BACK: &str = "Unstaging the deletion takes it back while it is not committed.";

/// Takes a file, or every file of a folder, out of the index and leaves it on disk: `git rm --cached -- <path>`, with
/// `-r` for a folder; when git refuses because the index has a version found nowhere else, **Stop tracking anyway**
/// is offered, the same with `--force`.
///
/// Feature: changes.stop-tracking
#[must_use]
pub fn stop_tracking(context: &Context, answers: &Answers) -> Outcome {
    let path = match one_path(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let Some(Answer::PathStatuses(statuses)) = answers.get(PATH_STATUS) else {
        return Outcome::Read(PATH_STATUS, Query::PathStatuses { paths: vec![path] });
    };
    let Some(status) = statuses.first().filter(|status| status.tracked != Tracked::Nothing) else {
        return Outcome::Refuse(Refusal::saying(&format!("Git does not track {path}.")));
    };
    let folder = context.repository.folder.clone();
    let (step, offer) = untrack_steps(folder, &path, status.is_folder);
    let mut stopped = plan(
        format!("Stop tracking {path}"),
        vec![step],
        Danger::Confirm(Text::from(stays_on_disk(&path))),
        UNSTAGE_TAKES_IT_BACK,
    );
    stopped.on_failure.push(offer);
    Outcome::Run(stopped)
}

/// The path an operation on one file or folder was invoked on, from the top with `/`: a file of the changes, or a file
/// or a folder of the Files page, which gives it on disk; refused when there is none, when it is in conflict, or when
/// it is in a submodule.
pub(super) fn one_path(context: &Context) -> Result<String, Refusal> {
    let path = match &context.target {
        Target::Path(on_disk) => on_disk
            .strip_prefix(&context.repository.folder)
            .map(|inside| inside.to_string_lossy().into_owned())
            .map_err(|_| Refusal::saying(&format!("{} is not in this repository.", on_disk.display())))?,
        _ => super::files::target_file(context)?.to_owned(),
    };
    let path = path.replace('\\', "/").trim_matches('/').to_owned();
    if path.is_empty() {
        return Err(Refusal::saying(super::files::CHOOSE_A_FILE));
    }
    let tree = &context.repository.working_tree;
    if tree.conflicted.iter().any(|file| file.path == path) {
        return Err(Refusal::saying(&format!("{path} is in conflict: resolve it first.")));
    }
    let submodule = context.repository.submodules.iter().find(|submodule| {
        path.strip_prefix(submodule.folder.as_str())
            .is_some_and(|rest| rest.starts_with('/'))
    });
    if let Some(submodule) = submodule {
        return Err(Refusal::saying(&format!(
            "{path} is in the submodule {}: ignore it there.",
            submodule.name
        )));
    }
    Ok(path)
}

/// The step that takes a path out of the index, and the offer of the same with `--force` when git refuses it.
pub(super) fn untrack_steps(folder: PathBuf, path: &str, is_folder: bool) -> (Step, FollowUp) {
    let arguments: &[&str] = if is_folder {
        &["rm", "--cached", "-r"]
    } else {
        &["rm", "--cached"]
    };
    let forced_arguments: &[&str] = if is_folder {
        &["rm", "--cached", "-r", "--force"]
    } else {
        &["rm", "--cached", "--force"]
    };
    let step = with_files(folder.clone(), arguments, [path]);
    let lost = format!("The version of {path} that is staged is lost: it is neither committed nor on disk.");
    let forced = plan(
        String::from("Stop tracking anyway"),
        vec![with_files(folder, forced_arguments, [path])],
        Danger::Destructive(Text::from(lost)),
        UNSTAGE_TAKES_IT_BACK,
    );
    let offer = FollowUp::OfferWhenFailed(Box::new(FailureOffer {
        when: String::from(STAGED_DIFFERENT),
        plan: forced,
    }));
    (step, offer)
}

/// The warning of stopping tracking a path.
pub(super) fn stays_on_disk(path: &str) -> String {
    format!(
        "{path} stays on your disk. Once committed, it is deleted from the repository, and from the disk of whoever \
         pulls that commit."
    )
}

#[cfg(test)]
#[path = "stop_tracking_tests.rs"]
mod tests;
