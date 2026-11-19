//! `changes.skip` and `changes.unskip`: hiding the changes of a tracked file from git, and showing them to git again.

use std::ffi::OsString;

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FeatureId, Outcome, Query, Question, QuestionKind, Refusal,
    Step, Target, Text,
};
use model::state::SkippedFile;
use model::worktree_setup::Tracked;

use super::files::plan;
use super::stop_tracking::{PATH_STATUS, one_path};

/// The key of how git leaves the file alone: [`SKIP_WORKTREE`] or [`ASSUME_UNCHANGED`].
pub const HOW: AnswerKey = AnswerKey("skip-how");
/// The choice of skipping the file in the working tree.
pub const SKIP_WORKTREE: &str = "skip-worktree";
/// The choice of assuming the file is unchanged.
pub const ASSUME_UNCHANGED: &str = "assume-unchanged";

/// Has git leave the changes of a tracked file alone: `git update-index --skip-worktree -- <path>`, or
/// `--assume-unchanged`, the path given as it is, since `git update-index` takes paths, not patterns.
///
/// Feature: changes.skip
#[must_use]
pub fn skip(context: &Context, answers: &Answers) -> Outcome {
    let path = match one_path(context) {
        Ok(path) => path,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    if context.repository.skipped.iter().any(|file| file.path == path) {
        return Outcome::Refuse(Refusal::saying(&format!("{path} is hidden from git already.")));
    }
    let Some(Answer::PathStatuses(statuses)) = answers.get(PATH_STATUS) else {
        return Outcome::Read(PATH_STATUS, Query::PathStatuses { paths: vec![path] });
    };
    let status = statuses.first();
    if status.is_none_or(|status| status.tracked == Tracked::Nothing) {
        let mut refusal = Refusal::saying(&format!("Git does not track {path}: ignore it instead."));
        refusal.offer = Some((FeatureId("changes.ignore"), Text::from("Ignore\u{2026}")));
        return Outcome::Refuse(refusal);
    }
    if status.is_some_and(|status| status.is_folder) {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{path} is a folder: hide the changes of its files one by one."
        )));
    }
    let Some(how) = answers.choice(HOW) else {
        return Outcome::Ask(how_question(&path));
    };
    let option = if how == ASSUME_UNCHANGED {
        "--assume-unchanged"
    } else {
        "--skip-worktree"
    };
    Outcome::Run(plan(
        format!("Hide the changes of {path} from git"),
        vec![update_index(context, option, [path.as_str()])],
        Danger::None,
        "Show its changes to git again takes it back.",
    ))
}

/// Takes back what `changes.skip` did, for one file or, on every change, for every file hidden:
/// `git update-index --no-skip-worktree -- <path>…` for the files skipped, then `--no-assume-unchanged` for those assumed
/// unchanged, one command for each since git given both options at once changes only the first.
///
/// Feature: changes.unskip
#[must_use]
pub fn unskip(context: &Context, _answers: &Answers) -> Outcome {
    let hidden = &context.repository.skipped;
    let files: Vec<&SkippedFile> = match &context.target {
        Target::Changes => hidden.iter().collect(),
        _ => match one_path(context) {
            Ok(path) => hidden.iter().filter(|file| file.path == path).collect(),
            Err(refusal) => return Outcome::Refuse(refusal),
        },
    };
    if files.is_empty() {
        return Outcome::Refuse(Refusal::saying("No file is hidden from git there."));
    }
    let mut steps = Vec::new();
    let skipped: Vec<&str> = files
        .iter()
        .filter(|file| file.how.is_skip_worktree())
        .map(|file| file.path.as_str())
        .collect();
    if !skipped.is_empty() {
        steps.push(update_index(context, "--no-skip-worktree", skipped));
    }
    let assumed: Vec<&str> = files
        .iter()
        .filter(|file| file.how.is_assumed_unchanged())
        .map(|file| file.path.as_str())
        .collect();
    if !assumed.is_empty() {
        steps.push(update_index(context, "--no-assume-unchanged", assumed));
    }
    let title = match files.as_slice() {
        [file] => format!("Show the changes of {} to git again", file.path),
        _ => format!("Show the changes of {} files to git again", files.len()),
    };
    Outcome::Run(plan(
        title,
        steps,
        Danger::None,
        "Hide its changes from git hides it again.",
    ))
}

/// `git update-index <option> -- <path>…`, each path as it is.
pub(super) fn update_index<'a>(context: &Context, option: &str, paths: impl IntoIterator<Item = &'a str>) -> Step {
    let mut arguments = vec![
        OsString::from("update-index"),
        OsString::from(option),
        OsString::from("--"),
    ];
    arguments.extend(paths.into_iter().map(OsString::from));
    Step::git(context.repository.folder.clone(), arguments)
}

/// **How git leaves `<path>` alone**.
fn how_question(path: &str) -> Question {
    let choice = |identifier: &str, label: &str, description: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    };
    Question {
        key: HOW,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    SKIP_WORKTREE,
                    "Skip it in the working tree",
                    "Git keeps the file as committed and leaves your version on disk alone. A pull that changes it \
                     stops until you show its changes to git again.",
                ),
                choice(
                    ASSUME_UNCHANGED,
                    "Assume it is unchanged",
                    "A promise that the file does not change, for speed: git may look at it again, and a checkout may \
                     write over your changes.",
                ),
            ],
        },
        title: Text::from(format!("How git leaves {path} alone")),
        detail: None,
        default: Some(Answer::Choice(String::from(SKIP_WORKTREE))),
    }
}

#[cfg(test)]
#[path = "skip_tests.rs"]
mod tests;
