//! `search.go-to-commit`: asks for anything git takes for a commit, and opens the timeline at it.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Text, Undo,
    Validation,
};

/// The key of what is typed in **Go to commit**.
pub const REVISION: AnswerKey = AnswerKey("revision");
/// The key of the commit git says it stands for.
pub const COMMIT: AnswerKey = AnswerKey("commit");

/// What is said of a name git does not know as a commit.
const UNKNOWN: &str = "Git does not know a commit by this name in this repository.";

/// Asks **Go to commit** for a name of a commit, reads the commit git says it stands for, and selects it in the
/// timeline; a name git does not know is refused with the words the box says while it is typed. Nothing changes in the
/// repository, so the plan has no step and nothing to undo.
///
/// Feature: search.go-to-commit
#[must_use]
pub fn go_to_commit(_context: &Context, answers: &Answers) -> Outcome {
    let Some(revision) = answers.text(REVISION) else {
        return Outcome::Ask(question());
    };
    let Some(commit) = answers.commit(COMMIT) else {
        return Outcome::Read(
            COMMIT,
            Query::Revision {
                revision: revision.trim().to_owned(),
            },
        );
    };
    let Some(commit) = commit else {
        return Outcome::Refuse(Refusal::saying(UNKNOWN));
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Go to commit {}", commit.short())),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Going to a commit changes nothing to undo.")),
        after: vec![FollowUp::SelectCommit(commit.clone())],
        on_failure: Vec::new(),
    })
}

/// **Go to commit**, in Gitardo's input box, with no default. The window checks each value typed with
/// `search.read-revision` and says [`UNKNOWN`] under the box while git knows no commit by it.
fn question() -> Question {
    Question {
        key: REVISION,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from("Go to commit"),
        detail: Some(Text::from(
            "A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}",
        )),
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};

    use super::{COMMIT, REVISION, go_to_commit};
    use crate::testing::{commit, context, plan, question, state};

    #[test]
    fn search_go_to_commit__asks_reads_and_selects_the_commit_a_name_stands_for() {
        let context = context(state(), Target::Head);
        let asked = question(go_to_commit(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Go to commit");
        assert_eq!(asked.default, None);
        let typed = Answers::new().with(REVISION, Answer::Text(String::from(" main@{yesterday} ")));
        assert_eq!(
            go_to_commit(&context, &typed),
            Outcome::Read(
                COMMIT,
                Query::Revision {
                    revision: String::from("main@{yesterday}")
                }
            )
        );
        let read = typed.with(COMMIT, Answer::Commit(Some(commit('b'))));
        let plan = plan(go_to_commit(&context, &read));
        assert_eq!(plan.steps.len(), 0);
        assert_eq!(plan.after, [FollowUp::SelectCommit(commit('b'))]);
    }

    #[test]
    fn search_go_to_commit__refuses_a_name_git_does_not_know() {
        let context = context(state(), Target::Head);
        let answers = Answers::new()
            .with(REVISION, Answer::Text(String::from("nope")))
            .with(COMMIT, Answer::Commit(None));
        let Outcome::Refuse(refusal) = go_to_commit(&context, &answers) else {
            panic!("a name git does not know is refused");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Git does not know a commit by this name in this repository."
        );
    }
}
