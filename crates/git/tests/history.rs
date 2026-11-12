//! The reads of the history against throwaway repositories: the search, the blame, the history of a file, the reflog,
//! the bisect and what rewriting commits reads; and their parsers on what git printed in awkward repositories
//! (`history/golden.rs`).

// The helpers of these tests fail as the tests do, loudly: clippy allows them what it allows tests.
#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

#[path = "history/blame.rs"]
mod blame;
#[path = "history/file_history.rs"]
mod file_history;
#[path = "history/golden.rs"]
mod golden;
#[path = "history/rebase.rs"]
mod rebase;
#[path = "history/reflog_and_bisect.rs"]
mod reflog_and_bisect;
#[path = "history/search.rs"]
mod search;

use std::fs;
use std::io::Write;
use std::process::{Output, Stdio};

use git::Git;
use testkit::Repository;

/// Git, run in a throwaway repository.
fn git(repository: &Repository) -> Git {
    Git::new(repository.path())
}

/// Writes a file of the working tree, making its folders.
fn write(repository: &Repository, name: &str, content: &str) {
    let path = repository.path().join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Runs a git command in the repository with what it reads on its standard input, and returns how it ended, whether
/// it failed or not: a rebase that stops on a conflict fails, and `Repository::git` would fail the test.
fn run(repository: &Repository, arguments: &[&str], input: &[u8]) -> Output {
    let mut child = repository
        .command("git")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

/// Makes the commits of `main` of a repository with no commits yet at once, with `git fast-import`, each with no files
/// and the message of its number, a minute apart. For a history too long to make one commit at a time.
fn make_history(repository: &Repository, count: usize, message: impl Fn(usize) -> String) {
    let commits: Vec<String> = (1..=count)
        .map(|number| {
            let message = message(number);
            let time = testkit::FIRST_COMMIT_TIME + 60 * number as u64;
            format!(
                "commit refs/heads/main\ncommitter Test Author <author@example.com> {time} +0000\ndata {}\n{message}\n",
                message.len()
            )
        })
        .collect();
    let output = run(repository, &["fast-import", "--quiet"], commits.concat().as_bytes());
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
