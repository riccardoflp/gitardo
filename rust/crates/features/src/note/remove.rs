//! `note.remove`: the note of a commit taken away.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Step, Text};

/// Takes the note of a commit away with `git notes remove`.
///
/// Feature: note.remove
#[must_use]
pub fn remove(context: &Context, _answers: &Answers) -> Outcome {
    let Some(commit) = super::commit_of(context) else {
        return Outcome::Refuse(Refusal::saying(super::ONLY_A_COMMIT));
    };
    if !context.repository.notes.has_note(commit) {
        return Outcome::Refuse(Refusal::saying(&format!("{} has no note.", commit.short())));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Remove the note of {}", commit.short())),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["notes", "remove", commit.as_str()],
        )],
        danger: Danger::None,
        undo: super::undo(context),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}
