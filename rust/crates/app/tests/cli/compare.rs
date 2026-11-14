use testkit::Repository;

use crate::{cli, cli_done, revision};

/// `main` with two commits, and `feature` one commit ahead of it, `main` checked out.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _feature = repository.commit("Add b", &[("b.txt", "b\n")]);
    repository.checkout("main");
    let _second = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository
}

#[test]
fn compare_choose__main_then_feature_opens_the_comparison_of_the_two() {
    let repository = repository();
    let ended = cli_done(&repository, &["compare.choose", "first=ref:main", "second=ref:feature"]);
    assert!(ended.out.contains("Compare main with feature"), "{}", ended.out);
}

#[test]
fn compare_choose__a_name_git_does_not_know_cannot_be_accepted() {
    let repository = repository();
    let ended = cli(
        &repository,
        &["compare.choose", "first=revision", "first-revision=nothing-here"],
    );
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("\"nothing-here\" is not a commit of this repository."),
        "{}",
        ended.errors
    );
}

#[test]
fn compare_choose__typing_head_tilde_names_the_side_as_typed() {
    let repository = repository();
    let ended = cli_done(
        &repository,
        &[
            "compare.choose",
            "first=revision",
            "first-revision=HEAD~1",
            "second=working-tree",
        ],
    );
    assert!(ended.out.contains("Compare HEAD~1 with Working tree"), "{}", ended.out);
}

#[test]
fn compare_with__on_feature_choosing_main_opens_feature_with_main() {
    let repository = repository();
    let ended = cli_done(
        &repository,
        &["--target", "branch:feature", "compare.with", "second=ref:main"],
    );
    assert!(ended.out.contains("Compare feature with main"), "{}", ended.out);
}

#[test]
fn compare_with_current__on_feature_compares_it_with_main_checked_out() {
    let repository = repository();
    let ended = cli_done(&repository, &["--target", "branch:feature", "compare.with-current"]);
    assert!(ended.out.contains("Compare feature with main"), "{}", ended.out);
}

#[test]
fn compare_selected_before__compares_the_two_commits_in_order() {
    let repository = repository();
    let (before, clicked) = (revision(&repository, "HEAD~1"), revision(&repository, "feature"));
    let target = format!("commits:{before},{clicked}");
    let ended = cli_done(&repository, &["--target", &target, "compare.selected-before"]);
    let short = |hash: &str| hash.get(..7).unwrap_or_default().to_owned();
    let said = format!("Compare {} with {}", short(&before), short(&clicked));
    assert!(ended.out.contains(&said), "{}", ended.out);
}

#[test]
fn reflog_browse__after_a_reset_the_lost_commit_is_listed_and_a_branch_made_there() {
    let mut repository = repository();
    let _third = repository.commit("Add d", &[("d.txt", "d\n")]);
    let lost = revision(&repository, "HEAD");
    let _reset = repository.git(&["reset", "--quiet", "--hard", "HEAD~2"]);
    let listed = cli(&repository, &["reflog.browse"]);
    assert!(
        listed.errors.contains("reflog-entry=HEAD@{1}  commit: Add d"),
        "{}",
        listed.errors
    );
    let ended = cli_done(
        &repository,
        &[
            "--yes",
            "reflog.browse",
            "reflog-entry=HEAD@{1}",
            "reflog-action=branch",
            "name=lost",
            "mode=only",
        ],
    );
    assert!(
        ended.out.contains(&format!("> git branch lost {lost}")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "refs/heads/lost"), lost);
}
