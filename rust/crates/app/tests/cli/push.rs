use testkit::{Remote, Repository};

use crate::{cli, cli_done, revision, write};

/// A repository with `main` pushed to `origin`, which it follows.
fn pushed() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let remote = Remote::new(&repository);
    (repository, remote)
}

fn remote_revision(remote: &Remote, name: &str) -> String {
    remote.git(&["rev-parse", name]).trim_end().to_owned()
}

#[test]
fn sync_pull__keeps_a_modified_file_and_leaves_untracked_files_alone() {
    let (mut repository, _remote) = pushed();
    let before = revision(&repository, "HEAD");
    let _later = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &before]);
    write(&repository, "a.txt", "a, changed\n");
    write(&repository, "notes.txt", "mine\n");
    let ended = cli_done(&repository, &["sync.pull"]);
    assert!(ended.out.contains("> git pull --rebase --autostash"), "{}", ended.out);
    assert!(repository.path().join("b.txt").exists(), "what came in is there");
    let status = repository.git(&["status", "--porcelain"]);
    assert!(status.contains(" M a.txt"), "{status}");
    assert!(status.contains("?? notes.txt"), "{status}");
}

#[test]
fn sync_pull__undo_right_after_moves_the_branch_back() {
    let (mut repository, _remote) = pushed();
    let before = revision(&repository, "HEAD");
    let _later = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &before]);
    let _pulled = cli_done(&repository, &["sync.pull"]);
    assert_ne!(revision(&repository, "HEAD"), before);
    let _undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "HEAD"), before);
}

#[test]
fn sync_pull_with__no_options_runs_git_pull_alone() {
    let (repository, _remote) = pushed();
    let ended = cli_done(&repository, &["sync.pull-with", "way=plain"]);
    assert!(ended.out.contains("> git pull\n"), "{}", ended.out);
}

#[test]
fn sync_push__main_following_origin_main_pushes_to_it() {
    let (mut repository, remote) = pushed();
    let _later = repository.commit("Add b", &[("b.txt", "b\n")]);
    let ended = cli_done(&repository, &["sync.push"]);
    assert!(ended.out.contains("> git push origin main"), "{}", ended.out);
    assert_eq!(remote_revision(&remote, "main"), revision(&repository, "HEAD"));
}

#[test]
fn sync_push__a_branch_following_another_name_pushes_there() {
    let (mut repository, remote) = pushed();
    let _local = repository.git(&["checkout", "--quiet", "-b", "local"]);
    let _published = repository.git(&["push", "--quiet", "--set-upstream", "origin", "local:remote-name"]);
    let _later = repository.commit("Add b", &[("b.txt", "b\n")]);
    let ended = cli_done(&repository, &["sync.push"]);
    assert!(
        ended.out.contains("> git push origin local:remote-name"),
        "{}",
        ended.out
    );
    assert_eq!(remote_revision(&remote, "remote-name"), revision(&repository, "HEAD"));
}

#[test]
fn sync_push__a_new_branch_is_published_and_follows_it_after() {
    let (repository, _remote) = pushed();
    let _feature = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let ended = cli_done(&repository, &["sync.push"]);
    assert!(
        ended.out.contains("> git push --set-upstream origin feature"),
        "{}",
        ended.out
    );
    let upstream = repository.git(&["rev-parse", "--abbrev-ref", "feature@{upstream}"]);
    assert_eq!(upstream.trim_end(), "origin/feature");
}

#[test]
fn sync_push__a_branch_with_a_tag_of_its_name_is_named_in_full() {
    let (mut repository, remote) = pushed();
    let _release = repository.git(&["checkout", "--quiet", "-b", "release"]);
    let _published = repository.git(&["push", "--quiet", "--set-upstream", "origin", "release"]);
    repository.tag("release");
    let _later = repository.commit("Add b", &[("b.txt", "b\n")]);
    let ended = cli_done(&repository, &["sync.push"]);
    assert!(
        ended.out.contains("> git push origin refs/heads/release"),
        "{}",
        ended.out
    );
    assert_eq!(
        remote_revision(&remote, "refs/heads/release"),
        revision(&repository, "HEAD")
    );
}

#[test]
fn sync_push__a_detached_head_runs_nothing() {
    let (repository, _remote) = pushed();
    let _detached = repository.git(&["checkout", "--quiet", "--detach"]);
    let ended = cli(&repository, &["sync.push"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("Cannot push: HEAD is detached."),
        "{}",
        ended.errors
    );
}

#[test]
fn remote_add__push_in_a_repository_with_no_remote_offers_to_add_one() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let ended = cli(&repository, &["sync.push"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("This repository has no remote."),
        "{}",
        ended.errors
    );
    assert!(
        ended.errors.contains("Instead: Add remote… (remote.add)"),
        "{}",
        ended.errors
    );
    let ended = cli_done(
        &repository,
        &["remote.add", "name=origin", "url=https://github.com/owner/app.git"],
    );
    assert!(
        ended
            .out
            .contains("> git remote add origin https://github.com/owner/app.git"),
        "{}",
        ended.out
    );
}

#[test]
fn sync_push_force_with_lease__after_a_rebase_asks_with_the_commits_then_forces() {
    let (mut repository, remote) = pushed();
    let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _amended = repository.git(&["commit", "--quiet", "--amend", "-m", "Add b, better"]);
    let asked = cli(&repository, &["sync.push-force-with-lease"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.out);
    assert!(
        asked
            .out
            .contains("These commits on `origin/main` are not in `main` and will be overwritten:"),
        "{}",
        asked.out
    );
    assert!(asked.out.contains(" Add b\n"), "{}", asked.out);
    let ended = cli_done(&repository, &["--yes", "sync.push-force-with-lease"]);
    assert!(
        ended.out.contains("> git push --force-with-lease origin main"),
        "{}",
        ended.out
    );
    assert_eq!(remote_revision(&remote, "main"), revision(&repository, "HEAD"));
}

#[test]
fn sync_push_force_with_lease__refuses_when_the_remote_moved_since_the_fetch() {
    let (mut repository, remote) = pushed();
    let first = revision(&repository, "HEAD");
    // Someone pushed `Add b`; this repository has not fetched it.
    let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let theirs = revision(&repository, "HEAD");
    let _back = repository.git(&["reset", "--quiet", "--hard", &first]);
    let _not_fetched = repository.git(&["update-ref", "refs/remotes/origin/main", &first]);
    let _mine = repository.commit("Add c", &[("c.txt", "c\n")]);
    let ended = cli(&repository, &["--yes", "sync.push-force-with-lease"]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert_eq!(remote_revision(&remote, "main"), theirs);
}

#[test]
fn sync_push_force__asks_with_what_it_overwrites_then_forces() {
    let (mut repository, remote) = pushed();
    let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
    let asked = cli(&repository, &["sync.push-force"]);
    assert_eq!(asked.code, Some(2));
    assert!(asked.out.contains(" Add b\n"), "{}", asked.out);
    let ended = cli_done(&repository, &["--yes", "sync.push-force"]);
    assert!(ended.out.contains("> git push --force origin main"), "{}", ended.out);
    assert_eq!(remote_revision(&remote, "main"), revision(&repository, "HEAD"));
}
