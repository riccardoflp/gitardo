//! The reads a rebase makes before it moves the other branches of a stack: the version of git, once for each engine,
//! and the branches on the commits it makes again (`repo.read-git-version`, `rebase.read-stacked-branches`).

use std::sync::OnceLock;

use git::read::rebase::{self, StackedRequest};
use git::read::repo::{GitVersionRequest, git_version_arguments, read_git_version};
use git::{Git, GitError};
use model::contract::{Answer, Query};
use model::rebase::GitVersion;

/// The version of the git of this engine, read the first time a feature asks: it is the same for every repository.
static VERSION: OnceLock<Option<GitVersion>> = OnceLock::new();

/// The answer of [`Query::GitVersion`] or [`Query::StackedBranches`], read now, or kept for the version.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::StackedBranches { base, tip } => {
            let request = StackedRequest {
                base: base.clone(),
                tip: tip.clone(),
            };
            rebase::read_stacked_branches(git, &request).map(Answer::Stacked)
        }
        _ => Ok(Answer::GitVersion(version_once(&VERSION, || {
            // A line of another form, or a git that does not start, is taken as a git too old for what needs a version.
            read_git_version(git, &GitVersionRequest).ok()
        }))),
    }
}

/// The version kept in `kept`, read with `read` the first time only.
fn version_once(kept: &OnceLock<Option<GitVersion>>, read: impl FnOnce() -> Option<GitVersion>) -> Option<GitVersion> {
    kept.get_or_init(read).clone()
}

/// The commands of these reads, as the log writes them at the debug level.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::StackedBranches { base, tip } => {
            let request = StackedRequest {
                base: base.clone(),
                tip: tip.clone(),
            };
            [
                rebase::made_again_arguments(&request),
                rebase::local_branches_arguments(),
                rebase::update_refs_arguments(),
            ]
            .iter()
            .map(|arguments| git::command_line(arguments))
            .collect::<Vec<_>>()
            .join(", ")
        }
        _ => git::command_line(&git_version_arguments()),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::cell::Cell;
    use std::sync::OnceLock;

    use model::rebase::GitVersion;

    use super::version_once;

    #[test]
    fn repo_read_git_version__runs_once_for_each_engine_however_many_repositories_are_open() {
        let kept = OnceLock::new();
        let reads = Cell::new(0);
        let read = || {
            reads.set(reads.get() + 1);
            Some(GitVersion {
                major: 2,
                minor: 53,
                patch: 0,
                said: String::from("2.53.0"),
            })
        };
        for _repository in 0..3 {
            assert_eq!(version_once(&kept, read).map(|version| version.minor), Some(53));
        }
        assert_eq!(reads.get(), 1);
    }
}
