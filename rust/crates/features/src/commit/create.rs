//! `commit.create`: makes a commit of what is staged, with the message of the commit box.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text, Undo,
    Validation,
};

/// The message of the commit box. The Changes pane gives it as it is; the command line asks for it.
pub const MESSAGE: AnswerKey = AnswerKey("message");

/// The confirmation to stage all changes when nothing is staged.
pub const STAGE_ALL: AnswerKey = AnswerKey("stage-all");

/// Commits what is staged with the message of the commit box; when nothing is staged and something else is changed,
/// asks first to stage it all.
///
/// Feature: commit.create
#[must_use]
pub fn create(context: &Context, answers: &Answers) -> Outcome {
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Ask(message_question());
    };
    let message = message.trim();
    if message.is_empty() {
        return Outcome::Refuse(Refusal::saying("Enter a commit message."));
    }
    let working_tree = &context.repository.working_tree;
    let stages_all = working_tree.staged.is_empty();
    if stages_all && working_tree.is_clean() {
        return Outcome::Refuse(Refusal::saying("There are no changes to commit."));
    }
    if stages_all && !answers.is_confirmed(STAGE_ALL) {
        return Outcome::Ask(stage_all_question());
    }
    let folder = context.repository.folder.clone();
    let mut steps = Vec::new();
    if stages_all {
        steps.push(Step::git(folder.clone(), ["add", "--all"]));
    }
    let options = model::commit::option_arguments(&super::options_of(answers), &context.repository.signing, message);
    let mut arguments: Vec<std::ffi::OsString> = vec!["commit".into()];
    arguments.extend(options);
    arguments.extend(["-m".into(), message.into()]);
    steps.push(Step::git(folder, arguments));
    let title_line = message.lines().next().unwrap_or(message);
    Outcome::Run(Plan {
        title: Text::from(format!("Commit \"{title_line}\"")),
        steps,
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![FollowUp::EmptyCommitMessage],
        on_failure: Vec::new(),
    })
}

fn message_question() -> Question {
    Question {
        key: MESSAGE,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from("Message"),
        detail: None,
        default: None,
    }
}

fn stage_all_question() -> Question {
    Question {
        key: STAGE_ALL,
        kind: QuestionKind::Confirm {
            accept: Text::from("Stage all and commit"),
        },
        title: Text::from("There are no staged changes. Stage all changes and commit them?"),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MESSAGE, STAGE_ALL, create};
    use crate::testing::{context, lines, modified, plan, question, state};
    use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target};

    fn with_message(message: &str) -> Answers {
        Answers::new().with(MESSAGE, Answer::Text(message.to_owned()))
    }

    #[test]
    fn commit_create__commits_what_is_staged_with_the_message() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        let plan = plan(create(&context(state, Target::Head), &with_message("Fix the parser")));
        assert_eq!(lines(&plan), ["git commit -m \"Fix the parser\""]);
    }

    #[test]
    fn commit_create__asks_to_stage_all_when_nothing_is_staged() {
        let mut state = state();
        state.working_tree.unstaged.push(modified("a.txt"));
        let context = context(state, Target::Head);
        let asked = question(create(&context, &with_message("Fix the parser")));
        assert_eq!(
            asked.title.as_str(),
            "There are no staged changes. Stage all changes and commit them?"
        );
        assert!(
            matches!(asked.kind, QuestionKind::Confirm { ref accept } if accept.as_str() == "Stage all and commit")
        );
        let confirmed = with_message("Fix the parser").with(STAGE_ALL, Answer::Confirmed);
        assert_eq!(
            lines(&plan(create(&context, &confirmed))),
            ["git add --all", "git commit -m \"Fix the parser\""]
        );
    }

    #[test]
    fn commit_create__declining_runs_nothing() {
        // Declining is backing out of the confirmation: the engine calls the operation no more, so nothing runs. What
        // the operation says until it is confirmed is the question again, never a plan.
        let mut state = state();
        state.working_tree.untracked.push(String::from("new.txt"));
        assert!(matches!(
            create(&context(state, Target::Head), &with_message("Add")),
            Outcome::Ask(_)
        ));
    }

    #[test]
    fn commit_create__refuses_a_message_of_spaces_and_blank_lines() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        let Outcome::Refuse(refusal) = create(&context(state, Target::Head), &with_message("  \n  ")) else {
            panic!("an empty message is refused")
        };
        assert_eq!(refusal.message.as_str(), "Enter a commit message.");
    }

    #[test]
    fn commit_create__empties_the_message_box_after() {
        let mut state = state();
        state.working_tree.staged.push(modified("a.txt"));
        let plan = plan(create(
            &context(state, Target::Head),
            &with_message("  Fix the parser\n\n"),
        ));
        assert_eq!(plan.after, [FollowUp::EmptyCommitMessage]);
        assert_eq!(lines(&plan), ["git commit -m \"Fix the parser\""]);
    }

    #[test]
    fn commit_create__refuses_when_nothing_is_changed() {
        let Outcome::Refuse(refusal) = create(&context(state(), Target::Head), &with_message("Nothing")) else {
            panic!("nothing to commit is refused")
        };
        assert_eq!(refusal.message.as_str(), "There are no changes to commit.");
    }
}
