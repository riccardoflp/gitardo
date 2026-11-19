//! A tag deleted from a remote and here, run from the window as its menu does (`tag.delete-remote`), on repositories
//! made for each test with a bare remote beside them.

use app::store::{Message, State};
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use testkit::{Remote, Repository};
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::OperationsDriver;

/// A repository whose `origin` has the tag `v1`, made here.
fn with_v1_pushed() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let remote = Remote::new(&repository);
    repository.tag("v1");
    let _ = repository.git(&["push", "--quiet", "origin", "refs/tags/v1"]);
    (repository, remote)
}

/// Runs **Delete here and from origin** on `v1`, says yes to its warning, and gives the lines of the log once it ended.
fn delete_here_and_from(repository: &Repository) -> Vec<String> {
    let mut state: State = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let here = Answers::new().with(AnswerKey("here"), Answer::Flag(true));
    let run = FeatureIntent::RunWith(FeatureId("tag.delete-remote"), Target::Tag(String::from("v1")), here);
    driver.intent(&mut state, Intent::Feature(run));
    loop {
        match driver.next(&mut state) {
            Message::FeatureConfirms(_) => driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => break,
            other => panic!("tag.delete-remote asks nothing here, and said {other:?}"),
        }
    }
    state.log.entries().map(|entry| entry.line.clone()).collect()
}

#[test]
fn tag_delete_remote__delete_here_and_from_origin_pushes_the_deletion_then_deletes_the_tag_here() {
    let (repository, remote) = with_v1_pushed();
    let lines = delete_here_and_from(&repository);
    let ran: Vec<&String> = lines.iter().filter(|line| line.starts_with("git ")).collect();
    assert_eq!(ran, ["git push origin --delete refs/tags/v1", "git tag -d v1"]);
    assert_eq!(remote.git(&["tag", "--list"]), "");
    assert_eq!(repository.git(&["tag", "--list"]), "");
}

#[test]
fn tag_delete_remote__when_the_push_fails_git_tag_d_does_not_run_and_the_tag_stays_here() {
    let (repository, _remote) = with_v1_pushed();
    let nowhere = repository.path().with_file_name("nowhere.git");
    let _ = repository.git(&["remote", "set-url", "origin", &nowhere.to_string_lossy()]);
    let lines = delete_here_and_from(&repository);
    assert!(!lines.iter().any(|line| line == "git tag -d v1"), "{lines:?}");
    assert_eq!(repository.git(&["tag", "--list"]), "v1\n");
}
