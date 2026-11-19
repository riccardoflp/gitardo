//! `compare.open-on-service`: the same comparison on the hosting service of the main remote, when it is not GitHub,
//! each side by the name it has there.

use model::compare::{CompareMode, Side};
use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Query, Refusal, Target};
use model::hosting::{HostingPage, HostingService, NamedSide, detect_remote, web_link};

use super::no_step;
use super::open_on_github::named_on_service;

/// The key of the remotes, read to find the hosting service.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// Opens the comparison of the page on the hosting service of the main remote in the browser of the system: GitLab,
/// Bitbucket, Azure DevOps, Gitea or Forgejo, the address `hosting.web-links` gives. On GitHub it is
/// `compare.open-on-github`. Nothing changes in the repository.
///
/// Feature: compare.open-on-service
#[must_use]
pub fn open_on_service(context: &Context, answers: &Answers) -> Outcome {
    let Target::Comparison(target) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Open a comparison first."));
    };
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let Some(repository) = detect_remote(remotes, &context.settings.hosting) else {
        return Outcome::Refuse(Refusal::saying(
            "The main remote of this repository is not on a hosting service Gitardo knows.",
        ));
    };
    let service = repository.service;
    if service == HostingService::GitHub {
        return super::open_on_github(context, answers);
    }
    let (Side::Commit(first), Side::Commit(second)) = (&target.first.side, &target.second.side) else {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} does not have the working tree: compare two commits that are pushed.",
            service.name()
        )));
    };
    if first == second {
        return Outcome::Refuse(Refusal::saying("The two sides are the same commit."));
    }
    if target.mode == CompareMode::Everything && service.compares_only_since_diverged() {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} compares only since the two diverged.",
            service.name()
        )));
    }
    let main = repository.remote.as_str();
    let (first_name, first_kind) = named_on_service(&context.repository, main, &target.first, first);
    let (second_name, second_kind) = named_on_service(&context.repository, main, &target.second, second);
    let url = web_link(
        &repository,
        &HostingPage::Compare {
            first: NamedSide {
                name: &first_name,
                kind: first_kind,
            },
            second: NamedSide {
                name: &second_name,
                kind: second_kind,
            },
            mode: target.mode,
        },
    );
    let title = format!(
        "Open {} with {} on {}",
        target.first.name,
        target.second.name,
        service.name()
    );
    Outcome::Run(no_step(title, vec![FollowUp::OpenUrl(url)]))
}

#[cfg(test)]
#[path = "open_on_service_tests.rs"]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
