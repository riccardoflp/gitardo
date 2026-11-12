//! The page of a commit (`docs/features/04-details-and-commit-page.md`): `commit-page.open`, which opens a commit as a
//! page, asking for it from Quick search.

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Target, Text,
    Undo, Validation,
};

/// The key of what is typed in **Open commit**.
pub const REVISION: AnswerKey = AnswerKey("revision");
/// The key of the commit git says it stands for.
pub const COMMIT: AnswerKey = AnswerKey("commit");

/// What is said of a name git does not know as a commit.
const UNKNOWN: &str = "Git does not know a commit by this name in this repository.";

/// Opens a commit as a page: the commit it was invoked on, or, from Quick search, the commit git says the name typed
/// in **Open commit** stands for; a name git does not know is refused with the words the box says while it is typed.
/// Nothing changes in the repository, so the plan has no step and nothing to undo.
///
/// Feature: commit-page.open
#[must_use]
pub fn open(context: &Context, answers: &Answers) -> Outcome {
    if let Target::Commit(commit) = &context.target {
        return opened(commit);
    }
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
    match commit {
        Some(commit) => opened(commit),
        None => Outcome::Refuse(Refusal::saying(UNKNOWN)),
    }
}

/// The plan that opens the page of a commit, or shows the one it has.
fn opened(commit: &CommitId) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from(format!("Open commit {} as a page", commit.short())),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Opening a page changes nothing to undo.")),
        after: vec![FollowUp::OpenCommitPage(commit.clone())],
        on_failure: Vec::new(),
    })
}

/// **Open commit**, in Gitardo's input box, with no default. The window checks each value typed with
/// `search.read-revision` and says [`UNKNOWN`] under the box while git knows no commit by it.
fn question() -> Question {
    Question {
        key: REVISION,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Anything,
        },
        title: Text::from("Open commit"),
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

    use super::{COMMIT, REVISION, open};
    use crate::testing::{commit, context, plan, question, state};

    #[test]
    fn commit_page_open__on_a_commit_opens_its_page_with_no_question_and_no_step() {
        let context = context(state(), Target::Commit(commit('b')));
        let plan = plan(open(&context, &Answers::new()));
        assert_eq!(plan.steps.len(), 0);
        assert_eq!(plan.after, [FollowUp::OpenCommitPage(commit('b'))]);
    }

    #[test]
    fn commit_page_open__from_quick_search_asks_reads_and_opens_the_commit_a_name_stands_for() {
        let context = context(state(), Target::Head);
        let asked = question(open(&context, &Answers::new()));
        assert_eq!(asked.title.as_str(), "Open commit");
        assert_eq!(
            asked.detail.map(|detail| detail.as_str().to_owned()).as_deref(),
            Some("A commit hash, or anything else git understands: HEAD~3, v1.0^, main@{yesterday}")
        );
        let typed = Answers::new().with(REVISION, Answer::Text(String::from(" HEAD~3 ")));
        assert_eq!(
            open(&context, &typed),
            Outcome::Read(
                COMMIT,
                Query::Revision {
                    revision: String::from("HEAD~3")
                }
            )
        );
        let read = typed.with(COMMIT, Answer::Commit(Some(commit('c'))));
        assert_eq!(
            plan(open(&context, &read)).after,
            [FollowUp::OpenCommitPage(commit('c'))]
        );
    }

    #[test]
    fn commit_page_open__refuses_a_name_git_does_not_know() {
        let context = context(state(), Target::Head);
        let answers = Answers::new()
            .with(REVISION, Answer::Text(String::from("nope")))
            .with(COMMIT, Answer::Commit(None));
        let Outcome::Refuse(refusal) = open(&context, &answers) else {
            panic!("a name git does not know is refused");
        };
        assert_eq!(
            refusal.message.as_str(),
            "Git does not know a commit by this name in this repository."
        );
    }
}
