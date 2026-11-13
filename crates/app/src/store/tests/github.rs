//! When the window reads the pull requests of GitHub on its own (`github.read-pull-requests`): after a read of the
//! repository when the last request is old, and after a push whatever was read last.

use git::read::graph::CommitPage;
use model::contract::{FollowUp, Text};
use model::github::{GitHubRepo, PullRequests};
use model::repo::Repository;

use super::{folder, open, repository_state, window};
use crate::store::feature::Ended;
use crate::store::github::{GitHubMessage, GitHubWork, PullsRead};
use crate::store::{Effect, Found, JobKind, Message, State, reduce};

/// The repository on GitHub of the tests.
fn on_github() -> GitHubRepo {
    GitHubRepo {
        owner: String::from("owner"),
        repo: String::from("app"),
    }
}

/// Answers the jobs of the effects as the workers and the thread of the pull requests would, the repository read at a
/// moment, its main remote on GitHub, the pull requests read; gives how many requests were sent to GitHub.
fn answer(state: &mut State, effects: Vec<Effect>, read_at: i64) -> usize {
    let mut requests = 0;
    let mut waiting = effects;
    while let Some(effect) = waiting.pop() {
        let message = match effect {
            Effect::Start(job) => match job.kind {
                JobKind::ReadRepository { folder, .. } => Message::RepositoryRead {
                    job: job.id,
                    result: Ok(Box::new((repository_state(&folder), CommitPage::default()))),
                    folder,
                    read_at,
                },
                JobKind::FindRepositories { folders } => Message::RepositoriesFound {
                    job: job.id,
                    found: Found::Repositories(
                        folders
                            .into_iter()
                            .map(|folder| Repository {
                                folder,
                                superproject: None,
                            })
                            .collect(),
                    ),
                },
                _ => continue,
            },
            Effect::GitHub(job) => Message::GitHub(match job.work {
                GitHubWork::Detect => GitHubMessage::Detected {
                    job: job.id,
                    repository: Some(on_github()),
                },
                GitHubWork::Read { .. } => {
                    requests += 1;
                    GitHubMessage::Read {
                        job: job.id,
                        result: PullsRead::Read(PullRequests::default()),
                    }
                }
            }),
            _ => continue,
        };
        waiting.extend(reduce(state, message));
    }
    requests
}

/// A window showing `/work/app`, read at the second 0, whose pull requests were read then with the token kept.
fn showing() -> State {
    let mut state = window();
    let app = folder("app");
    let effects = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    assert_eq!(answer(&mut state, effects, 0), 1, "the first read asks GitHub");
    state
}

/// Reads the repository again at a moment, and gives how many requests were sent to GitHub.
fn read_again(state: &mut State, read_at: i64) -> usize {
    let effects = reduce(state, Message::Intent(views::Intent::Refresh));
    answer(state, effects, read_at)
}

#[test]
fn github_read_pull_requests__two_reads_of_the_repository_30_seconds_apart_send_one_request_not_two() {
    let mut state = showing();
    assert_eq!(read_again(&mut state, 30), 0);
    assert_eq!(read_again(&mut state, 60), 0);
    assert_eq!(
        read_again(&mut state, 121),
        1,
        "more than 120 seconds after the last request"
    );
    assert_eq!(read_again(&mut state, 151), 0);
}

#[test]
fn github_read_pull_requests__a_push_that_succeeded_is_followed_by_a_request_even_10_seconds_after_the_last() {
    let mut state = showing();
    let pushed = Ended::Ran {
        title: Text::from("Push main"),
        after: vec![FollowUp::ReadPullRequests],
        can_undo: false,
        prepared_message: None,
    };
    let effects = reduce(
        &mut state,
        Message::FeatureEnded {
            ended: pushed,
            log: Vec::new(),
        },
    );
    assert_eq!(answer(&mut state, effects, 10), 1);
    assert_eq!(read_again(&mut state, 20), 0, "and not again 10 seconds after it");
}
