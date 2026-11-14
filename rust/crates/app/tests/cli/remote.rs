use testkit::{Remote, Repository};

use crate::{cli, cli_done};

/// A repository with `main` pushed to `origin`, and the same bare repository as `upstream` too, fetched.
fn with_two_remotes() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    let url = remote.path().to_string_lossy().into_owned();
    let _added = repository.git(&["remote", "add", "upstream", &url]);
    let _fetched = repository.git(&["fetch", "--quiet", "upstream"]);
    (repository, remote)
}

#[test]
fn remote_fetch__fetches_one_remote_and_forgets_its_deleted_branches() {
    let (repository, remote) = with_two_remotes();
    let _pushed = repository.git(&["push", "--quiet", "upstream", "main:gone"]);
    let _fetched = repository.git(&["fetch", "--quiet", "upstream"]);
    assert!(repository.git(&["branch", "--remotes"]).contains("upstream/gone"));
    let _deleted = remote.git(&["branch", "-D", "gone"]);
    let ended = cli_done(&repository, &["--target", "remote:upstream", "remote.fetch"]);
    assert!(ended.out.contains("> git fetch --prune upstream"), "{}", ended.out);
    assert!(!repository.git(&["branch", "--remotes"]).contains("upstream/gone"));
}

#[test]
fn remote_set_url__changes_the_url_and_runs_nothing_when_unchanged() {
    let (repository, _remote) = with_two_remotes();
    let ended = cli_done(
        &repository,
        &[
            "--target",
            "remote:origin",
            "remote.set-url",
            "url=git@github.com:owner/app.git",
        ],
    );
    assert!(
        ended
            .out
            .contains("> git remote set-url origin git@github.com:owner/app.git"),
        "{}",
        ended.out
    );
    assert_eq!(
        repository.git(&["config", "remote.origin.url"]).trim_end(),
        "git@github.com:owner/app.git"
    );
    let same = cli_done(
        &repository,
        &[
            "--target",
            "remote:origin",
            "remote.set-url",
            "url=git@github.com:owner/app.git",
        ],
    );
    assert!(!same.out.contains("> git"), "nothing runs: {}", same.out);
}

#[test]
fn remote_rename__renames_its_branches_and_the_upstreams_that_name_it() {
    let (repository, _remote) = with_two_remotes();
    let ended = cli_done(
        &repository,
        &["--target", "remote:origin", "remote.rename", "new-name=github"],
    );
    assert!(ended.out.contains("> git remote rename origin github"), "{}", ended.out);
    let branches = repository.git(&["branch", "--remotes"]);
    assert!(
        branches.contains("github/main") && !branches.contains("origin/main"),
        "{branches}"
    );
    let upstream = repository.git(&["rev-parse", "--abbrev-ref", "main@{upstream}"]);
    assert_eq!(upstream.trim_end(), "github/main");
}

#[test]
fn remote_remove__asks_first_then_removes_and_its_followers_lose_their_upstream() {
    let (repository, _remote) = with_two_remotes();
    let _follows = repository.git(&["branch", "--quiet", "--track", "follower", "upstream/main"]);
    let asked = cli(&repository, &["--target", "remote:upstream", "remote.remove"]);
    assert_eq!(asked.code, Some(2), "it asks also with commandPreview off");
    assert!(repository.git(&["remote"]).contains("upstream"));
    let ended = cli_done(&repository, &["--yes", "--target", "remote:upstream", "remote.remove"]);
    assert!(ended.out.contains("> git remote remove upstream"), "{}", ended.out);
    let state = cli_done(&repository, &["state"]).out;
    let follower = state.lines().find(|line| line.contains("follower")).unwrap_or_default();
    assert!(!follower.contains("upstream/"), "{follower}");
}

#[test]
fn remote_delete_branch__asks_then_deletes_the_branch_on_the_remote() {
    let (repository, remote) = with_two_remotes();
    let _pushed = repository.git(&["push", "--quiet", "origin", "main:feature/login"]);
    let target = ["--target", "remote-branch:origin/feature/login", "remote.delete-branch"];
    let asked = cli(&repository, &target);
    assert_eq!(asked.code, Some(2), "it asks also with commandPreview off");
    let mut confirmed = vec!["--yes"];
    confirmed.extend(target);
    let ended = cli_done(&repository, &confirmed);
    assert!(
        ended.out.contains("> git push origin --delete feature/login"),
        "{}",
        ended.out
    );
    assert!(
        !repository
            .git(&["branch", "--remotes"])
            .contains("origin/feature/login")
    );
    assert!(!remote.git(&["branch"]).contains("feature/login"));
}

#[test]
fn remote_manage__copy_url_logs_nothing() {
    let (repository, _remote) = with_two_remotes();
    let ended = cli_done(&repository, &["remote.manage", "remote=origin", "action=copy"]);
    assert!(ended.out.contains("Copy "), "{}", ended.out);
    assert!(!ended.out.contains("> git"), "{}", ended.out);
}
