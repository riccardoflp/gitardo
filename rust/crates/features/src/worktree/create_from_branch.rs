//! `worktree.create-from-branch`: puts a local branch in a worktree of its own, or a new branch made from it with the
//! format of branch names.

use model::branch::other_worktree;
use model::contract::{AnswerKey, Answers, Context, Outcome, Refusal, Target};
use model::worktree::folder_in;

use super::add::{Addition, add, with_offers};
use super::create::{NAME, WAY};
use super::folder_of_worktrees;
use super::names::{USE_FORMAT, format_of, formatted, name_question, rule, way_question};

/// The name of the folder of the worktree, when the branch itself goes in it.
pub const FOLDER: AnswerKey = AnswerKey("folder");

/// Puts the branch invoked on in a worktree of its own: `git worktree add <folder> <branch>`, the folder named after
/// the branch unless another name is typed. With `branches.format` set, it asks first whether to use the format, which
/// makes a new branch from it with `--no-track -b`, or the branch itself. A branch checked out here or in another
/// worktree is not put in one more.
///
/// Feature: worktree.create-from-branch
#[must_use]
pub fn create_from_branch(context: &Context, answers: &Answers) -> Outcome {
    let Target::Branch(branch) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Choose a local branch to put in a worktree."));
    };
    let state = &context.repository;
    let is_checked_out = state
        .branches
        .iter()
        .any(|known| known.name == *branch && known.is_checked_out);
    if is_checked_out || other_worktree(state, branch).is_some() {
        return Outcome::Refuse(Refusal::saying(&format!(
            "`{branch}` is checked out already: git checks a branch out in one worktree at a time."
        )));
    }
    let format = format_of(context);
    let uses_format = match (format, answers.choice(WAY)) {
        (None, _) => false,
        (Some(format), None) => {
            let other = (
                format!("Use the branch `{branch}`"),
                Some(String::from("Check it out in the worktree")),
            );
            return Outcome::Ask(way_question(WAY, format, (&other.0, other.1)));
        }
        (Some(_), Some(way)) => way == USE_FORMAT,
    };
    match (format.filter(|_| uses_format), answers.text(NAME), answers.text(FOLDER)) {
        (Some(format), None, _) => Outcome::Ask(name_question(
            NAME,
            (
                "Create worktree",
                "Name of the new branch, and of the folder of the worktree",
            ),
            rule(context, true, None),
            Some(formatted(context, format, branch)),
        )),
        (Some(_), Some(name), _) => made(context, answers, (branch, Some(name.trim()), name.trim())),
        (None, _, None) => Outcome::Ask(name_question(
            FOLDER,
            ("Create worktree", "Name of the folder of the worktree"),
            rule(context, false, None),
            Some(branch.clone()),
        )),
        (None, _, Some(folder)) => made(context, answers, (branch, None, folder.trim())),
    }
}

/// The plan: the branch itself, or a new branch from it, in the folder of a name.
fn made(context: &Context, answers: &Answers, (branch, new_branch, name): (&str, Option<&str>, &str)) -> Outcome {
    let folder = folder_in(&folder_of_worktrees(context), name);
    let path = folder.to_string_lossy().into_owned();
    let (title, arguments) = match new_branch {
        Some(new) => (
            format!("Create worktree {new} from {branch}"),
            vec![
                String::from("--no-track"),
                String::from("-b"),
                new.to_owned(),
                path,
                branch.to_owned(),
            ],
        ),
        None => (format!("Put {branch} in a worktree"), vec![path, branch.to_owned()]),
    };
    let addition = Addition {
        title,
        arguments,
        folder: folder.clone(),
        start: branch.to_owned(),
    };
    match add(context, answers, addition) {
        Ok(made) => Outcome::Run(with_offers(context, made, &folder)),
        Err(outcome) => *outcome,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, Target};

    use super::{FOLDER, create_from_branch};
    use crate::testing::{lines, plan, question};
    use crate::worktree::add::not_there;
    use crate::worktree::create::{NAME, WAY};
    use crate::worktree::names::{NO_FORMAT, USE_FORMAT};
    use crate::worktree::testing::with_worktrees;

    fn on_release() -> model::contract::Context {
        with_worktrees(Target::Branch(String::from("release")))
    }

    #[test]
    fn worktree_create_from_branch__with_no_format_it_proposes_the_branch_name_and_adds_the_branch() {
        let context = on_release();
        let asked = question(create_from_branch(&context, &Answers::new()));
        assert_eq!(asked.key, FOLDER);
        assert_eq!(asked.default, Some(Answer::Text(String::from("release"))));
        let answers = not_there().with(FOLDER, Answer::Text(String::from("release")));
        let plan = plan(create_from_branch(&context, &answers));
        assert_eq!(lines(&plan)[0], "git worktree add /code/app.worktrees/release release");
    }

    #[test]
    fn is_refused_on_the_branch_checked_out_and_on_one_another_worktree_has() {
        for branch in ["main", "feature/login"] {
            let context = with_worktrees(Target::Branch(branch.to_owned()));
            assert!(matches!(
                create_from_branch(&context, &Answers::new()),
                model::contract::Outcome::Refuse(_)
            ));
        }
    }

    #[test]
    fn worktree_create_from_branch__with_a_format_it_puts_the_branch_or_a_new_one_made_from_it() {
        let mut context = on_release();
        context.settings.branch_format = String::from("${base}-spike");
        let way = question(create_from_branch(&context, &Answers::new()));
        let labels: Vec<&str> = match &way.kind {
            model::contract::QuestionKind::Pick { choices } => {
                choices.iter().map(|choice| choice.label.as_str()).collect()
            }
            _ => panic!(),
        };
        assert_eq!(labels, ["Use the format", "Use the branch `release`"]);
        let itself = not_there()
            .with(WAY, Answer::Choice(NO_FORMAT.to_owned()))
            .with(FOLDER, Answer::Text(String::from("release")));
        assert_eq!(
            lines(&plan(create_from_branch(&context, &itself)))[0],
            "git worktree add /code/app.worktrees/release release"
        );
        let formatted = Answers::new().with(WAY, Answer::Choice(USE_FORMAT.to_owned()));
        let asked = question(create_from_branch(&context, &formatted));
        assert_eq!(asked.default, Some(Answer::Text(String::from("release-spike"))));
        let new = not_there()
            .with(WAY, Answer::Choice(USE_FORMAT.to_owned()))
            .with(NAME, Answer::Text(String::from("release-spike")));
        assert_eq!(
            lines(&plan(create_from_branch(&context, &new)))[0],
            "git worktree add --no-track -b release-spike /code/app.worktrees/release-spike release"
        );
    }

    #[test]
    fn worktree_create_from_branch__a_folder_name_changed_puts_the_worktree_there_on_the_branch() {
        let context = on_release();
        let answers = not_there().with(FOLDER, Answer::Text(String::from("login")));
        let plan = plan(create_from_branch(&context, &answers));
        assert_eq!(lines(&plan)[0], "git worktree add /code/app.worktrees/login release");
    }
}
