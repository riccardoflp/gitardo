//! `commits.read-log` against throwaway repositories, with and without a remote and an upstream.

use git::read::commits::{LogRequest, read_log};
use model::commits::BranchLog;
use testkit::{Remote, Repository};

use crate::{git, make_history};

/// The first hundred commits of `HEAD`.
fn read(repository: &Repository) -> BranchLog {
    read_log(&git(repository), &LogRequest { limit: 100 }).unwrap()
}

/// The hashes of the commits marked not pushed.
fn unpushed(log: &BranchLog) -> Vec<&str> {
    log.unpushed.iter().map(model::CommitId::as_str).collect()
}

#[test]
fn commits_read_log__returns_no_commits_and_no_error_before_the_first_commit() {
    let repository = Repository::new();
    let log = read(&repository);
    assert_eq!(log.commits, Vec::new());
    assert!(!log.has_more);
    assert_eq!(log.unpushed, Vec::new());
}

#[test]
fn commits_read_log__marks_the_commits_not_in_the_upstream() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    let second = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    let third = repository.commit("Test the parser", &[("parser.rs", "3\n")]);
    let log = read(&repository);
    assert_eq!(log.upstream.as_deref(), Some("refs/remotes/origin/main"));
    assert_eq!(log.commits.len(), 3);
    assert_eq!(unpushed(&log), [third.as_str(), second.as_str()]);
}

#[test]
fn commits_read_log__marks_the_commits_in_no_remote_branch_without_an_upstream() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _remote = Remote::new(&repository);
    repository.branch("feature");
    repository.checkout("feature");
    let feature = repository.commit("Start the feature", &[("feature.rs", "1\n")]);
    let log = read(&repository);
    assert_eq!(log.upstream, None);
    assert!(log.has_remote);
    assert_eq!(unpushed(&log), [feature.as_str()]);
}

#[test]
fn commits_read_log__marks_nothing_in_a_repository_with_no_remote() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let _ = repository.commit("Fix the parser", &[("parser.rs", "2\n")]);
    let log = read(&repository);
    assert_eq!(log.commits.len(), 2);
    assert!(!log.has_remote);
    assert_eq!(log.unpushed, Vec::new());
}

#[test]
fn commits_read_log__lists_as_many_as_asked_and_says_there_are_more() {
    let repository = Repository::new();
    make_history(&repository, 150, |number| format!("Commit {number}"));
    let log = read(&repository);
    assert_eq!(log.commits.len(), 100);
    assert!(log.has_more);
    assert_eq!(log.commits[0].subject, "Commit 150");
    let all = read_log(&git(&repository), &LogRequest { limit: 200 }).unwrap();
    assert_eq!((all.commits.len(), all.has_more), (150, false));
}
