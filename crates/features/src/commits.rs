//! What is copied of a commit: its hash (`commits.copy-hash`) and its message (`commits.copy-message`).

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Refusal, Target, Text, Undo,
};

/// The message of the commit, as git keeps it.
const MESSAGE: AnswerKey = AnswerKey("message");

/// The commit a copy is made of.
fn commit_of(context: &Context) -> Option<&CommitId> {
    match &context.target {
        Target::Commit(commit) => Some(commit),
        _ => None,
    }
}

/// The refusal when the copy is not made of a commit.
fn no_commit() -> Outcome {
    Outcome::Refuse(Refusal::saying("Pick a commit first."))
}

/// A plan that changes nothing and puts some text on the clipboard.
fn copy(title: String, text: String, said: &str) -> Plan {
    Plan {
        title: Text::from(title),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing changed.")),
        after: vec![FollowUp::CopyToClipboard(text), FollowUp::Say(Text::from(said))],
        on_failure: Vec::new(),
    }
}

/// Puts the full hash of the commit on the clipboard.
///
/// Feature: commits.copy-hash
#[must_use]
pub fn copy_hash(context: &Context, _answers: &Answers) -> Outcome {
    let Some(commit) = commit_of(context) else {
        return no_commit();
    };
    Outcome::Run(copy(
        format!("Copy the hash of {}", commit.short()),
        commit.as_str().to_owned(),
        "Copied the hash.",
    ))
}

/// Reads the whole message of the commit, then puts it on the clipboard as it was written.
///
/// Feature: commits.copy-message
#[must_use]
pub fn copy_message(context: &Context, answers: &Answers) -> Outcome {
    let Some(commit) = commit_of(context) else {
        return no_commit();
    };
    let Some(message) = answers.text(MESSAGE) else {
        return Outcome::Read(MESSAGE, Query::Message { commit: commit.clone() });
    };
    Outcome::Run(copy(
        format!("Copy the message of {}", commit.short()),
        message.to_owned(),
        "Copied the message.",
    ))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{MESSAGE, copy_hash, copy_message};
    use crate::testing::{commit, context, plan, state};
    use model::contract::{Answer, Answers, FollowUp, Outcome, Query, Target};

    fn copied(outcome: Outcome) -> Vec<String> {
        plan(outcome)
            .after
            .into_iter()
            .filter_map(|follow| match follow {
                FollowUp::CopyToClipboard(text) => Some(text),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn commits_copy_hash__the_clipboard_holds_the_full_hash() {
        let on_commit = context(state(), Target::Commit(commit('b')));
        assert_eq!(copied(copy_hash(&on_commit, &Answers::new())), ["b".repeat(40)]);
        let plan = crate::testing::plan(copy_hash(&on_commit, &Answers::new()));
        assert_eq!(plan.steps, []);
    }

    #[test]
    fn commits_copy_message__the_clipboard_holds_the_message_read_whole() {
        let on_commit = context(state(), Target::Commit(commit('b')));
        assert_eq!(
            copy_message(&on_commit, &Answers::new()),
            Outcome::Read(MESSAGE, Query::Message { commit: commit('b') })
        );
        let message = "Fix the login\n\nThe session expired too early.\n  Kept as written.";
        let mut answers = Answers::new();
        answers.insert(MESSAGE, Answer::Text(message.to_owned()));
        assert_eq!(copied(copy_message(&on_commit, &answers)), [message]);
    }

    #[test]
    fn commits_copy_message__a_long_line_is_copied_as_written_not_rewrapped() {
        let on_commit = context(state(), Target::Commit(commit('b')));
        let long = format!("Subject\n\n{}", "word ".repeat(40).trim_end());
        let mut answers = Answers::new();
        answers.insert(MESSAGE, Answer::Text(long.clone()));
        assert_eq!(copied(copy_message(&on_commit, &answers)), [long]);
    }
}
