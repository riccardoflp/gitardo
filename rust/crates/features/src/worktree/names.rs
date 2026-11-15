//! The questions the operations that make worktrees share: the format of branch names or a name typed, and the name
//! of the new branch and of the folder, checked while it is typed.

use model::branch::{BaseOf, FormatAsking, FormatName, FormatValues, day_of, name_from_format, repository_name};
use model::contract::{Answer, AnswerKey, Answers, Choice, Context, Outcome, Question, QuestionKind, Text, Validation};
use model::worktree::{FolderBase, WorktreeNameRule};

use super::{folder_of_worktrees, taken_folders};

/// The choice that makes the name with `branches.format`.
pub(super) const USE_FORMAT: &str = "format";
/// The choice that types the name, or that puts the branch itself in the worktree.
pub(super) const NO_FORMAT: &str = "typed";

/// A choice of a pick.
pub(super) fn choice(identifier: &str, label: &str, description: Option<String>) -> Choice {
    Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: description.map(Text::from),
    }
}

/// The format of `branches.format`, when it is set.
pub(super) fn format_of(context: &Context) -> Option<&str> {
    let format = context.settings.branch_format.trim();
    (!format.is_empty()).then_some(format)
}

/// The name the format makes, through `branch.name-from-format`: it asks each `${input:…}` in turn, with the values
/// used last, and what `${base}` is at a commit; `${base}` known is a branch, a remote branch whose remote is left out,
/// or a short hash. `title` is the title of the questions.
///
/// # Errors
///
/// The question or the read that comes first, or the refusal of a format that cannot make a name.
pub(super) fn formatted(
    context: &Context,
    answers: &Answers,
    (format, base, title): (&str, BaseOf, &str),
) -> Result<FormatName, Box<Outcome>> {
    let state = &context.repository;
    let asking = FormatAsking {
        format,
        inputs: &context.settings.branch_inputs,
        used: &context.settings.branch_values,
        base,
        values: FormatValues {
            remotes: state.remotes.clone(),
            repository: repository_name(state),
            date: day_of(context.now, context.settings.utc_offset),
            ..FormatValues::default()
        },
        title,
    };
    name_from_format(&asking, answers)
}

/// **Use the format** or the other way, the format first and the default.
pub(super) fn way_question(key: AnswerKey, format: &str, other: (&str, Option<String>)) -> Question {
    let (label, description) = other;
    Question {
        key,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(USE_FORMAT, "Use the format", Some(format.to_owned())),
                choice(NO_FORMAT, label, description),
            ],
        },
        title: Text::from("Create worktree"),
        detail: None,
        default: Some(Answer::Choice(USE_FORMAT.to_owned())),
    }
}

/// The rule of a name inside the folder of the worktrees: a new branch's when `is_branch`, a folder's only otherwise.
pub(super) fn rule(context: &Context, is_branch: bool, empty_name: Option<String>) -> WorktreeNameRule {
    WorktreeNameRule {
        branches: is_branch.then(|| {
            context
                .repository
                .branches
                .iter()
                .map(|branch| branch.name.clone())
                .collect()
        }),
        empty_name,
        base: FolderBase::Worktrees(folder_of_worktrees(context)),
        taken_folders: taken_folders(context),
    }
}

/// A box for a name, checked while it is typed and saying where the folder goes.
pub(super) fn name_question(
    key: AnswerKey,
    (title, detail): (&str, &str),
    rule: WorktreeNameRule,
    default: Option<String>,
) -> Question {
    Question {
        key,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("feature/my-change")),
            validation: Validation::WorktreeName(Box::new(rule)),
        },
        title: Text::from(title),
        detail: Some(Text::from(detail)),
        default: default.map(Answer::Text),
    }
}
