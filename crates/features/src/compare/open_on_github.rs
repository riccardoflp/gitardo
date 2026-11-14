//! `compare.open-on-github`: the same comparison on github.com, each side by the name it has there.

use model::compare::{CompareSide, Side};
use model::contract::{AnswerKey, Answers, Context, FollowUp, Outcome, Query, Refusal, Target};
use model::github::{ComparedOnGitHub, comparison_url, detect_remote, main_remote};
use model::state::RepositoryState;

use super::no_step;

/// The key of the remotes, read to find the repository on GitHub.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// Opens the comparison of the page on github.com in the browser of the system, with `...` for **Since they
/// diverged** and `..` for **Everything that differs**. Nothing changes in the repository.
///
/// Feature: compare.open-on-github
#[must_use]
pub fn open_on_github(context: &Context, answers: &Answers) -> Outcome {
    let Target::Comparison(target) = &context.target else {
        return Outcome::Refuse(Refusal::saying("Open a comparison first."));
    };
    let (Side::Commit(first), Side::Commit(second)) = (&target.first.side, &target.second.side) else {
        return Outcome::Refuse(Refusal::saying(
            "GitHub does not have the working tree: compare two commits that are pushed.",
        ));
    };
    if first == second {
        return Outcome::Refuse(Refusal::saying("The two sides are the same commit."));
    }
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let (Some(repository), Some(main)) = (detect_remote(remotes), main_remote(remotes)) else {
        return Outcome::Refuse(Refusal::saying(
            "The main remote of this repository is not on github.com.",
        ));
    };
    let named =
        |side: &CompareSide, commit: &model::CommitId| name_on_github(&context.repository, &main.name, side, commit);
    let (first_name, second_name) = (named(&target.first, first), named(&target.second, second));
    let url = comparison_url(
        &repository,
        &ComparedOnGitHub {
            first: &first_name,
            second: &second_name,
            mode: target.mode,
        },
    );
    let title = format!("Open {} with {} on GitHub", target.first.name, target.second.name);
    Outcome::Run(no_step(title, vec![FollowUp::OpenUrl(url)]))
}

/// The name of a side on GitHub: a followed local branch by the branch of its upstream, when that upstream is on the
/// main remote and not gone; a followed remote branch of the main remote by its name without the remote; a followed
/// tag by its name; anything else by its full hash, which GitHub knows once it is pushed.
fn name_on_github(repository: &RepositoryState, main: &str, side: &CompareSide, commit: &model::CommitId) -> String {
    let full = || commit.as_str().to_owned();
    if !side.is_followed || side.is_gone {
        return full();
    }
    if let Some(branch) = repository.branches.iter().find(|branch| branch.name == side.name) {
        return branch
            .upstream
            .as_ref()
            .filter(|upstream| upstream.remote == main && !upstream.is_gone)
            .map_or_else(full, |upstream| {
                let name = upstream.remote_branch.strip_prefix("refs/heads/");
                name.unwrap_or(&upstream.remote_branch).to_owned()
            });
    }
    let on_main = side
        .name
        .strip_prefix(main)
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|_| repository.remote_branches.iter().any(|remote| remote.name == side.name));
    if let Some(name) = on_main {
        return name.to_owned();
    }
    if repository.tags.iter().any(|tag| tag.name == side.name) {
        return side.name.clone();
    }
    full()
}

#[cfg(test)]
#[path = "open_on_github_tests.rs"]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
