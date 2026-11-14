use testkit::{Remote, Repository};

use crate::{checked_out, cli, cli_done, revision};

/// `main` with two commits, and `feature` at the first.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let _feature = repository.git(&["branch", "feature"]);
    let _second = repository.commit("Add b", &[("b.txt", "b")]);
    repository
}

#[test]
fn branch_checkout__checks_out_feature_and_undo_goes_back_to_main() {
    let repository = repository();
    let ended = cli_done(&repository, &["--target", "branch:feature", "branch.checkout"]);
    assert!(ended.out.contains("> git checkout feature"), "{}", ended.out);
    assert_eq!(checked_out(&repository), "feature");
    let undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert!(undone.out.contains("> git checkout main"), "{}", undone.out);
    assert_eq!(checked_out(&repository), "main");
}

#[test]
fn branch_checkout__is_followed_by_the_update_of_the_submodules() {
    let repository = repository();
    let mut library = Repository::new();
    let _first = library.commit("Add the library", &[("lib.txt", "1")]);
    let library_path = library.path().to_string_lossy().replace('\\', "/");
    let _added = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        "--quiet",
        &library_path,
        "lib",
    ]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Add lib"]);
    let ended = cli(&repository, &["--target", "branch:feature", "branch.checkout"]);
    assert!(
        ended.out.contains("> git checkout feature\n") || ended.out.contains("> git checkout feature\r\n"),
        "{}",
        ended.out
    );
    assert!(
        ended.out.contains("> git submodule update --init --recursive"),
        "{}",
        ended.out
    );
}

#[test]
fn branch_checkout_detached__checks_out_the_full_hash() {
    let repository = repository();
    let first = revision(&repository, "HEAD~1");
    let target = format!("commit:{first}");
    let ended = cli_done(&repository, &["--target", &target, "branch.checkout-detached"]);
    assert!(
        ended.out.contains(&format!("> git checkout --detach {first}")),
        "{}",
        ended.out
    );
    assert_eq!(checked_out(&repository), "HEAD");
    assert_eq!(revision(&repository, "HEAD"), first);
}

#[test]
fn tag_checkout__checks_out_the_tag_named_in_full_beside_a_branch_of_its_name() {
    let repository = repository();
    let _tagged = repository.git(&["tag", "v1.0.0", "HEAD~1"]);
    let _namesake = repository.git(&["branch", "v1.0.0", "HEAD"]);
    let ended = cli_done(&repository, &["--target", "tag:v1.0.0", "tag.checkout"]);
    assert!(
        ended.out.contains("> git checkout --detach refs/tags/v1.0.0"),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), revision(&repository, "refs/tags/v1.0.0"));
}

/// A repository whose `origin` has `feature` two commits past the local `feature`, which follows it.
fn behind() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let remote = Remote::new(&repository);
    let _branch = repository.git(&["checkout", "--quiet", "-b", "feature"]);
    let _second = repository.commit("Add b", &[("b.txt", "b")]);
    let _third = repository.commit("Add c", &[("c.txt", "c")]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", "HEAD~2"]);
    repository.checkout("main");
    (repository, remote)
}

#[test]
fn branch_checkout_remote__tracks_a_remote_branch_with_no_local_branch() {
    let (repository, _remote) = behind();
    let _gone = repository.git(&["branch", "-D", "feature"]);
    let ended = cli_done(
        &repository,
        &["--target", "remote-branch:origin/feature", "branch.checkout-remote"],
    );
    assert!(
        ended.out.contains("> git checkout --track origin/feature"),
        "{}",
        ended.out
    );
    assert_eq!(checked_out(&repository), "feature");
    assert_eq!(
        repository
            .git(&["rev-parse", "--abbrev-ref", "feature@{upstream}"])
            .trim(),
        "origin/feature"
    );
}

#[test]
fn branch_checkout_remote__checks_out_and_fast_forwards_a_branch_behind() {
    let (repository, _remote) = behind();
    let ended = cli_done(
        &repository,
        &["--target", "remote-branch:origin/feature", "branch.checkout-remote"],
    );
    assert!(ended.out.contains("> git checkout feature"), "{}", ended.out);
    assert!(
        ended.out.contains("> git merge --ff-only origin/feature"),
        "{}",
        ended.out
    );
    assert_eq!(
        revision(&repository, "feature"),
        revision(&repository, "origin/feature")
    );
}

#[test]
fn branch_checkout_remote_and_rebase__puts_the_commit_of_the_branch_on_top() {
    let (mut repository, _remote) = behind();
    repository.checkout("feature");
    let _own = repository.commit("Add d", &[("d.txt", "d")]);
    repository.checkout("main");
    let ended = cli_done(
        &repository,
        &[
            "--target",
            "remote-branch:origin/feature",
            "branch.checkout-remote-and-rebase",
        ],
    );
    assert!(
        ended.out.contains("> git rebase --autostash origin/feature feature"),
        "{}",
        ended.out
    );
    assert_eq!(checked_out(&repository), "feature");
    assert_eq!(repository.git(&["log", "-1", "--format=%s"]).trim(), "Add d");
    assert_eq!(revision(&repository, "HEAD~1"), revision(&repository, "origin/feature"));
}
