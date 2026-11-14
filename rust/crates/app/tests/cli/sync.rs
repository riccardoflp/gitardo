use testkit::{Remote, Repository};

use crate::{cli, cli_done, revision};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository
}

#[test]
fn sync_fetch__prunes_a_branch_deleted_on_the_remote() {
    let repository = repository();
    let remote = Remote::new(&repository);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main:gone"]);
    assert!(repository.git(&["branch", "--remotes"]).contains("origin/gone"));
    let _deleted = remote.git(&["branch", "-D", "gone"]);
    let ended = cli_done(&repository, &["sync.fetch"]);
    assert!(ended.out.contains("> git fetch --all --prune"), "{}", ended.out);
    assert!(!repository.git(&["branch", "--remotes"]).contains("origin/gone"));
}

#[test]
fn sync_fetch__counts_ahead_and_behind_with_the_new_remote_branches() {
    let mut repository = repository();
    let _remote = Remote::new(&repository);
    let before = revision(&repository, "HEAD");
    // The remote's main moves one commit on, and this repository has not fetched it yet.
    let _later = repository.commit("Add b", &[("b.txt", "b")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &before]);
    let _not_fetched = repository.git(&["update-ref", "refs/remotes/origin/main", &before]);
    assert!(cli_done(&repository, &["state"]).out.contains("origin/main +0 -0"));
    let _fetched = cli_done(&repository, &["sync.fetch"]);
    let state = cli_done(&repository, &["state"]).out;
    assert!(state.contains("origin/main +0 -1"), "{state}");
}

#[test]
fn sync_fetch__refuses_without_a_remote() {
    let repository = repository();
    let ended = cli(&repository, &["sync.fetch"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("This repository has no remote."),
        "{}",
        ended.errors
    );
}
