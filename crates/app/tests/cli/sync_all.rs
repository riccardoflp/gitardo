use testkit::{Remote, Repository};

use crate::{cli, cli_done, revision};

/// `main` one commit ahead of `origin/main` and two behind it, as of the last fetch; with the commit it was at before
/// its own.
fn ahead_and_behind() -> (Repository, Remote, String) {
    let mut repository = Repository::new();
    let base = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    let _theirs = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _more = repository.commit("Add c", &[("c.txt", "c\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    let mine = repository.commit("Add d", &[("d.txt", "d\n")]);
    (repository, remote, mine)
}

fn remote_main(remote: &Remote) -> String {
    remote.git(&["rev-parse", "main"]).trim_end().to_owned()
}

#[test]
fn sync_sync__fetches_pulls_and_pushes_and_main_meets_its_upstream() {
    let (repository, remote, _mine) = ahead_and_behind();
    assert!(cli_done(&repository, &["state"]).out.contains("origin/main +1 -2"));
    let ended = cli_done(&repository, &["sync.sync"]);
    let fetch = ended.out.find("> git fetch --all --prune");
    let pull = ended.out.find("> git pull --rebase --autostash");
    let push = ended.out.find("> git push origin main");
    assert!(fetch < pull && pull < push && fetch.is_some(), "{}", ended.out);
    assert_eq!(revision(&repository, "main"), revision(&repository, "origin/main"));
    assert_eq!(remote_main(&remote), revision(&repository, "main"));
}

#[test]
fn sync_sync__with_nothing_to_push_has_no_push() {
    let (repository, _remote, _mine) = ahead_and_behind();
    let _pushless = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let ended = cli(&repository, &["--plan", "sync.sync"]);
    assert!(ended.out.contains("> git pull"), "{}", ended.out);
    assert!(!ended.out.contains("> git push"), "{}", ended.out);
}

#[test]
fn sync_sync__with_command_preview_all_shows_the_commands_and_runs_nothing() {
    let (repository, remote, mine) = ahead_and_behind();
    let before = remote_main(&remote);
    let ended = cli(&repository, &["--plan", "sync.sync"]);
    assert!(ended.out.contains("> git fetch --all --prune"), "{}", ended.out);
    assert!(ended.out.contains("> git push origin main"), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD"), mine, "nothing ran");
    assert_eq!(remote_main(&remote), before);
}

#[test]
fn sync_sync__a_branch_with_no_upstream_is_only_published() {
    let (repository, _remote, _mine) = ahead_and_behind();
    let _feature = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let asked = cli(&repository, &["sync.sync"]);
    assert_eq!(asked.code, Some(2));
    assert!(
        asked
            .errors
            .contains("`feature` has no upstream. Publish it to `origin`?"),
        "{}",
        asked.errors
    );
    let ended = cli_done(&repository, &["sync.sync", "publish=yes"]);
    assert!(
        ended.out.contains("> git push --set-upstream origin feature"),
        "{}",
        ended.out
    );
    assert!(!ended.out.contains("> git pull"), "{}", ended.out);
    let upstream = repository.git(&["rev-parse", "--abbrev-ref", "feature@{upstream}"]);
    assert_eq!(upstream.trim_end(), "origin/feature");
}

#[test]
fn sync_sync__a_pull_that_stops_on_a_conflict_pushes_nothing() {
    let mut repository = Repository::new();
    let base = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    let _theirs = repository.commit("Change a there", &[("a.txt", "theirs\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let theirs = remote_main(&remote);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    let _mine = repository.commit("Change a here", &[("a.txt", "mine\n")]);
    let ended = cli(&repository, &["sync.sync"]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(!ended.out.contains("> git push"), "{}", ended.out);
    let state = cli_done(&repository, &["state"]).out;
    assert!(state.contains("Rebase in progress"), "the conflicts are shown: {state}");
    assert!(state.contains("1 in conflict"), "{state}");
    assert_eq!(remote_main(&remote), theirs, "the remote is where it was");
}

#[test]
fn sync_sync__a_detached_head_runs_nothing() {
    let (repository, _remote, _mine) = ahead_and_behind();
    let _detached = repository.git(&["checkout", "--quiet", "--detach"]);
    let ended = cli(&repository, &["sync.sync"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("Cannot sync: HEAD is detached. Check out a branch first."),
        "{}",
        ended.errors
    );
}

#[test]
fn sync_sync__undo_moves_main_back_before_the_pull_and_leaves_the_remote() {
    let (repository, remote, mine) = ahead_and_behind();
    let _synced = cli_done(&repository, &["sync.sync"]);
    let pushed = remote_main(&remote);
    let _undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "main"), mine);
    assert_eq!(remote_main(&remote), pushed);
}
