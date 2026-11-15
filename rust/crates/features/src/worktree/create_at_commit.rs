//! `worktree.create-at-commit`: makes a worktree at a commit of the timeline, on a new branch made there, or on a
//! detached `HEAD` when no name is given.

use model::CommitId;
use model::branch::{FormatName, base_of};
use model::contract::{Answers, Context, FollowUp, Outcome, Refusal, Target};
use model::worktree::folder_in;

use super::add::{Addition, add, with_offers};
use super::create::{NAME, WAY};
use super::folder_of_worktrees;
use super::names::{USE_FORMAT, format_of, formatted, name_question, rule, way_question};

/// What `${base}` of the format stands for: a branch at the commit, the short hash, or a branch that has the commit.
pub use model::branch::BASE;

/// Makes a worktree at the commit invoked on: on a new branch, `git worktree add --no-track -b <name> <folder>
/// <commit>`, or, with the name left empty, on a detached `HEAD` in a folder named after the short hash,
/// `git worktree add --detach <folder> <commit>`. With `branches.format` set it asks whether to use it, then its inputs
/// and what `${base}` is when the format has it (`branch.name-from-format`): the branches at the commit first, then
/// the short hash, then the other branches that have the commit.
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
    let from_format = match format.filter(|_| uses_format) {
        Some(format) => {
            let title = format!("Create worktree at {}", commit.short());
            match formatted(
                context,
                answers,
                (format, base_of(&context.repository, &context.target), &title),
            ) {
                Ok(made) => Some(made),
                Err(outcome) => return *outcome,
            }
        }
        None => None,
    };
    let Some(name) = answers.text(NAME) else {
        let detail = if from_format.is_some() {
            "Name of the new branch, and of the folder of the worktree"
        } else {
            "Name of a new branch, and of the folder of the worktree. Leave empty for a detached HEAD."
        };
        return Outcome::Ask(name_question(
            NAME,
            ("Create worktree", detail),
            rule(context, true, Some(commit.short().to_owned())),
            from_format.map(|made| made.name),
        ));
    };
    remembering(made(context, answers, (commit, name.trim())), from_format)
}

/// The plan, ending with the inputs given to the format kept for the next time.
fn remembering(outcome: Outcome, from_format: Option<FormatName>) -> Outcome {
    match (outcome, from_format) {
        (Outcome::Run(mut plan), Some(made)) => {
            plan.after.push(FollowUp::RememberInputs(made.inputs));
            Outcome::Run(plan)
        }
        (outcome, _) => outcome,
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
    use model::branch::{BranchInput, CONTAINING, INPUTS, InputKind};
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query, QuestionKind, Target};

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
    fn worktree_create_at_commit__the_picker_of_base_lists_the_branch_at_the_commit_first_then_those_that_have_it() {
        let mut context = at('a');
        context.settings.branch_format = String::from("${base}-spike");
        let answers = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
        let Outcome::Read(key, Query::Containing { .. }) = create_at_commit(&context, &answers) else {
            panic!("expected the read of the branches that have the commit");
        };
        assert_eq!(key, CONTAINING);
        let read = answers.with(
            CONTAINING,
            Answer::Names(vec![String::from("refs/heads/main"), String::from("refs/heads/later")]),
        );
        let base = question(create_at_commit(&context, &read));
        let QuestionKind::Pick { choices } = &base.kind else {
            panic!()
        };
        let identifiers: Vec<&str> = choices.iter().map(|choice| choice.identifier.as_str()).collect();
        assert_eq!(identifiers, ["main", commit('a').short(), "later"]);
        let picked = read.with(BASE, Answer::Choice(String::from("later")));
        let name = question(create_at_commit(&context, &picked));
        assert_eq!(name.default, Some(Answer::Text(String::from("later-spike"))));
    }

    #[test]
    fn worktree_create_at_commit__a_format_with_inputs_asks_them_and_keeps_them_for_the_next_time() {
        let mut context = at('c');
        context.settings.branch_format = String::from("PRJ-${input:ticket}");
        context.settings.branch_inputs = vec![ticket()];
        let answers = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
        let asked = question(create_at_commit(&context, &answers));
        assert_eq!(Some(&asked.key), INPUTS.first());
        let Some(&input) = INPUTS.first() else {
            panic!("the key of the first input");
        };
        let typed = answers.with(input, Answer::Text(String::from("42")));
        let name = question(create_at_commit(&context, &typed));
        assert_eq!(name.default, Some(Answer::Text(String::from("PRJ-42"))));
        let ran = plan(create_at_commit(
            &context,
            &not_there()
                .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
                .with(input, Answer::Text(String::from("42")))
                .with(NAME, Answer::Text(String::from("PRJ-42"))),
        ));
        let remembered = ran.after.iter().any(|follow_up| {
            matches!(follow_up, FollowUp::RememberInputs(values) if values.get("ticket").map(String::as_str) == Some("42"))
        });
        assert!(remembered, "{:?}", ran.after);
    }

    /// The input `${input:ticket}` of a format, typed.
    fn ticket() -> BranchInput {
        BranchInput {
            id: String::from("ticket"),
            kind: InputKind::PromptString,
            description: Some(String::from("Ticket")),
            default: None,
            pattern: None,
            options: Vec::new(),
            allow_custom: false,
        }
    }
}
