//! Long histories made at once with `git fast-import`: a line of commits, and a line of merges each bringing in one
//! commit of a topic branch.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::{Command, Stdio};

use testkit::{FIRST_COMMIT_TIME, Repository};

/// A commit of the stream: on a branch, with a mark, a minute after the one before, changing one file.
pub struct Commit<'a> {
    /// The branch it is made on.
    pub branch: &'a str,
    /// Its mark, from 1.
    pub mark: u64,
    /// Its first parent, when it is not the commit before on the branch.
    pub from: Option<u64>,
    /// Its second parent, for a merge.
    pub merge: Option<u64>,
    /// Its message.
    pub message: &'a str,
    /// The file it changes.
    pub file: &'a str,
}

/// Adds a commit to a stream of `git fast-import`.
pub fn write_commit(stream: &mut String, commit: &Commit<'_>) {
    let date = FIRST_COMMIT_TIME + commit.mark * 60;
    let message = format!("{}\n", commit.message);
    let content = format!("content {}\n", commit.mark);
    write!(
        stream,
        "commit refs/heads/{}\nmark :{}\ncommitter Test Author <author@example.com> {date} +0000\ndata {}\n{message}",
        commit.branch,
        commit.mark,
        message.len()
    )
    .unwrap();
    if let Some(from) = commit.from {
        writeln!(stream, "from :{from}").unwrap();
    }
    if let Some(merge) = commit.merge {
        writeln!(stream, "merge :{merge}").unwrap();
    }
    write!(
        stream,
        "M 644 inline {}\ndata {}\n{content}\n",
        commit.file,
        content.len()
    )
    .unwrap();
}

/// A line of commits on `main`, `Change 1` first.
pub fn line(count: u64) -> String {
    let mut stream = String::new();
    for mark in 1..=count {
        let message = format!("Change {mark}");
        let file = format!("src/file{}.txt", mark % 200);
        write_commit(
            &mut stream,
            &Commit {
                branch: "main",
                mark,
                from: None,
                merge: None,
                message: &message,
                file: &file,
            },
        );
    }
    stream
}

/// A commit on `main`, then rounds of a commit on `main`, one on `topic` from the merge before, and the merge of
/// `topic` into `main`: three rows a round.
pub fn merges(rounds: u64) -> String {
    let mut stream = String::new();
    let mut commit = |mark: u64, branch, (from, merge): (Option<u64>, Option<u64>), message: String| {
        write_commit(
            &mut stream,
            &Commit {
                branch,
                mark,
                from,
                merge,
                message: &message,
                file: if branch == "topic" { "topic.txt" } else { "main.txt" },
            },
        );
    };
    commit(1, "main", (None, None), String::from("Start"));
    for round in 0..rounds {
        let base = 1 + round * 3;
        commit(base + 1, "main", (None, None), format!("Main {round}"));
        commit(base + 2, "topic", (Some(base), None), format!("Topic {round}"));
        commit(base + 3, "main", (None, Some(base + 2)), format!("Merge topic {round}"));
    }
    stream
}

/// Makes the history of a stream in the repository and checks `main` out.
pub fn import(repository: &Repository, stream: &str) {
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(repository.path())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stream.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "git fast-import failed");
    let _ = repository.git(&["reset", "--quiet", "--hard", "main"]);
}

/// A repository of a history.
pub fn imported(stream: &str) -> Repository {
    let repository = Repository::new();
    import(&repository, stream);
    repository
}
