//! Stashes: setting changes aside, bringing them back, dropping them and making a branch of one
//! (`docs/features/08-stashes.md`).

mod apply;
mod branch;
mod drop;
mod pop;
mod push;
mod push_file;
mod push_staged;
mod undo_drop;

pub use apply::apply;
pub use branch::{NAME, branch};
pub use drop::drop;
pub use pop::pop;
pub use push::{MESSAGE, push};
pub use push_file::push_file;
pub use push_staged::push_staged;
pub(crate) use undo_drop::restore_plan;
pub use undo_drop::undo_drop;

use model::contract::{
    AnswerKey, Answers, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step, Text, Undo,
    Validation,
};
use model::stash::{StashPreviewInput, preview};
use model::state::Stash;

/// What is read of the stash before it is applied or popped.
pub const READ: AnswerKey = AnswerKey("stash");

/// Why stashing is not taken back by Undo.
const PUSH_UNDO: &str = "Undo does not take it back, as no branch moves: Pop brings the changes back.";

/// The stash a feature was invoked on, or the refusal when it is not there.
fn target_stash(context: &Context) -> Result<&Stash, Box<Outcome>> {
    let model::contract::Target::Stash(index) = context.target else {
        return Err(Box::new(Outcome::Refuse(Refusal::saying("Pick a stash first."))));
    };
    context
        .repository
        .stashes
        .iter()
        .find(|stash| stash.index == index)
        .ok_or_else(|| {
            Box::new(Outcome::Refuse(Refusal::saying(&format!(
                "stash@{{{index}}} is no longer there."
            ))))
        })
}

/// The refusal before the first commit, when git has nothing to stash onto.
fn refuse_before_first_commit(context: &Context) -> Option<Outcome> {
    (!context.repository.has_commit()).then(|| {
        Outcome::Refuse(Refusal::saying(
            "There is nothing to stash onto before the first commit.",
        ))
    })
}

/// The box of the optional message of a stash.
fn message_question(title: String, prompt: &str) -> Question {
    Question {
        key: push::MESSAGE,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from(title),
        detail: Some(Text::from(prompt)),
        default: None,
    }
}

/// `git stash push` with options, then `-m <message>` when one was typed, the message trimmed, then what follows it.
fn push_plan(context: &Context, title: &str, (options, message): (&[&str], &str), after_message: &[String]) -> Plan {
    let mut arguments: Vec<String> = ["stash", "push"]
        .iter()
        .chain(options)
        .map(|word| (*word).to_owned())
        .collect();
    let message = message.trim();
    if !message.is_empty() {
        arguments.push(String::from("-m"));
        arguments.push(message.to_owned());
    }
    arguments.extend(after_message.iter().cloned());
    Plan {
        title: Text::from(title),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(PUSH_UNDO)),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// The plan of `git stash apply` or `git stash pop` on the stash invoked on, always confirmed, its note saying what it
/// would stop on, once the stash is read.
fn bring_back(context: &Context, answers: &Answers, (verb, note): (&str, &str)) -> Outcome {
    let stash = match target_stash(context) {
        Ok(stash) => stash,
        Err(refusal) => return *refusal,
    };
    let Some(reading) = answers.stash(READ) else {
        return Outcome::Read(READ, Query::Stash { index: stash.index });
    };
    let working_tree = &context.repository.working_tree;
    let mut changed_here: Vec<String> = working_tree
        .staged
        .iter()
        .chain(&working_tree.unstaged)
        .map(|change| change.path.clone())
        .collect();
    changed_here.extend(working_tree.conflicted.iter().map(|file| file.path.clone()));
    let said = preview(&StashPreviewInput {
        files: &reading.files,
        changed_here: &changed_here,
        untracked_there: &reading.untracked_there,
        conflicts: reading.conflicts.as_deref(),
    });
    let mut paragraphs = vec![note.to_owned()];
    paragraphs.extend(said.paragraphs);
    let name = format!("stash@{{{}}}", stash.index);
    let capitalized = format!(
        "{}{}",
        verb.get(..1).unwrap_or_default().to_uppercase(),
        verb.get(1..).unwrap_or_default()
    );
    Outcome::Run(Plan {
        title: Text::from(format!("{capitalized} {name}")),
        steps: vec![Step::git(context.repository.folder.clone(), ["stash", verb, &name])],
        danger: Danger::Confirm(Text::from(paragraphs.join("\n\n"))),
        undo: Undo::Impossible(Text::from(
            "Not by the reflog, which it does not move: discard the changes to take it back.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}
