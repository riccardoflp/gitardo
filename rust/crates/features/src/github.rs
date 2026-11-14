//! The pull requests of GitHub: loading them for the first time, with the token that reads them
//! (`docs/features/15-github.md`).

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Text,
    Undo, Validation,
};
use model::github::{GitHubRepo, HelperAsked, Token, detect_remote};

/// The remotes, read to find the repository on GitHub.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// The token found, in the credential store or by git's credential helper.
pub const FOUND: AnswerKey = AnswerKey("found-token");

/// The token typed, when none was found.
pub const TOKEN: AnswerKey = AnswerKey("token");

/// Finds a token for GitHub, asking for one only when neither the credential store nor git's credential helper has
/// one, keeps it, and reads the pull requests at once.
///
/// Feature: github.load-pull-requests
#[must_use]
pub fn load_pull_requests(_context: &Context, answers: &Answers) -> Outcome {
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let Some(repository) = detect_remote(remotes) else {
        return Outcome::Refuse(Refusal::saying(
            "The main remote of this repository is not on github.com.",
        ));
    };
    let Some(Answer::Token(found)) = answers.get(FOUND) else {
        let query = Query::GitHubToken {
            helper: HelperAsked::Allowed,
        };
        return Outcome::Read(FOUND, query);
    };
    let found = found.clone();
    let token = match (found, answers.text(TOKEN)) {
        (Some(token), _) => token,
        (None, Some(typed)) => Token(typed.trim().to_owned()),
        (None, None) => return Outcome::Ask(token_question(&repository)),
    };
    Outcome::Run(Plan {
        title: Text::from("Load pull requests from GitHub"),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing to undo: it changes nothing in the repository.")),
        after: vec![FollowUp::KeepGitHubToken(token), FollowUp::ReadPullRequests],
        on_failure: Vec::new(),
    })
}

/// The question that asks for a token, in a box that hides what is typed.
fn token_question(repository: &GitHubRepo) -> Question {
    Question {
        key: TOKEN,
        kind: QuestionKind::Secret {
            validation: Validation::Required(Text::from("Type a token")),
        },
        title: Text::from("GitHub token"),
        detail: Some(Text::from(format!(
            "A token that can read the pull requests of {}/{}",
            repository.owner, repository.repo
        ))),
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
