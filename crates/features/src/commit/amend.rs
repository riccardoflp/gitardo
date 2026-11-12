//! `commit.amend`: replaces the last commit with one that has the staged changes too, and the new message if one is
//! written.

use model::contract::{
    Answers, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text, Undo, Validation,
};

use super::create::MESSAGE;

/// Amends the last commit with what is staged, nothing needing to be: `git commit --amend -m <message>`, the message
/// without the spaces and blank lines around it; with the message box empty, `git commit --amend --no-edit`, which
/// keeps the message the commit has.
///
/// Feature: commit.amend
#[must_use]
pub fn amend(context: &Context, answers: &Answers) -> Outcome {
    if !context.repository.has_commit() {
        return Outcome::Refuse(Refusal::saying("There is no commit to amend yet."));
    }
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Ask(message_question());
    };
    let message = message.trim();
    let folder = context.repository.folder.clone();
    let (step, title) = if message.is_empty() {
        (
            Step::git(folder, ["commit", "--amend", "--no-edit"]),
            String::from("Amend the last commit"),
        )
    } else {
        let title_line = message.lines().next().unwrap_or(message);
        (
            Step::git(folder, ["commit", "--amend", "-m", message]),
            format!("Amend the last commit as \"{title_line}\""),
        )
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![step],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![FollowUp::EmptyCommitMessage],
        on_failure: Vec::new(),
    })
}

/// The message of the commit box, which may be left empty to keep the one the commit has.
fn message_question() -> Question {
    Question {
        key: MESSAGE,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from("Message"),
        detail: Some(Text::from("Leave the message empty to keep it.")),
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::amend;
    use crate::commit::create::MESSAGE;
    use crate::testing::{context, lines, modified, plan, state};
    use model::contract::{Answer, Answers, FollowUp, Outcome, Target};

    fn with_message(message: &str) -> Answers {
        Answers::new().with(MESSAGE, Answer::Text(message.to_owned()))
    }

    #[test]
    fn commit_amend__with_a_message_and_nothing_staged_replaces_the_message() {
        let plan = plan(amend(&context(state(), Target::Head), &with_message("Better words")));
        assert_eq!(lines(&plan), ["git commit --amend -m \"Better words\""]);
        assert_eq!(plan.after, [FollowUp::EmptyCommitMessage]);
    }

    #[test]
    fn commit_amend__with_an_empty_message_keeps_the_old_one() {
        let mut state = state();
        state.working_tree.staged.push(modified("b.txt"));
        let plan = plan(amend(&context(state, Target::Head), &with_message("  \n")));
        assert_eq!(lines(&plan), ["git commit --amend --no-edit"]);
    }

    #[test]
    fn commit_amend__in_a_repository_with_no_commit_cannot_run() {
        let mut state = state();
        state.head.commit = None;
        assert!(matches!(
            amend(&context(state, Target::Head), &with_message("Words")),
            Outcome::Refuse(_)
        ));
    }
}
