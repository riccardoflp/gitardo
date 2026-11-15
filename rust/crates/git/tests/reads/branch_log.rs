//! `branch.read-log`, `repo.common-folder` and `commit.prepared-message` against throwaway repositories: the reads
//! the extension did in TypeScript that became cards of the engine in stage 5 of the transition (Q397, Q398).

use std::fs;
use std::path::PathBuf;

use git::Git;
use git::read::branch::{BranchLogRequest, read_log};
use git::read::commit::{PreparedMessageRequest, read_prepared_message};
use git::read::repo::{CommonFolderRequest, read_common_folder};
use testkit::{Remote, Repository};

use crate::{beside, run, write};

/// The subjects of the commits listed.
fn subjects(log: &model::commit::BranchLog) -> Vec<&str> {
    log.commits.iter().map(|commit| commit.subject.as_str()).collect()
}

#[test]
fn branch_read_log__the_newest_commits_of_head_first_and_whether_there_are_more() {
    let mut repository = Repository::new();
    for subject in ["First", "Second", "Third"] {
        let _ = repository.commit(subject, &[("a.txt", subject)]);
    }
    repository.branch("other");
    repository.checkout("other");
    let _ = repository.commit("Only on other", &[("b.txt", "b")]);
    repository.checkout("main");
    let log = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 2 }).unwrap();
    assert_eq!(subjects(&log), ["Third", "Second"]);
    assert!(log.has_more);
    let all = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 10 }).unwrap();
    assert_eq!(subjects(&all), ["Third", "Second", "First"]);
    assert!(!all.has_more);
}

#[test]
fn branch_read_log__before_the_first_commit_nothing_is_listed_and_nothing_fails() {
    let repository = Repository::new();
    let log = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 5 }).unwrap();
    assert_eq!(log.commits, []);
    assert_eq!((log.has_more, log.unpushed, log.upstream), (false, None, None));
}

#[test]
fn branch_read_log__the_commits_not_in_the_upstream_are_unpushed_and_the_upstream_is_named_short() {
    let mut repository = Repository::new();
    let _ = repository.commit("Pushed", &[("a.txt", "1")]);
    let _remote = Remote::new(&repository);
    let local = repository.commit("Not pushed", &[("a.txt", "2")]);
    let log = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 5 }).unwrap();
    assert_eq!(
        log.unpushed
            .unwrap()
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<Vec<_>>(),
        [local]
    );
    assert_eq!(log.upstream.as_deref(), Some("origin/main"));
}

#[test]
fn branch_read_log__without_an_upstream_the_commits_in_no_remote_branch_are_unpushed() {
    let mut repository = Repository::new();
    let _ = repository.commit("Pushed", &[("a.txt", "1")]);
    let _remote = Remote::new(&repository);
    repository.branch("topic");
    repository.checkout("topic");
    let topic = repository.commit("On topic only", &[("b.txt", "b")]);
    let log = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 5 }).unwrap();
    assert_eq!(
        log.unpushed
            .unwrap()
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<Vec<_>>(),
        [topic]
    );
    assert_eq!(log.upstream, None);
}

#[test]
fn branch_read_log__with_no_remote_nothing_is_said_of_what_is_pushed() {
    let mut repository = Repository::new();
    let _ = repository.commit("Local", &[("a.txt", "1")]);
    let log = read_log(&Git::new(repository.path()), &BranchLogRequest { count: 5 }).unwrap();
    assert_eq!(subjects(&log), ["Local"]);
    assert_eq!((log.unpushed, log.upstream), (None, None));
}

/// A repository with one commit and a linked worktree of a new branch beside it, and the folder of that worktree.
fn with_worktree() -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _ = repository.commit("First", &[("a.txt", "1")]);
    let _ = repository.git(&["worktree", "add", "--quiet", "-b", "side", "../side"]);
    let worktree = fs::canonicalize(beside(&repository).join("side")).unwrap();
    (repository, worktree)
}

/// A path made whole by the system, without the `\\?\` Windows puts before it, to compare folders.
fn whole(path: &std::path::Path) -> PathBuf {
    let path = fs::canonicalize(path).unwrap();
    PathBuf::from(path.to_string_lossy().trim_start_matches(r"\\?\"))
}

#[test]
fn repo_common_folder__in_the_main_worktree_it_is_its_dot_git() {
    let (repository, _worktree) = with_worktree();
    let folder = read_common_folder(&Git::new(repository.path()), &CommonFolderRequest).unwrap();
    assert_eq!(whole(&folder), whole(&repository.path().join(".git")));
}

#[test]
fn repo_common_folder__in_a_linked_worktree_it_is_the_dot_git_of_the_main_worktree() {
    let (repository, worktree) = with_worktree();
    let folder = read_common_folder(&Git::new(&worktree), &CommonFolderRequest).unwrap();
    assert_eq!(whole(&folder), whole(&repository.path().join(".git")));
}

#[test]
fn repo_common_folder__from_a_folder_inside_the_working_tree_it_is_the_same() {
    let (repository, _worktree) = with_worktree();
    write(&repository, "src/deep/b.txt", "b\n");
    let folder = read_common_folder(&Git::new(repository.path().join("src/deep")), &CommonFolderRequest).unwrap();
    assert!(folder.is_absolute(), "{}", folder.display());
    assert_eq!(whole(&folder), whole(&repository.path().join(".git")));
}

/// A repository whose branch `topic` changes `a.txt`, squash-merged into `main` and not committed yet.
fn squash_waiting() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("First", &[("a.txt", "1\n")]);
    repository.branch("topic");
    repository.checkout("topic");
    let _ = repository.commit("Change a on topic", &[("a.txt", "2\n")]);
    repository.checkout("main");
    let merged = run(&repository, &["merge", "--squash", "topic"]);
    assert!(merged.status.success(), "{merged:?}");
    repository
}

#[test]
fn commit_prepared_message__after_a_squash_merge_the_message_git_wrote_without_its_end() {
    let repository = squash_waiting();
    let message = read_prepared_message(&Git::new(repository.path()), &PreparedMessageRequest)
        .unwrap()
        .unwrap();
    assert!(message.starts_with("Squashed commit of the following:"), "{message}");
    assert!(message.contains("Change a on topic"), "{message}");
    assert_eq!(message, message.trim_end());
}

#[test]
fn commit_prepared_message__with_no_squash_waiting_there_is_none() {
    let mut repository = Repository::new();
    let _ = repository.commit("First", &[("a.txt", "1\n")]);
    assert_eq!(
        read_prepared_message(&Git::new(repository.path()), &PreparedMessageRequest).unwrap(),
        None
    );
}

#[test]
fn commit_prepared_message__a_linked_worktree_has_its_own() {
    let repository = squash_waiting();
    let _ = repository.git(&["worktree", "add", "--quiet", "-b", "side", "../side", "HEAD"]);
    let side = beside(&repository).join("side");
    assert_eq!(
        read_prepared_message(&Git::new(&side), &PreparedMessageRequest).unwrap(),
        None
    );
    assert!(
        read_prepared_message(&Git::new(repository.path()), &PreparedMessageRequest)
            .unwrap()
            .is_some()
    );
}

#[test]
fn commit_prepared_message__an_empty_message_is_none() {
    let repository = squash_waiting();
    fs::write(repository.path().join(".git").join("SQUASH_MSG"), "\n\n").unwrap();
    assert_eq!(
        read_prepared_message(&Git::new(repository.path()), &PreparedMessageRequest).unwrap(),
        None
    );
}
