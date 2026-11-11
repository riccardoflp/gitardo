//! How long `repo.read-state` takes on a large repository: 50,000 commits over 1,000 files, 100 branches and 100 tags,
//! and a branch ahead of its upstream. Not run by default; the exit criterion of milestone M1 is under one second:
//!
//! ```text
//! cargo test -p git --release --test repo_read_state -- --ignored --nocapture
//! ```

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::Instant;

use testkit::{FIRST_COMMIT_TIME, Remote, Repository};

use crate::read;

/// How many commits the repository has.
const COMMITS: u64 = 50_000;
/// How many files the commits change, one each, in turn.
const FILES: u64 = 1_000;
/// How many branches and how many tags, spread over the history.
const REFERENCES: u64 = 100;

/// The stream `git fast-import` reads to make the history: one commit a minute on `main`, each changing one file, and
/// the branches and tags at commits spread over it.
fn history() -> String {
    let mut stream = String::new();
    for number in 1..=COMMITS {
        let content = format!("content {number}\n");
        let message = format!("Change {number}\n");
        let date = FIRST_COMMIT_TIME + number * 60;
        let file = number % FILES;
        write!(
            stream,
            "commit refs/heads/main\nmark :{number}\ncommitter Test Author <author@example.com> {date} +0000\n\
             data {}\n{message}M 644 inline src/file{file}.txt\ndata {}\n{content}\n",
            message.len(),
            content.len(),
        )
        .unwrap();
    }
    for number in 1..=REFERENCES {
        let mark = number * (COMMITS / REFERENCES);
        write!(stream, "reset refs/heads/branch-{number}\nfrom :{mark}\n\n").unwrap();
        write!(stream, "reset refs/tags/v{number}\nfrom :{mark}\n\n").unwrap();
    }
    stream
}

/// Feeds the history to `git fast-import` in the repository, and checks out what it made.
fn import(repository: &Repository) {
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(repository.path())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(history().as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "git fast-import failed");
    let _ = repository.git(&["reset", "--quiet", "--hard", "main"]);
}

#[test]
#[ignore = "makes a repository of 50,000 commits; run by hand to measure"]
fn repo_read_state__speed_on_50000_commits() {
    let mut repository = Repository::new();
    let made = Instant::now();
    import(&repository);
    let _remote = Remote::new(&repository);
    for number in 0..3 {
        let _ = repository.commit("Change here", &[("local.txt", &format!("{number}\n"))]);
    }
    println!("made the repository in {:?}", made.elapsed());
    for attempt in 1..=5 {
        let started = Instant::now();
        let state = read(&repository);
        let elapsed = started.elapsed();
        assert_eq!(state.branches.len(), 101);
        assert_eq!(state.tags.len(), 100);
        println!("read {attempt}: {elapsed:?}");
    }
}
