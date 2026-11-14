//! Merging into the branch checked out, in its four ways, and resetting it to a commit.

use testkit::Repository;

use crate::{cli, cli_done, revision, write};

/// `main` with a commit of its own and `feature` with one, both from `Add a`: no fast-forward is possible.
fn diverged() -> Repository {
    let mut repository = Repository::new();
    let _base = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    let _main = repository.commit("Add m", &[("m.txt", "m\n")]);
    repository.checkout("feature");
    let _feature = repository.commit("Add f", &[("f.txt", "f\n")]);
    repository.checkout("main");
    repository
}

#[test]
fn merge_into_current__merges_feature_into_main_with_git_merge_feature() {
    let repository = diverged();
    let ended = cli_done(&repository, &["--target", "branch:feature", "merge.into-current"]);
    assert!(ended.out.contains("> git merge feature\n"), "{}", ended.out);
    assert_eq!(repository.git(&["rev-list", "--count", "--merges", "HEAD"]).trim(), "1");
}

#[test]
fn merge_into_current__with_a_tag_of_the_same_name_merges_refs_heads_feature() {
    let repository = diverged();
    let _tag = repository.git(&["tag", "feature", "HEAD~1"]);
    let ended = cli_done(&repository, &["--target", "branch:feature", "merge.into-current"]);
    assert!(ended.out.contains("> git merge refs/heads/feature\n"), "{}", ended.out);
}

#[test]
fn merge_into_current__a_merge_predicted_to_conflict_asks_first_and_names_the_file() {
    let mut repository = Repository::new();
    let _base = repository.commit("Add a", &[("src/a.rs", "base\n")]);
    repository.branch("feature");
    let _main = repository.commit("Change a on main", &[("src/a.rs", "main\n")]);
    repository.checkout("feature");
    let _feature = repository.commit("Change a on feature", &[("src/a.rs", "feature\n")]);
    repository.checkout("main");
    let before = revision(&repository, "HEAD");
    let ended = cli(&repository, &["--target", "branch:feature", "merge.into-current"]);
    assert_eq!(ended.code, Some(2), "{}", ended.errors);
    assert!(
        ended.out.contains("This will stop with conflicts in 1 file:\nsrc/a.rs"),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), before);
}

#[test]
fn merge_choose_way__no_fast_forward_runs_git_merge_no_ff() {
    let mut repository = Repository::new();
    let _base = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _feature = repository.commit("Add f", &[("f.txt", "f\n")]);
    repository.checkout("main");
    let ended = cli_done(
        &repository,
        &["--target", "branch:feature", "merge.choose-way", "way=no-ff"],
    );
    assert!(ended.out.contains("> git merge --no-ff feature\n"), "{}", ended.out);
    assert_eq!(repository.git(&["rev-list", "--count", "--merges", "HEAD"]).trim(), "1");
}

#[test]
fn merge_choose_way__a_fast_forward_only_that_is_not_possible_runs_nothing() {
    let repository = diverged();
    let before = revision(&repository, "HEAD");
    let ended = cli(
        &repository,
        &["--target", "branch:feature", "merge.choose-way", "way=ff-only"],
    );
    assert_eq!(ended.code, Some(1));
    assert!(
        ended
            .errors
            .contains("main cannot be fast-forwarded to feature: it has commits that feature has not."),
        "{}",
        ended.errors
    );
    assert_eq!(revision(&repository, "HEAD"), before);
}

#[test]
fn merge_choose_way__closing_the_pick_runs_nothing() {
    let repository = diverged();
    let before = revision(&repository, "HEAD");
    let ended = cli(&repository, &["--target", "branch:feature", "merge.choose-way"]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.errors.contains("Merge feature into main"), "{}", ended.errors);
    assert_eq!(revision(&repository, "HEAD"), before);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn merge_choose_way__a_squash_stages_the_changes_and_commits_nothing() {
    let repository = diverged();
    let before = revision(&repository, "HEAD");
    let ended = cli_done(
        &repository,
        &["--yes", "--target", "branch:feature", "merge.choose-way", "way=squash"],
    );
    assert!(ended.out.contains("> git merge --squash feature\n"), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD"), before);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "f.txt\n");
}

#[test]
fn reset_to_commit__mixed_runs_git_reset_mixed_with_the_full_hash() {
    let mut repository = Repository::new();
    let first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
    let target = format!("commit:{first}");
    let ended = cli_done(&repository, &["--target", &target, "reset.to-commit", "mode=mixed"]);
    assert!(
        ended.out.contains(&format!("> git reset --mixed {first}\n")),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), first);
    assert_eq!(repository.git(&["status", "--porcelain"]), "?? b.txt\n");
}

#[test]
fn reset_to_commit__hard_shows_the_command_and_the_warning_first() {
    let mut repository = Repository::new();
    let first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let second = repository.commit("Add b", &[("b.txt", "b\n")]);
    write(&repository, "a.txt", "changed\n");
    let target = format!("commit:{first}");
    let ended = cli(&repository, &["--target", &target, "reset.to-commit", "mode=hard"]);
    assert_eq!(ended.code, Some(2));
    assert!(
        ended.out.contains(&format!("> git reset --hard {first}")),
        "{}",
        ended.out
    );
    assert!(
        ended
            .out
            .contains("A hard reset throws away all uncommitted changes to tracked files."),
        "{}",
        ended.out
    );
    assert_eq!(revision(&repository, "HEAD"), second);
}

#[test]
fn reset_to_commit__closing_the_pick_runs_nothing() {
    let mut repository = Repository::new();
    let first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let second = repository.commit("Add b", &[("b.txt", "b\n")]);
    let target = format!("commit:{first}");
    let ended = cli(&repository, &["--target", &target, "reset.to-commit"]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.errors.contains("Soft"), "{}", ended.errors);
    assert_eq!(revision(&repository, "HEAD"), second);
}
