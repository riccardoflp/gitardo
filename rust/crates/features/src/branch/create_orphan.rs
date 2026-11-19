//! `branch.create-orphan`: starts a branch that shares no commit with the others, whose first commit has no parent.

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text,
    Undo, UndoRecord, Validation,
};

/// The name typed for the new branch.
pub const NAME: AnswerKey = AnswerKey("name");

/// What the working tree keeps: [`NO_FILES`] or [`KEEP_FILES`].
pub const KEEPS: AnswerKey = AnswerKey("keeps");

/// The choice that takes the tracked files out of the working tree and the index: `git switch --orphan`.
pub const NO_FILES: &str = "no-files";

/// The choice that keeps the files, staged as new on the branch: `git checkout --orphan`.
pub const KEEP_FILES: &str = "keep-files";

/// Starts a branch with no history at `HEAD`'s place: the name, checked as `branch.create` checks it, then what the
/// working tree keeps. **Start with no files** runs `git switch --orphan <name>`, confirmed first, and is refused before
/// git runs while a tracked file has changes not committed; **Keep the files, staged** runs `git checkout --orphan
/// <name>`. Git writes no line in the reflog for the move, so the plan leaves its Undo in the journal: what was checked
/// out before. Refused while an operation or a bisect is under way, and before the first commit, when the branch
/// checked out has no history already.
///
/// Feature: branch.create-orphan
#[must_use]
pub fn create_orphan(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = crate::rebase::refuse_under_way(context) {
        return refusal;
    }
    if !context.repository.has_commit() {
        return Outcome::Refuse(Refusal::saying(
            "There is no commit yet: the first commit made here has no parent already.",
        ));
    }
    let Some(name) = answers.text(NAME) else {
        return Outcome::Ask(name_question(context));
    };
    let keeps_files = match answers.choice(KEEPS) {
        None => return Outcome::Ask(keeps_question()),
        Some(choice) => choice == KEEP_FILES,
    };
    let working_tree = &context.repository.working_tree;
    let has_tracked_changes = !working_tree.staged.is_empty() || !working_tree.unstaged.is_empty();
    if !keeps_files && has_tracked_changes {
        return Outcome::Refuse(Refusal::saying(
            "Commit or stash the changes first: starting with no files would take them out of the working tree.",
        ));
    }
    Outcome::Run(plan(context, name, keeps_files))
}

/// The name, in an input box with the placeholder `site`.
fn name_question(context: &Context) -> Question {
    let taken = context
        .repository
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("site")),
            validation: Validation::ReferenceName {
                taken,
                taken_message: Text::from("A branch with that name already exists"),
            },
        },
        title: Text::from("Create branch with no history"),
        detail: None,
        default: None,
    }
}

/// **What the working tree keeps**, starting on **Start with no files** (Q1125).
fn keeps_question() -> Question {
    let choice = |identifier: &str, label: &str, description: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: Some(Text::from(description)),
    };
    Question {
        key: KEEPS,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(
                    NO_FILES,
                    "Start with no files",
                    "The tracked files leave the working tree and the index; untracked and ignored files stay",
                ),
                choice(
                    KEEP_FILES,
                    "Keep the files, staged",
                    "The first commit holds the files as they are now",
                ),
            ],
        },
        title: Text::from("What the working tree keeps"),
        detail: None,
        default: Some(Answer::Choice(NO_FILES.to_owned())),
    }
}

/// `git switch --orphan <name>`, confirmed, or `git checkout --orphan <name>`; no update of the submodules follows,
/// since the new branch records none.
fn plan(context: &Context, name: &str, keeps_files: bool) -> Plan {
    let (arguments, danger) = if keeps_files {
        (["checkout", "--orphan", name], Danger::None)
    } else {
        let before = crate::history::current(context);
        (
            ["switch", "--orphan", name],
            Danger::Confirm(Text::from(format!(
                "The tracked files leave the working tree. {before} still has them, and gives them back when it is \
                 checked out again."
            ))),
        )
    };
    Plan {
        title: Text::from(format!("Create branch {name} with no history")),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger,
        undo: Undo::Journal(UndoRecord::BranchWithNoHistory {
            name: name.to_owned(),
            before: context.repository.head.clone(),
        }),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
