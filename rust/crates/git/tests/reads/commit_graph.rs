//! `upkeep.read-commit-graph` against throwaway repositories: a history made by `git fast-import`, which writes no
//! commit-graph file, then the file written alone and as a chain, git told not to read it, and a worktree.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::Stdio;

use git::Git;
use git::read::upkeep::{CommitGraphRequest, read_commit_graph, read_commit_graph_kept};
use model::upkeep::CommitGraph;
use testkit::Repository;

use crate::beside;

/// A repository whose history of three commits is made by `git fast-import`, as a large one is in the tests of speed.
fn imported() -> Repository {
    let repository = Repository::new();
    let mut stream = String::new();
    for mark in 1..=3 {
        let message = format!("Change {mark}\n");
        let content = format!("content {mark}\n");
        write!(
            stream,
            "commit refs/heads/main\nmark :{mark}\ncommitter Test Author <author@example.com> {} +0000\ndata {}\n{message}\
             M 644 inline file.txt\ndata {}\n{content}\n",
            1_735_689_600 + mark * 60,
            message.len(),
            content.len()
        )
        .unwrap();
    }
    let mut child = repository
        .command("git")
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stream.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "git fast-import failed");
    let _ = repository.git(&["reset", "--quiet", "--hard", "main"]);
    repository
}

/// What the read finds in a folder.
fn read_in(folder: &std::path::Path) -> CommitGraph {
    let request = CommitGraphRequest {
        folder: folder.to_path_buf(),
        before: None,
    };
    read_commit_graph(&Git::new(folder), &request).unwrap()
}

#[test]
fn upkeep_read_commit_graph__a_history_imported_has_no_file_and_has_one_once_written_alone_or_as_a_chain() {
    let repository = imported();
    let folder = repository.path();
    let missing = CommitGraph {
        has_file: false,
        is_ignored: false,
    };
    let written = CommitGraph {
        has_file: true,
        is_ignored: false,
    };
    let request = CommitGraphRequest {
        folder: folder.to_path_buf(),
        before: None,
    };
    let first = read_commit_graph_kept(&Git::new(folder), &request).unwrap();
    assert_eq!(first.graph, missing);
    let _ = repository.git(&["commit-graph", "write", "--reachable"]);
    // The read kept from the one before sees the object folder changed, and reads again.
    let kept = CommitGraphRequest {
        before: Some(first),
        ..request
    };
    assert_eq!(read_commit_graph_kept(&Git::new(folder), &kept).unwrap().graph, written);
    let chained = imported();
    let _ = chained.git(&["commit-graph", "write", "--reachable", "--split"]);
    assert!(
        chained.path().join(".git/objects/info/commit-graphs").is_dir(),
        "git wrote a chain"
    );
    assert_eq!(read_in(chained.path()), written);
}

#[test]
fn upkeep_read_commit_graph__with_core_commit_graph_false_it_reads_as_not_read_by_git_file_or_not() {
    let repository = imported();
    let _ = repository.git(&["config", "core.commitGraph", "false"]);
    assert_eq!(
        read_in(repository.path()),
        CommitGraph {
            has_file: false,
            is_ignored: true,
        }
    );
    let _ = repository.git(&["commit-graph", "write", "--reachable"]);
    let read = read_in(repository.path());
    assert!(read.is_ignored && !read.is_missing(), "{read:?}");
}

#[test]
fn upkeep_read_commit_graph__in_a_worktree_it_reads_the_file_of_the_repository_the_worktrees_share() {
    let repository = imported();
    let _ = repository.git(&["worktree", "add", "--quiet", "../other", "-b", "other"]);
    let other = beside(&repository).join("other");
    assert!(read_in(&other).is_missing());
    let _ = repository.git(&["commit-graph", "write", "--reachable"]);
    assert!(read_in(&other).has_file);
}
