//! `worktree.create-at-commit`: makes a worktree at a commit of the timeline, on a new branch made there, or on a
//! detached `HEAD` when no name is given.

use model::CommitId;
use model::branch::local_name;
use model::contract::{Answer, AnswerKey, Answers, Context, Outcome, Question, QuestionKind, Refusal, Target, Text};
use model::worktree::folder_in;

use super::add::{Addition, add, with_offers};
use super::create::{NAME, WAY};
use super::folder_of_worktrees;
use super::names::{USE_FORMAT, choice, format_of, formatted, name_question, rule, way_question};

/// What `${base}` of the format stands for: a branch at the commit, or its short hash.
pub const BASE: AnswerKey = AnswerKey("base");

/// Makes a worktree at the commit invoked on: on a new branch, `git worktree add --no-track -b <name> <folder>
/// <commit>`, or, with the name left empty, on a detached `HEAD` in a folder named after the short hash,
/// `git worktree add --detach <folder> <commit>`. With `branches.format` set it asks whether to use it, and what
/// `${base}` is when the format has it: the branches at the commit first, then the short hash.
///
/// Feature: worktree.create-at-commit
#[must_use]
pub fn create_at_commit(context: &Context, answers: &Answers) -> Outcome {
    let Target::Commit(commit) = &context.target else {
        return Outcome::Refuse(Refusal::saying(
            "Choose a commit of the timeline to make a worktree at.",
        ));
    };
    let format = format_of(context);
    let uses_format = match (format, answers.choice(WAY)) {
        (None, _) => false,
        (Some(format), None) => return Outcome::Ask(way_question(WAY, format, ("Type a branch name", None))),
        (Some(_), Some(way)) => way == USE_FORMAT,
    };
    let Some(name) = answers.text(NAME) else {
        return match format.filter(|_| uses_format) {
            Some(format) => format_name(context, answers, (format, commit)),
            None => Outcome::Ask(name_question(
                NAME,
                (
                    "Create worktree",
                    "Name of a new branch, and of the folder of the worktree. Leave empty for a detached HEAD.",
                ),
                rule(context, true, Some(commit.short().to_owned())),
                None,
            )),
        };
    };
    made(context, answers, (commit, name.trim()))
}

/// What `${base}` is, when the format has it, then the name the format makes, which can be changed.
fn format_name(context: &Context, answers: &Answers, (format, commit): (&str, &CommitId)) -> Outcome {
    let base = if format.contains("${base}") {
        match answers.choice(BASE) {
            Some(base) => base.to_owned(),
            None => return Outcome::Ask(base_question(context, commit)),
        }
    } else {
        commit.short().to_owned()
    };
    Outcome::Ask(name_question(
        NAME,
        (
            "Create worktree",
            "Name of the new branch, and of the folder of the worktree",
        ),
        rule(context, true, Some(commit.short().to_owned())),
        Some(formatted(context, format, &base)),
    ))
}

/// What `${base}` is: the branches at the commit, local then remote by their local names, then the short hash.
fn base_question(context: &Context, commit: &CommitId) -> Question {
    let state = &context.repository;
    let mut names: Vec<String> = state
        .branches
        .iter()
        .filter(|branch| branch.commit == *commit)
        .map(|branch| branch.name.clone())
        .collect();
    for remote in state.remote_branches.iter().filter(|branch| branch.commit == *commit) {
        let name = local_name(&remote.name, &state.remotes).to_owned();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let mut choices: Vec<_> = names
        .iter()
        .map(|name| choice(name, name, Some(String::from("Branch at this commit"))))
        .collect();
    choices.push(choice(
        commit.short(),
        commit.short(),
        Some(String::from("This commit")),
    ));
    let default = choices.first().map(|first| Answer::Choice(first.identifier.clone()));
    Question {
        key: BASE,
        kind: QuestionKind::Pick { choices },
        title: Text::from(format!("Create worktree at {}: what ${{base}} is", commit.short())),
        detail: None,
        default,
    }
}

/// The plan: a new branch at the commit, or a detached `HEAD` when the name is empty.
fn made(context: &Context, answers: &Answers, (commit, name): (&CommitId, &str)) -> Outcome {
    let short = commit.short();
    let folder = folder_in(
        &folder_of_worktrees(context),
        if name.is_empty() { short } else { name },
    );
    let path = folder.to_string_lossy().into_owned();
    let (title, mut arguments) = if name.is_empty() {
        (
            format!("Create a worktree at {short}"),
            vec![String::from("--detach"), path],
        )
    } else {
        (
            format!("Create worktree {name} at {short}"),
            vec![String::from("--no-track"), String::from("-b"), name.to_owned(), path],
        )
    };
    arguments.push(commit.to_string());
    let addition = Addition {
        title,
        arguments,
        folder: folder.clone(),
        start: commit.to_string(),
    };
    match add(context, answers, addition) {
        Ok(made) => Outcome::Run(with_offers(context, made, &folder)),
        Err(outcome) => *outcome,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, QuestionKind, Target};

    use super::{BASE, create_at_commit};
    use crate::testing::{commit, lines, plan, question};
    use crate::worktree::add::not_there;
    use crate::worktree::create::{NAME, WAY};
    use crate::worktree::names::USE_FORMAT;
    use crate::worktree::testing::with_worktrees;

    fn at(digit: char) -> model::contract::Context {
        with_worktrees(Target::Commit(commit(digit)))
    }

    #[test]
    fn worktree_create_at_commit__with_the_name_left_empty_it_adds_a_detached_worktree_named_after_the_short_hash() {
        let context = at('c');
        let short = commit('c').short().to_owned();
        let asked = question(create_at_commit(&context, &Answers::new()));
        let QuestionKind::Text { validation, .. } = &asked.kind else {
            panic!()
        };
        assert_eq!(validation.problem(""), None, "an empty name is taken");
        assert_eq!(
            validation.hint("").unwrap().as_str(),
            format!("In `/code/app.worktrees/{short}`")
        );
        let answers = not_there().with(NAME, Answer::Text(String::new()));
        assert_eq!(
            lines(&plan(create_at_commit(&context, &answers))),
            [format!(
                "git worktree add --detach /code/app.worktrees/{short} {}",
                commit('c')
            )]
        );
    }

    #[test]
    fn worktree_create_at_commit__with_a_name_it_adds_a_new_branch_there() {
        let answers = not_there().with(NAME, Answer::Text(String::from("try/idea")));
        assert_eq!(
            lines(&plan(create_at_commit(&at('c'), &answers))),
            [format!(
                "git worktree add --no-track -b try/idea /code/app.worktrees/try/idea {}",
                commit('c')
            )]
        );
    }

    #[test]
    fn worktree_create_at_commit__the_picker_of_base_lists_the_branch_at_the_commit_first() {
        let mut context = at('a');
        context.settings.branch_format = String::from("${base}-spike");
        let answers = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
        let base = question(create_at_commit(&context, &answers));
        let QuestionKind::Pick { choices } = &base.kind else {
            panic!()
        };
        assert_eq!(choices[0].identifier, "main");
        assert_eq!(choices.last().unwrap().identifier, commit('a').short());
        let picked = answers.with(BASE, Answer::Choice(String::from("main")));
        let name = question(create_at_commit(&context, &picked));
        assert_eq!(name.default, Some(Answer::Text(String::from("main-spike"))));
    }
}
