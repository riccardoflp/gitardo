//! `blame.read-line` and `blame.read-file` against throwaway repositories.

use std::path::PathBuf;

use git::read::blame::{BlameFileRequest, BlameLineRequest, Whitespace, read_file, read_line};
use model::Timestamp;
use model::blame::{BlameCommit, BlameLine};
use testkit::{FIRST_COMMIT_TIME, Repository};

use crate::{git, write};

/// The blame of one line of a file, with the text of the File page when it has edits not saved.
fn line(repository: &Repository, path: &str, line: usize, contents: Option<&str>) -> Option<BlameLine> {
    let request = BlameLineRequest {
        path: PathBuf::from(path),
        line,
        whitespace: Whitespace::Counted,
        contents: contents.map(|text| text.as_bytes().to_vec()),
    };
    read_line(&git(repository), &request).unwrap()
}

/// The blame of every line of a file as it is on disk, or as of a commit.
fn file(
    repository: &Repository,
    path: &str,
    whitespace: Whitespace,
    revision: Option<&str>,
) -> Option<Vec<Option<BlameLine>>> {
    let request = BlameFileRequest {
        path: PathBuf::from(path),
        whitespace,
        contents: None,
        revision: revision.map(str::to_owned),
    };
    read_file(&git(repository), &request).unwrap()
}

/// The commit of a line, when it has one.
fn commit(line: Option<&BlameLine>) -> &BlameCommit {
    match line {
        Some(BlameLine::Committed(commit)) => commit,
        other => panic!("not a committed line: {other:?}"),
    }
}

/// A file added by a first commit, its second line changed by a second.
fn two_commits() -> (Repository, String, String) {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "one\ntwo\nthree\n")]);
    let second = repository.commit("Fix the parser", &[("parser.rs", "one\nTWO\nthree\n")]);
    (repository, first, second)
}

#[test]
fn blame_read_line__gives_the_hash_author_date_and_summary_of_the_commit() {
    let (repository, _, second) = two_commits();
    let blamed = line(&repository, "parser.rs", 2, None);
    let commit = commit(blamed.as_ref());
    assert_eq!(commit.id.as_str(), second);
    assert_eq!(commit.author, "Test Author");
    assert_eq!(
        commit.author_time,
        Timestamp(i64::try_from(FIRST_COMMIT_TIME).unwrap() + 60)
    );
    assert_eq!(commit.summary, "Fix the parser");
}

#[test]
fn blame_read_line__gives_a_line_not_committed_for_a_hash_of_zeros() {
    let (repository, _, _) = two_commits();
    let typed = "one\nTWO typed\nthree\n";
    assert_eq!(
        line(&repository, "parser.rs", 2, Some(typed)),
        Some(BlameLine::NotCommitted)
    );
    write(&repository, "parser.rs", "one\nTWO\nthree\nfour\n");
    assert_eq!(line(&repository, "parser.rs", 4, None), Some(BlameLine::NotCommitted));
}

#[test]
fn blame_read_line__gives_nothing_when_git_fails() {
    let (repository, _, _) = two_commits();
    write(&repository, "untracked.rs", "one\n");
    assert_eq!(line(&repository, "untracked.rs", 1, None), None);
    assert_eq!(line(&repository, "parser.rs", 9, None), None);
}

#[test]
fn blame_read_file__gives_each_line_its_commit_and_the_commit_before() {
    let (repository, first, second) = two_commits();
    let lines = file(&repository, "parser.rs", Whitespace::Counted, None).unwrap();
    let ids: Vec<&str> = lines.iter().map(|line| commit(line.as_ref()).id.as_str()).collect();
    assert_eq!(ids, [first.as_str(), second.as_str(), first.as_str()]);
    let previous = commit(lines[1].as_ref()).previous.as_ref().unwrap();
    assert_eq!(
        (previous.commit.as_str(), previous.path.as_str()),
        (first.as_str(), "parser.rs")
    );
    assert_eq!(commit(lines[0].as_ref()).previous, None);
}

#[test]
fn blame_read_file__gives_the_details_of_a_commit_to_every_line_of_it() {
    let (repository, _, _) = two_commits();
    let lines = file(&repository, "parser.rs", Whitespace::Counted, None).unwrap();
    assert_eq!(commit(lines[0].as_ref()), commit(lines[2].as_ref()));
    assert_eq!(commit(lines[2].as_ref()).summary, "Add the parser");
    assert_eq!(commit(lines[2].as_ref()).author, "Test Author");
}

#[test]
fn blame_read_file__gives_no_blame_for_an_untracked_file() {
    let (repository, _, _) = two_commits();
    write(&repository, "untracked.rs", "one\n");
    assert_eq!(file(&repository, "untracked.rs", Whitespace::Counted, None), None);
}

#[test]
fn blame_read_file__blames_the_file_as_of_a_commit() {
    let (repository, first, _) = two_commits();
    let lines = file(&repository, "parser.rs", Whitespace::Counted, Some(&first)).unwrap();
    assert!(lines.iter().all(|line| commit(line.as_ref()).id.as_str() == first));
}

#[test]
fn blame_read_file__leaves_a_line_reindented_to_who_wrote_it_when_whitespace_is_ignored() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "fn parse() {\nlet x = 1;\n}\n")]);
    let second = repository.commit(
        "Indent the parser",
        &[("parser.rs", "fn parse() {\n    let x = 1;\n}\n")],
    );
    let counted = file(&repository, "parser.rs", Whitespace::Counted, None).unwrap();
    assert_eq!(commit(counted[1].as_ref()).id.as_str(), second);
    let ignored = file(&repository, "parser.rs", Whitespace::IgnoredWithMoves, None).unwrap();
    assert_eq!(commit(ignored[1].as_ref()).id.as_str(), first);
}
