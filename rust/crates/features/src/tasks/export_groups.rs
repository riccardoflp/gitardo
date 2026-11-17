//! `tasks.export-groups`: groups of the repository saved to a file another repository, or Run My Tasks in VS Code,
//! can import.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FileQuestion, FollowUp, Outcome, Plan, Question, QuestionKind,
    Refusal, Step, Text, Undo,
};
use model::tasks::TaskGroup;

use super::exchange::{PROPOSED_NAME, counted, exchange_text, filters};
use super::groups_file::kept_groups;

/// The key of the groups chosen, by their names.
pub const CHOSEN: AnswerKey = AnswerKey("groups");

/// The key of the path the file is saved to.
pub const PATH: AnswerKey = AnswerKey("path");

/// What the questions and the plan are called.
const TITLE: &str = "Export groups";

/// Saves groups of the repository to a file: asks which, all chosen, then where, in the save dialog, which starts in
/// the top folder of the worktree shown with `task-groups.json`. The file is the one Run My Tasks 0.8.0 reads, each
/// task by the name VS Code gives it, with no launch configurations.
///
/// Feature: tasks.export-groups
#[must_use]
pub fn export_groups(context: &Context, answers: &Answers) -> Outcome {
    let kept = match kept_groups(context, answers) {
        Ok(kept) => kept,
        Err(outcome) => return *outcome,
    };
    if kept.groups.is_empty() {
        return refuse("There are no groups to export.");
    }
    let Some(chosen) = answers.choices(CHOSEN) else {
        return Outcome::Ask(groups_question(&kept.groups));
    };
    let chosen: Vec<TaskGroup> = kept
        .groups
        .iter()
        .filter(|group| chosen.contains(&group.name))
        .cloned()
        .collect();
    if chosen.is_empty() {
        return refuse("No group was chosen: nothing was exported.");
    }
    let Some(path) = answers.path(PATH) else {
        return Outcome::Ask(Question {
            key: PATH,
            kind: QuestionKind::SaveFile(FileQuestion {
                folder: context.repository.folder.clone(),
                file_name: Some(String::from(PROPOSED_NAME)),
                filters: filters(),
            }),
            title: Text::from(TITLE),
            detail: None,
            default: None,
        });
    };
    let exported = counted(chosen.len(), "group", "groups");
    let saved = if chosen.len() == 1 { "was" } else { "were" };
    Outcome::Run(Plan {
        title: Text::from(format!("Export {exported}")),
        steps: vec![Step::Write {
            path: path.to_path_buf(),
            contents: exchange_text(&chosen).into_bytes(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "The file saved is outside what Undo takes back: delete it to take it back.",
        )),
        after: vec![FollowUp::Say(Text::from(format!(
            "{exported} {saved} saved in `{}`.",
            path.display()
        )))],
        on_failure: Vec::new(),
    })
}

/// Which groups to export, all chosen.
fn groups_question(groups: &[TaskGroup]) -> Question {
    let choice = |group: &TaskGroup| Choice {
        identifier: group.name.clone(),
        label: Text::from(group.name.as_str()),
        description: Some(Text::from(counted(group.tasks.len(), "task", "tasks"))),
    };
    Question {
        key: CHOSEN,
        kind: QuestionKind::PickMany {
            choices: groups.iter().map(choice).collect(),
        },
        title: Text::from(TITLE),
        detail: Some(Text::from(
            "The file can be imported in another repository, and by Run My Tasks in VS Code.",
        )),
        default: Some(Answer::Choices(groups.iter().map(|group| group.name.clone()).collect())),
    }
}

/// The refusal that says a sentence.
fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}

#[cfg(test)]
#[path = "export_groups_tests.rs"]
mod tests;
