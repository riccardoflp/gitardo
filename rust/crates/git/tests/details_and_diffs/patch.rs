//! The reads of patches against throwaway repositories: `patch.read-format-patch` and `patch.check`.

use std::fs;

use git::read::patch::{FormatPatchRequest, PatchCheckRequest, PatchSource, check, read_format_patch};
use model::CommitId;
use model::patch::PatchCheck;
use testkit::Repository;

use crate::awkward::{git_in, write};

/// A commit of the throwaway repositories, by its hash.
fn id(hash: &str) -> CommitId {
    CommitId::parse(hash).unwrap()
}

/// The patch of some commits, in the order given.
fn format_patch(repository: &Repository, commits: &[&str]) -> String {
    let request = FormatPatchRequest {
        commits: commits.iter().map(|commit| id(commit)).collect(),
    };
    read_format_patch(&git_in(repository), &request).unwrap()
}

/// The hashes the commits of a patch start with, in order.
fn patch_commits(patch: &str) -> Vec<&str> {
    patch
        .lines()
        .filter_map(|line| line.strip_prefix("From ")?.split(' ').next())
        .filter(|hash| hash.len() == 40)
        .collect()
}

/// A history of four commits on `main`.
fn history() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let commits = (1..=4)
        .map(|number| repository.commit(&format!("Change {number}"), &[("a.txt", &format!("{number}\n"))]))
        .collect();
    (repository, commits)
}

#[test]
fn patch_read_format_patch__three_commits_in_any_order_give_one_patch_oldest_first() {
    let (repository, commits) = history();
    let patch = format_patch(&repository, &[&commits[3], &commits[1], &commits[2]]);
    assert_eq!(patch_commits(&patch), [&commits[1], &commits[2], &commits[3]]);
    let patch = format_patch(&repository, &[&commits[1], &commits[3], &commits[0]]);
    assert_eq!(patch_commits(&patch), [&commits[0], &commits[1], &commits[3]]);
}

#[test]
fn patch_read_format_patch__one_commit_gives_a_patch_of_that_commit_only() {
    let (repository, commits) = history();
    let patch = format_patch(&repository, &[&commits[1]]);
    assert_eq!(patch_commits(&patch), [&commits[1]]);
}

#[test]
fn patch_read_format_patch__each_commit_starts_with_its_full_hash_and_has_its_author_date_and_message() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let second = repository.commit("Fix the parser: [draft] é\n\nWith a body.", &[("parser.rs", "2\n")]);
    let patch = format_patch(&repository, &[&first, &second]);
    assert!(patch.starts_with(&format!("From {first} ")));
    assert!(patch.contains(&format!("\nFrom {second} ")));
    assert!(patch.contains("From: Test Author <author@example.com>"));
    assert!(patch.contains("Date: Wed, 1 Jan 2025 00:01:00 +0000"));
    assert!(patch.contains("Subject: [PATCH 1/2] Add the parser\n"));
    assert!(patch.contains("With a body."));
}

#[test]
fn patch_read_format_patch__a_merge_commit_gives_no_patch_of_its_own() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("a.txt", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let feature = repository.commit("On feature", &[("b.txt", "1\n")]);
    repository.checkout("main");
    let _ = repository.commit("On main", &[("a.txt", "2\n")]);
    repository.merge("feature");
    let merge = repository.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let patch = format_patch(&repository, &[&merge, &feature]);
    assert_eq!(patch_commits(&patch), [&feature]);
}

#[test]
fn patch_read_format_patch__no_external_diff_or_text_conversion_of_the_configuration_is_used() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("a.txt", "abc\n"), (".gitattributes", "*.txt diff=upper\n")]);
    let _ = repository.git(&["config", "diff.upper.textconv", "sed s/b/B/"]);
    let _ = repository.git(&["config", "diff.external", "echo EXTERNAL"]);
    let second = repository.commit("Change", &[("a.txt", "abd\n")]);
    let patch = format_patch(&repository, &[&second]);
    assert!(patch.contains("\n-abc\n+abd\n"), "{patch}");
    assert!(!patch.contains("EXTERNAL"));
}

/// A repository where `a.txt` changed on `main` after the commit a patch is made of, on `feature`.
fn patch_and_repository() -> (Repository, String) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("a.txt", "one\ntwo\nthree\n"), ("b.txt", "b\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit("Change two", &[("a.txt", "one\nTWO\nthree\n")]);
    let patch = repository.git(&["format-patch", "-1", "--stdout"]);
    repository.checkout("main");
    (repository, patch)
}

/// What checking a patch given as text says.
fn checked(repository: &Repository, patch: &str) -> PatchCheck {
    let request = PatchCheckRequest {
        patch: PatchSource::Text(patch.as_bytes().to_vec()),
    };
    check(&git_in(repository), &request).unwrap()
}

#[test]
fn patch_check__a_patch_that_applies_cleanly_has_no_conflicts_and_no_errors() {
    let (repository, patch) = patch_and_repository();
    assert_eq!(
        checked(&repository, &patch),
        PatchCheck::Applies { conflicts: Vec::new() }
    );
    let file = repository.path().parent().unwrap().join("-change.patch");
    fs::write(&file, &patch).unwrap();
    let request = PatchCheckRequest {
        patch: PatchSource::File(file),
    };
    assert_eq!(
        check(&git_in(&repository), &request).unwrap(),
        PatchCheck::Applies { conflicts: Vec::new() }
    );
}

#[test]
fn patch_check__a_patch_that_three_way_merges_with_conflicts_names_those_files() {
    let (mut repository, patch) = patch_and_repository();
    let _ = repository.commit("Change two otherwise", &[("a.txt", "one\n2\nthree\n")]);
    assert_eq!(
        checked(&repository, &patch),
        PatchCheck::Applies {
            conflicts: vec![String::from("a.txt")]
        }
    );
}

#[test]
fn patch_check__a_patch_of_a_file_that_is_not_there_returns_git_reason_and_changes_nothing() {
    let (mut repository, patch) = patch_and_repository();
    let _ = repository.git(&["rm", "--quiet", "--", "a.txt"]);
    let _ = repository.commit("Remove a", &[]);
    let index = fs::read(repository.path().join(".git/index")).unwrap();
    let PatchCheck::Refused { reasons } = checked(&repository, &patch) else {
        panic!("the patch applied")
    };
    assert_eq!(reasons, ["a.txt: does not exist in index"]);
    assert_eq!(fs::read(repository.path().join(".git/index")).unwrap(), index);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn patch_check__runs_as_a_read_while_another_program_holds_the_index() {
    let (repository, patch) = patch_and_repository();
    write(&repository, ".git/index.lock", b"held by a commit in a terminal");
    assert_eq!(
        checked(&repository, &patch),
        PatchCheck::Applies { conflicts: Vec::new() }
    );
    assert!(repository.path().join(".git/index.lock").exists());
    fs::remove_file(repository.path().join(".git/index.lock")).unwrap();
}
