//! `branch.create`: makes a new branch at a commit, a branch or a tag, and checks it out or not.

use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Target, Text, Undo,
    Validation,
};

/// The name typed for the new branch.
pub const NAME: AnswerKey = AnswerKey("name");

/// Whether the new branch is checked out: [`CHECK_OUT`] or [`CREATE_ONLY`].
pub const MODE: AnswerKey = AnswerKey("mode");

/// The choice that checks the new branch out.
pub const CHECK_OUT: &str = "checkout";

/// The choice that leaves the branch checked out as it was.
pub const CREATE_ONLY: &str = "only";

/// Makes a new branch where the user invoked it: at `HEAD` from the toolbar and the palette, at a commit, a branch
/// or a tag from its menu. Asks for the name, then whether to check it out.
///
/// The format of `branches.format` is asked for by `branch.name-from-format`, which is not built yet: until it is,
/// the name is always typed.
///
/// Feature: branch.create
#[must_use]
pub fn create(context: &Context, answers: &Answers) -> Outcome {
    let Some(name) = answers.text(NAME) else {
        return Outcome::Ask(name_question(context));
    };
    let mode = match answers.choice(MODE) {
        None => return Outcome::Ask(mode_question(name)),
        Some(CHECK_OUT) => Mode::CheckOut,
        Some(_) => Mode::CreateOnly,
    };
    Outcome::Run(plan(context, name, mode))
}

/// What happens to the new branch once it is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// It is checked out.
    CheckOut,
    /// What is checked out stays.
    CreateOnly,
}

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
            placeholder: Some(Text::from("feature/my-change")),
            validation: Validation::ReferenceName {
                taken,
                taken_message: Text::from("A branch with that name already exists"),
            },
        },
        title: Text::from("Create branch"),
        detail: None,
        default: None,
    }
}

fn mode_question(name: &str) -> Question {
    let choice = |identifier: &str, label: &str, description: Option<&str>| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: description.map(Text::from),
    };
    Question {
        key: MODE,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(CHECK_OUT, "Create and check out", None),
                choice(CREATE_ONLY, "Create only", Some("Stay on the current branch")),
            ],
        },
        title: Text::from(format!("Create branch `{name}`")),
        detail: None,
        default: None,
    }
}

/// `git checkout -b <name> <start>` or `git branch <name> <start>`.
fn plan(context: &Context, name: &str, mode: Mode) -> Plan {
    let (start, start_words) = start(context);
    let (arguments, undo) = match mode {
        Mode::CheckOut => (vec!["checkout", "-b", name, &start], Undo::ByReflog),
        Mode::CreateOnly => {
            let reason = Text::from("Undo does not take it back, since nothing moved HEAD.");
            (vec!["branch", name, &start], Undo::Impossible(reason))
        }
    };
    Plan {
        title: Text::from(format!("Create branch {name} at {start_words}")),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo,
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// Where the branch starts, as git is given it and as the title says it.
fn start(context: &Context) -> (String, String) {
    let names = RefNames::of(&context.repository);
    match &context.target {
        // A stash, a remote, files, a file, the list of the Changes page, a change of a Diff page, several
        // commits, a comparison, the file of a Merge page and resolutions are no place to start a branch: it starts at
        // what is checked out.
        Target::Head
        | Target::Stash(_)
        | Target::Remote(_)
        | Target::Files(_)
        | Target::File(_)
        | Target::Changes
        | Target::DiffChange(_)
        | Target::Commits(_)
        | Target::Comparison(_)
        | Target::MergeResult(_)
        | Target::Resolutions(_) => (String::from("HEAD"), String::from("HEAD")),
        Target::Commit(commit) => (commit.to_string(), commit.short().to_owned()),
        Target::Branch(name) => (ref_argument(RefKind::Branch, name, &names), name.clone()),
        Target::RemoteBranch(name) => (ref_argument(RefKind::RemoteBranch, name, &names), name.clone()),
        Target::Tag(name) => (ref_argument(RefKind::Tag, name, &names), name.clone()),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
