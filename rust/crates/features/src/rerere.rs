//! Git's rerere, which records how conflicts are resolved and writes the same resolution the next time
//! (`docs/features/09-conflicts.md`, D84): turning it on and off for one repository, and forgetting a resolution.

mod forget;

pub use forget::{forget, warning as forget_warning};

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal,
    Step, Text, Undo,
};

/// The answer of what git does with a conflict it resolved before.
pub const WHEN_RESOLVED: AnswerKey = AnswerKey("when-resolved-before");
/// The answer that leaves the file git resolved for the user to look at.
pub const LEAVE: &str = "leave";
/// The answer that has git stage the file it resolved.
pub const STAGE: &str = "stage";
/// The question of `rerere.turn-on`.
const QUESTION: &str = "When git meets a conflict it resolved before";
/// Said when it is on already.
const ON_ALREADY: &str = "Git already remembers how conflicts are resolved in this repository.";
/// Said when it is off already.
const OFF_ALREADY: &str = "Git does not remember how conflicts are resolved in this repository.";

/// Turns git's recording of resolutions on for this repository, asking whether git also stages what it writes:
/// `git config --local rerere.enabled true`, and `rerere.autoUpdate true` with the second answer.
///
/// Feature: rerere.turn-on
#[must_use]
pub fn turn_on(context: &Context, answers: &Answers) -> Outcome {
    if context.repository.rerere.is_on {
        return Outcome::Refuse(Refusal::saying(ON_ALREADY));
    }
    let Some(when) = answers.choice(WHEN_RESOLVED) else {
        return Outcome::Ask(question());
    };
    let folder = &context.repository.folder;
    let mut steps = vec![Step::git(
        folder.clone(),
        ["config", "--local", "rerere.enabled", "true"],
    )];
    if when == STAGE {
        steps.push(Step::git(
            folder.clone(),
            ["config", "--local", "rerere.autoUpdate", "true"],
        ));
    }
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Outcome::Run(Plan {
        title: Text::from("Remember how conflicts are resolved"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Remember how conflicts are resolved, ticked in the same menu, turns it off.",
        )),
        after: vec![FollowUp::Say(Text::from(format!(
            "Git now records how you resolve conflicts in {name}, and resolves the same conflicts the same way."
        )))],
        on_failure: Vec::new(),
    })
}

/// The question of what git does with a conflict it resolved before.
fn question() -> Question {
    let choice = |identifier: &str, label: &str, description: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    };
    Question {
        key: WHEN_RESOLVED,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    LEAVE,
                    "Write the same resolution, and leave the file for me to look at",
                    "The file stays in conflict, without markers, until you mark it as resolved.",
                ),
                choice(
                    STAGE,
                    "Write the same resolution and stage it",
                    "The file is staged at once: look at it before you continue.",
                ),
            ],
        },
        title: Text::from(QUESTION),
        detail: None,
        default: Some(Answer::Choice(String::from(LEAVE))),
    }
}

/// Turns git's recording of resolutions off: `git config --local rerere.enabled false`, rather than taking the key
/// away, since a folder `rr-cache` would keep it on; then takes `rerere.autoUpdate` away when the repository sets it.
/// What git recorded stays.
///
/// Feature: rerere.turn-off
#[must_use]
pub fn turn_off(context: &Context, _answers: &Answers) -> Outcome {
    let rerere = context.repository.rerere;
    if !rerere.is_on {
        return Outcome::Refuse(Refusal::saying(OFF_ALREADY));
    }
    let folder = &context.repository.folder;
    let mut steps = vec![Step::git(
        folder.clone(),
        ["config", "--local", "rerere.enabled", "false"],
    )];
    if rerere.sets_stages {
        steps.push(Step::git(
            folder.clone(),
            ["config", "--local", "--unset", "rerere.autoUpdate"],
        ));
    }
    Outcome::Run(Plan {
        title: Text::from("Stop remembering how conflicts are resolved"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Remember how conflicts are resolved… turns it on again.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
