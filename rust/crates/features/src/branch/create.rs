//! `branch.create`: makes a new branch at a commit, a branch or a tag, and checks it out or not.

use model::branch::{
    FormatAsking, FormatName, FormatValues, RefKind, RefNames, base_of, day_of, name_from_format, ref_argument,
    repository_name,
};
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Step, Target,
    Text, Undo, UndoRecord, Validation,
};

use super::submodules::update_after_checkout;

/// The name typed for the new branch.
pub const NAME: AnswerKey = AnswerKey("name");

/// How the name is made, when `branches.format` is set: [`USE_FORMAT`] or [`TYPE_NAME`].
pub const WAY: AnswerKey = AnswerKey("way");

/// The choice that makes the name with the format.
pub const USE_FORMAT: &str = "format";

/// The choice that types the name.
pub const TYPE_NAME: &str = "typed";

/// Whether the new branch is checked out: [`CHECK_OUT`] or [`CREATE_ONLY`].
pub const MODE: AnswerKey = AnswerKey("mode");

/// The choice that checks the new branch out.
pub const CHECK_OUT: &str = "checkout";

/// The choice that leaves the branch checked out as it was.
pub const CREATE_ONLY: &str = "only";

/// Makes a new branch where the user invoked it: at `HEAD` from the toolbar and the palette, at a commit, a branch
/// or a tag from its menu. With `branches.format` set, asks first whether to use it, and then the parts of the format
/// (`branch.name-from-format`) and the name it makes, to change or keep; otherwise the name is typed. Then whether to
/// check the branch out, which ends with the update of the submodules (`submodule.update-on-checkout`).
///
/// Feature: branch.create
#[must_use]
pub fn create(context: &Context, answers: &Answers) -> Outcome {
    let format = context.settings.branch_format.trim();
    let from_format = if format.is_empty() {
        None
    } else {
        match answers.choice(WAY) {
            None => return Outcome::Ask(way_question(format)),
            Some(USE_FORMAT) => match name_from_format(&asking(context, format), answers) {
                Ok(made) => Some(made),
                Err(outcome) => return *outcome,
            },
            Some(_) => None,
        }
    };
    let Some(name) = answers.text(NAME) else {
        return Outcome::Ask(name_question(context, from_format.as_ref()));
    };
    let mode = match answers.choice(MODE) {
        None => return Outcome::Ask(mode_question(name)),
        Some(CHECK_OUT) => Mode::CheckOut,
        Some(_) => Mode::CreateOnly,
    };
    let (start, start_words) = start(context);
    let mut plan = plan(context, name, mode, &start, &start_words);
    if let Some(formatted) = from_format {
        plan.after.push(FollowUp::RememberInputs(formatted.inputs));
    }
    match mode {
        Mode::CheckOut => update_after_checkout(context, answers, &start, plan),
        Mode::CreateOnly => Outcome::Run(plan),
    }
}

/// What the questions of the format are made from: the inputs, the values used last in this workspace, what `${base}`
/// is where the feature was invoked, and today in the local time of the computer.
fn asking<'a>(context: &'a Context, format: &'a str) -> FormatAsking<'a> {
    let state = &context.repository;
    FormatAsking {
        format,
        inputs: &context.settings.branch_inputs,
        used: &context.settings.branch_values,
        base: base_of(state, &context.target),
        values: FormatValues {
            remotes: state.remotes.clone(),
            repository: repository_name(state),
            date: day_of(context.now, context.settings.utc_offset),
            ..FormatValues::default()
        },
        title: "Create branch",
    }
}

/// **Use the format**, the format beside it, where the pick starts, or **Type a branch name**.
fn way_question(format: &str) -> Question {
    let choice = |identifier: &str, label: &str, description: Option<&str>| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: description.map(Text::from),
    };
    Question {
        key: WAY,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(USE_FORMAT, "Use the format", Some(format)),
                choice(TYPE_NAME, "Type a branch name", None),
            ],
        },
        title: Text::from("Create branch"),
        detail: None,
        default: Some(Answer::Choice(USE_FORMAT.to_owned())),
    }
}

/// What happens to the new branch once it is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// It is checked out.
    CheckOut,
    /// What is checked out stays.
    CreateOnly,
}

/// The name typed, or the one the format made, to change or keep.
fn name_question(context: &Context, made: Option<&FormatName>) -> Question {
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
        title: Text::from(if made.is_some() {
            "Name of the new branch"
        } else {
            "Create branch"
        }),
        detail: None,
        default: made.map(|made| Answer::Text(made.name.clone())),
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

/// `git checkout -b <name> <start>`, with the new branch selected after, or `git branch <name> <start>`.
fn plan(context: &Context, name: &str, mode: Mode, start: &str, start_words: &str) -> Plan {
    let (arguments, undo, after) = match mode {
        Mode::CheckOut => (
            vec!["checkout", "-b", name, start],
            Undo::ByReflog,
            vec![FollowUp::SelectHead],
        ),
        Mode::CreateOnly => (
            vec!["branch", name, start],
            Undo::Journal(UndoRecord::BranchCreated { name: name.to_owned() }),
            Vec::new(),
        ),
    };
    Plan {
        title: Text::from(format!("Create branch {name} at {start_words}")),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo,
        after,
        on_failure: Vec::new(),
    }
}

/// Where the branch starts, as git is given it and as the title says it.
fn start(context: &Context) -> (String, String) {
    let names = RefNames::of(&context.repository);
    match &context.target {
        // A stash, a remote, files, a file, the list of the Changes page, a change of a Diff page, several
        // commits, a comparison, the file of a Merge page or of a File page, resolutions, a submodule, a worktree,
        // the settings of new worktrees, a path and some lines are no place to start a branch: it starts at what is
        // checked out.
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
        | Target::Resolutions(_)
        | Target::FileSave(_)
        | Target::FileAt(_)
        | Target::FileSaves(_)
        | Target::Submodule(_)
        | Target::Worktree(_)
        | Target::SetupSettings(_)
        | Target::Path(_)
        | Target::Lines(_)
        | Target::LineOfFile(_)
        | Target::ListedFile(_) => (String::from("HEAD"), String::from("HEAD")),
        Target::Commit(commit) => (commit.to_string(), commit.short().to_owned()),
        Target::Branch(name) => (ref_argument(RefKind::Branch, name, &names), name.clone()),
        Target::RemoteBranch(name) => (ref_argument(RefKind::RemoteBranch, name, &names), name.clone()),
        Target::Tag(name) => (ref_argument(RefKind::Tag, name, &names), name.clone()),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
