//! `worktree.create`: makes a new worktree on a new branch, from a branch the user picks, in the folder of the
//! worktrees.

use model::branch::{BaseOf, RefKind, RefNames, ref_argument};
use model::contract::{Answer, AnswerKey, Answers, Context, FollowUp, Outcome, Question, QuestionKind, Text};
use model::worktree::folder_in;

use super::add::{Addition, add, with_offers};
use super::folder_of_worktrees;
use super::names::{USE_FORMAT, choice, format_of, formatted, name_question, rule, way_question};

/// Whether the name is made with `branches.format`, asked only when it is set.
pub const WAY: AnswerKey = AnswerKey("way");
/// What the new branch starts from: a local branch, a remote branch, or `HEAD`.
pub const START: AnswerKey = AnswerKey("start");
/// The name of the new branch, and of the folder of the worktree.
pub const NAME: AnswerKey = AnswerKey("name");

/// The start that is `HEAD` when no branch is checked out.
const DETACHED: &str = "HEAD";

/// Makes a worktree on a new branch: asks, when `branches.format` is set, whether to use it; then what it starts from,
/// the branch checked out first; then, with the format, its inputs (`branch.name-from-format`), `${base}` being the
/// branch it starts from or the short hash of a detached `HEAD`; then the name, made by the format or typed, refused while typed when git would not
/// take it, when it is a branch already or beside one, or when its folder is a worktree's. Then
/// `git worktree add --no-track -b <name> <folder> <start>`, so that the new branch has no upstream, followed by what
/// new worktrees get.
///
/// Feature: worktree.create
#[must_use]
pub fn create(context: &Context, answers: &Answers) -> Outcome {
    let format = format_of(context);
    let uses_format = match (format, answers.choice(WAY)) {
        (None, _) => false,
        (Some(format), None) => {
            return Outcome::Ask(way_question(WAY, format, ("Type a branch name", None)));
        }
        (Some(_), Some(way)) => way == USE_FORMAT,
    };
    let Some(start) = answers.choice(START) else {
        return Outcome::Ask(start_question(context));
    };
    let from_format = match format.filter(|_| uses_format) {
        Some(format) => match formatted(context, answers, (format, base_of_start(context, start), TITLE)) {
            Ok(made) => Some(made),
            Err(outcome) => return *outcome,
        },
        None => None,
    };
    let Some(name) = answers.text(NAME) else {
        let detail = "Name of the new branch, and of the folder of the worktree";
        return Outcome::Ask(name_question(
            NAME,
            (TITLE, detail),
            rule(context, true, None),
            from_format.map(|made| made.name),
        ));
    };
    let name = name.trim();
    let folder = folder_in(&folder_of_worktrees(context), name);
    let start = start_argument(context, start);
    let addition = Addition {
        title: format!("Create worktree {name} from {start}"),
        arguments: vec![
            String::from("--no-track"),
            String::from("-b"),
            name.to_owned(),
            folder.to_string_lossy().into_owned(),
            start.clone(),
        ],
        folder: folder.clone(),
        start,
    };
    match add(context, answers, addition) {
        Ok(made) => {
            let mut plan = with_offers(context, made, &folder);
            plan.after
                .extend(from_format.map(|made| FollowUp::RememberInputs(made.inputs)));
            Outcome::Run(plan)
        }
        Err(outcome) => *outcome,
    }
}

/// The title of the questions.
const TITLE: &str = "Create worktree";

/// What `${base}` is for the start picked: the branch, or a remote branch, by its name; the short hash of `HEAD` when
/// it is detached.
fn base_of_start(context: &Context, start: &str) -> BaseOf {
    match (start, &context.repository.head.commit) {
        (DETACHED, Some(commit)) => BaseOf::Known(commit.short().to_owned()),
        _ => BaseOf::Known(start.to_owned()),
    }
}

/// What the new branch can start from: the branch checked out, or `HEAD` when none is, first; the other local
/// branches; then the remote branches.
fn start_question(context: &Context) -> Question {
    let state = &context.repository;
    let checked_out = state.branches.iter().find(|branch| branch.is_checked_out);
    let first = match checked_out {
        Some(branch) => choice(&branch.name, &branch.name, Some(String::from("Checked out"))),
        None => choice(DETACHED, DETACHED, Some(String::from("Detached"))),
    };
    let mut choices = vec![first];
    choices.extend(
        state
            .branches
            .iter()
            .filter(|branch| !branch.is_checked_out)
            .map(|branch| choice(&branch.name, &branch.name, None)),
    );
    choices.extend(
        state
            .remote_branches
            .iter()
            .map(|branch| choice(&branch.name, &branch.name, Some(String::from("Remote")))),
    );
    let default = choices.first().map(|first| Answer::Choice(first.identifier.clone()));
    Question {
        key: START,
        kind: QuestionKind::Pick { choices },
        title: Text::from("Create worktree"),
        detail: Some(Text::from("What the new branch starts from")),
        default,
    }
}

/// The start as git is given it: a branch or a remote branch by its short name, in full when a namesake would make it
/// ambiguous; `HEAD` as it is.
fn start_argument(context: &Context, start: &str) -> String {
    if start == DETACHED {
        return start.to_owned();
    }
    let names = RefNames::of(&context.repository);
    let kind = if context.repository.branches.iter().any(|branch| branch.name == start) {
        RefKind::Branch
    } else {
        RefKind::RemoteBranch
    };
    ref_argument(kind, start, &names)
}

/// The choice that types the name, for the tests.
#[cfg(test)]
pub(super) const TYPED: &str = super::names::NO_FORMAT;

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
