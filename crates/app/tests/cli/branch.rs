use testkit::Repository;

use crate::{checked_out, cli, cli_done, revision};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    let _second = repository.commit("Add b", &[("b.txt", "b")]);
    repository
}

#[test]
fn branch_create__creates_and_checks_out_from_the_toolbar() {
    let repository = repository();
    let ended = cli_done(&repository, &["branch.create", "name=fix/login", "mode=checkout"]);
    assert!(ended.out.contains("> git checkout -b fix/login HEAD"), "{}", ended.out);
    assert_eq!(checked_out(&repository), "fix/login");
}

#[test]
fn branch_create__creates_only_on_a_commit_and_stays() {
    let repository = repository();
    let first = revision(&repository, "HEAD~1");
    let target = format!("commit:{first}");
    let ended = cli_done(
        &repository,
        &["--target", &target, "branch.create", "name=old", "mode=only"],
    );
    assert!(
        ended.out.contains(&format!("> git branch old {first}")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "refs/heads/old"), first);
    assert_eq!(checked_out(&repository), "main");
}

#[test]
fn branch_create__on_a_branch_named_like_a_tag_takes_the_branch() {
    let repository = repository();
    repository.branch("release");
    let _tagged = repository.git(&["tag", "release", "HEAD~1"]);
    let ended = cli_done(
        &repository,
        &["--target", "branch:release", "branch.create", "name=next", "mode=only"],
    );
    assert!(
        ended.out.contains("> git branch next refs/heads/release"),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "refs/heads/next"), revision(&repository, "main"));
}

#[test]
fn branch_create__refuses_a_name_that_is_taken() {
    let repository = repository();
    let ended = cli(&repository, &["branch.create", "name=main", "mode=checkout"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("A branch with that name already exists"),
        "{}",
        ended.errors
    );
}

#[test]
fn branch_create__backing_out_runs_nothing() {
    let repository = repository();
    let ended = cli(&repository, &["branch.create", "name=unfinished"]);
    assert_eq!(ended.code, Some(2));
    assert!(
        ended.errors.contains("mode=only"),
        "the choices are said: {}",
        ended.errors
    );
    assert_eq!(repository.git(&["branch", "--list", "unfinished"]), "");
}
