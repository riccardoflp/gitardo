//! `note.edit`: the note of a commit written, a new one or in place of the one it had.

use std::ffi::OsString;

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step, Text, Validation,
};

/// The note typed.
pub const NOTE: AnswerKey = AnswerKey("note");
/// The note the commit has, read before the question.
pub const OLD_NOTE: AnswerKey = AnswerKey("old-note");

/// What is said of an empty note.
const EMPTY: &str = "A note cannot be empty: remove it instead.";

/// Writes the note of a commit with `git notes add -f -F -`, the text on its standard input: the note it has proposed
/// in the question, refused empty, and nothing run when it is the same.
///
/// Feature: note.edit
#[must_use]
pub fn edit(context: &Context, answers: &Answers) -> Outcome {
    let Some(commit) = super::commit_of(context) else {
        return Outcome::Refuse(Refusal::saying(super::ONLY_A_COMMIT));
    };
    let has_note = context.repository.notes.has_note(commit);
    let old = if has_note {
        match answers.text(OLD_NOTE) {
            Some(old) => old,
            None => return Outcome::Read(OLD_NOTE, Query::Note { commit: commit.clone() }),
        }
    } else {
        ""
    };
    let Some(typed) = answers.text(NOTE) else {
        return Outcome::Ask(Question {
            key: NOTE,
            kind: QuestionKind::Text {
                placeholder: None,
                validation: Validation::Required(Text::from(EMPTY)),
            },
            title: Text::from(format!("Note of {}", commit.short())),
            detail: None,
            default: has_note.then(|| model::contract::Answer::Text(old.to_owned())),
        });
    };
    let note = typed.trim();
    if note.is_empty() {
        return Outcome::Refuse(Refusal::saying(EMPTY));
    }
    if note == old.trim() {
        return Outcome::Refuse(Refusal::saying("The note is the same: nothing to change."));
    }
    let arguments = ["notes", "add", "-f", "-F", "-", commit.as_str()];
    let title = if has_note {
        format!("Edit the note of {}", commit.short())
    } else {
        format!("Add a note to {}", commit.short())
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::Git {
            arguments: arguments.iter().map(OsString::from).collect(),
            directory: context.repository.folder.clone(),
            input: Some(format!("{note}\n").into_bytes()),
        }],
        danger: Danger::None,
        undo: super::undo(context),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}
