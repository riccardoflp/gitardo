//! `reflog.read` and `bisect.read` against throwaway repositories.

use git::read::bisect::{BisectRequest, read};
use git::read::reflog::{ReflogRequest, read as read_reflog};
use model::bisect::Bisect;
use model::reflog::ReflogPosition;
use testkit::{FIRST_COMMIT_TIME, Repository};

use crate::git;

/// The last positions of `HEAD`.
fn positions(repository: &Repository, count: usize) -> Vec<ReflogPosition> {
    let request = ReflogRequest {
        reference: String::from("HEAD"),
        count,
    };
    read_reflog(&git(repository), &request).unwrap()
}

#[test]
fn reflog_read__gives_each_entry_its_four_fields_newest_first() {
    let mut repository = Repository::new();
    let commits: Vec<String> = (1..=3)
        .map(|number| repository.commit(&format!("Commit {number}"), &[("file.txt", &format!("{number}\n"))]))
        .collect();
    let positions = positions(&repository, 10);
    assert_eq!(positions.len(), 3);
    let first = &positions[0];
    assert_eq!(first.commit.as_str(), commits[2]);
    assert_eq!(first.subject, "commit: Commit 3");
    assert_eq!(first.selector, "HEAD@{0}");
    assert_eq!(first.time.0, i64::try_from(FIRST_COMMIT_TIME).unwrap() + 120);
    let selectors: Vec<&str> = positions.iter().map(|position| position.selector.as_str()).collect();
    assert_eq!(selectors, ["HEAD@{0}", "HEAD@{1}", "HEAD@{2}"]);
    assert_eq!(positions[2].subject, "commit (initial): Commit 1");
    assert_eq!(self::positions(&repository, 2).len(), 2);
}

#[test]
fn reflog_read__keeps_a_subject_with_spaces_and_colons_whole() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    assert_eq!(
        positions(&repository, 1)[0].subject,
        "checkout: moving from main to feature"
    );
    assert_eq!(positions(&Repository::new(), 10), Vec::new());
}

/// The bisect of a repository.
fn bisect(repository: &Repository) -> Option<Bisect> {
    let request = BisectRequest {
        git_folder: repository.path().join(".git"),
    };
    read(&git(repository), &request).unwrap()
}

/// Five commits in a line, `A` to `E`.
fn five_commits() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let commits = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|name| repository.commit(name, &[("file.txt", name)]))
        .collect();
    (repository, commits)
}

#[test]
fn bisect_read__counts_the_commits_that_can_still_be_the_first_bad_one() {
    let (repository, commits) = five_commits();
    let _ = repository.git(&["bisect", "start"]);
    let _ = repository.git(&["bisect", "bad", &commits[4]]);
    let _ = repository.git(&["bisect", "good", &commits[0]]);
    let bisect = bisect(&repository).unwrap();
    assert_eq!(bisect.remaining, Some(4));
    assert_eq!(bisect.bad.unwrap().as_str(), commits[4]);
    assert_eq!(bisect.good.len(), 1);
}

#[test]
fn bisect_read__has_no_count_with_only_a_bad_commit() {
    let (repository, commits) = five_commits();
    let _ = repository.git(&["bisect", "start"]);
    let _ = repository.git(&["bisect", "bad", &commits[4]]);
    let bisect = bisect(&repository).unwrap();
    assert_eq!(bisect.remaining, None);
    assert_eq!(bisect.good, Vec::new());
}

#[test]
fn bisect_read__has_no_bisect_without_bisect_start_even_with_marks_left() {
    let (repository, commits) = five_commits();
    let _ = repository.git(&["update-ref", "refs/bisect/bad", &commits[4]]);
    assert_eq!(bisect(&repository), None);
}

#[test]
fn bisect_read__lists_the_skipped_commits() {
    let (repository, commits) = five_commits();
    let _ = repository.git(&["bisect", "start"]);
    let _ = repository.git(&["bisect", "bad", &commits[4]]);
    let _ = repository.git(&["bisect", "good", &commits[0]]);
    let _ = repository.git(&["bisect", "skip"]);
    assert_eq!(bisect(&repository).unwrap().skipped.len(), 1);
}
