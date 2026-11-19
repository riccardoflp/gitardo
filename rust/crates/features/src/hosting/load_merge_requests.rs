//! `hosting.load-merge-requests`: the merge requests of a GitLab read for the first time, asking for a token when none
//! is found.

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Text,
    Undo, Validation,
};
use model::github::{HelperAsked, Token};
use model::hosting::{HostingService, detect_remote};

/// The remotes, read to find the project on GitLab.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// The token found, in the credential store or by git's credential helper.
pub const FOUND: AnswerKey = AnswerKey("found-token");

/// The token typed, when none was found.
pub const TOKEN: AnswerKey = AnswerKey("token");

/// Finds a token for the GitLab of the main remote, asking for one only when neither the credential store nor git's
/// credential helper has one for its host name, keeps it, and reads the merge requests at once.
///
/// Feature: hosting.load-merge-requests
#[must_use]
pub fn load_merge_requests(context: &Context, answers: &Answers) -> Outcome {
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let Some(project) =
        detect_remote(remotes, &context.settings.hosting).filter(|project| project.service == HostingService::GitLab)
    else {
        return Outcome::Refuse(Refusal::saying(
            "The main remote of this repository is not on a GitLab.",
        ));
    };
    let Some(Answer::Token(found)) = answers.get(FOUND) else {
        let query = Query::HostingToken {
            host: project.host.clone(),
            helper: HelperAsked::Allowed,
        };
        return Outcome::Read(FOUND, query);
    };
    let token = match (found.clone(), answers.text(TOKEN)) {
        (Some(token), _) => token,
        (None, Some(typed)) => Token(typed.trim().to_owned()),
        (None, None) => return Outcome::Ask(token_question(&project.host)),
    };
    Outcome::Run(Plan {
        title: Text::from("Load merge requests from GitLab"),
        steps: Vec::new(),
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Nothing to undo: it changes nothing in the repository.")),
        after: vec![
            FollowUp::KeepHostingToken(project.host, token),
            FollowUp::ReadPullRequests,
        ],
        on_failure: Vec::new(),
    })
}

/// The question that asks for a token, in a box that hides what is typed.
fn token_question(host: &str) -> Question {
    Question {
        key: TOKEN,
        kind: QuestionKind::Secret {
            validation: Validation::Required(Text::from("Type a token")),
        },
        title: Text::from("GitLab token"),
        detail: Some(Text::from(format!(
            "A personal access token of {host} with the scope read_api"
        ))),
        default: None,
    }
}

#[cfg(test)]
#[path = "load_merge_requests_tests.rs"]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
