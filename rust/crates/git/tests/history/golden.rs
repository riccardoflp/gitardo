//! The parsers of the reads of the history on what git printed in awkward repositories, kept in
//! `tests/golden/history`: `<case>.out` is what one command printed, `<case>.expected` what its parser makes of it.
//!
//! With `GITARDO_GOLDEN_RECORD=1`, each test makes its repository, runs the command with the arguments of the read and
//! records both files again; otherwise the parsers read the files of the repository, without git.

use std::ffi::OsString;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use git::GitError;
use git::read::bisect::{marks_arguments, parse_marks};
use git::read::blame::{BlameFileRequest, BlameLineRequest, Whitespace, file_arguments, line_arguments, parse_file};
use git::read::file_history::{NamesRequest, names_arguments, parse_name_changes};
use git::read::rebase::{PushedRequest, parse_pushed, pushed_arguments};
use git::read::reflog::{ReflogRequest, parse_positions, positions_arguments};
use git::read::search::{MatchesRequest, matches_arguments, parse_matches};
use model::CommitId;
use model::search::SearchMode;
use testkit::{Remote, Repository, assert_golden};

use crate::run;

/// The file of a case, with its extension.
fn golden_file(case: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/history")
        .join(format!("{case}.{extension}"))
}

/// When recording, makes the repository of a case, runs a read's arguments there with the configuration every read
/// gets and what it reads on its standard input, and keeps what git printed in `<case>.out`, with the folder of the
/// test written `<folder>` so that the file is the same on every machine.
fn record(case: &str, repository: impl FnOnce() -> (Repository, Vec<OsString>), input: &[u8]) {
    if std::env::var_os("GITARDO_GOLDEN_RECORD").is_none_or(|value| value != "1") {
        return;
    }
    let (repository, arguments) = repository();
    let mut words = vec![String::from("-c"), String::from("core.quotepath=false")];
    words.extend(arguments.iter().map(|argument| argument.to_str().unwrap().to_owned()));
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let output = run(&repository, &words, input);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let folder = repository.path().parent().unwrap().to_string_lossy().replace('\\', "/");
    let printed = String::from_utf8(output.stdout).unwrap().replace(&folder, "<folder>");
    assert_golden(golden_file(case, "out"), printed);
}

/// Parses `<case>.out` and compares what the parser made of it with `<case>.expected`.
fn check<Parsed: Debug>(case: &str, parse: impl Fn(&[u8]) -> Result<Parsed, GitError>) {
    let output = fs::read(golden_file(case, "out")).unwrap();
    let parsed = parse(&output).unwrap();
    assert_golden(golden_file(case, "expected"), format!("{parsed:#?}\n"));
}

/// A root, a merge of two branches, a subject with an accent, brackets and a tab, and an author with an accent.
fn log_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature/é");
    repository.checkout("feature/é");
    let _ = repository.commit("Change [the] parser\twith a tab: é", &[("parser.rs", "2\n")]);
    repository.checkout("main");
    let _ = repository.commit("Add the lexer", &[("lexer.rs", "1\n")]);
    repository.merge("feature/é");
    let _ = repository.git(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Zoë's commit",
        "--author",
        "Zoë <z@example.com>",
    ]);
    repository
}

/// A file renamed to a name with an accent, with lines ending in `\r\n` and no line break at its end, changed on two
/// branches merged, and a line typed and not committed.
fn blame_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("src/a b.rs", "one\r\ntwo\r\nthree\r\nfour")]);
    repository.branch("feature");
    let _ = repository.git(&["mv", "--", "src/a b.rs", "src/parsé.rs"]);
    let _ = repository.commit("Rename the parser", &[("src/parsé.rs", "one\r\nTWO\r\nthree\r\nfour")]);
    repository.checkout("feature");
    let _ = repository.commit("Change the end", &[("src/a b.rs", "one\r\ntwo\r\nthree\r\nFOUR")]);
    repository.checkout("main");
    repository.merge("feature");
    repository
}

/// What the File page has of the file of [`blame_repository`]: its second line typed over.
const TYPED: &[u8] = b"one\r\nTWO typed\r\nthree\r\nFOUR";

#[test]
fn search_read_matches__golden_messages_found() {
    let request = MatchesRequest {
        mode: SearchMode::Message,
        text: String::from("PARSER"),
        selection: vec![OsString::from("HEAD")],
    };
    record("matches", || (log_repository(), matches_arguments(&request)), b"");
    check("matches", parse_matches);
}

#[test]
fn blame_read_file__golden_blame_of_a_renamed_file_with_crlf_a_merge_and_a_typed_line() {
    let request = BlameFileRequest {
        path: PathBuf::from("src/parsé.rs"),
        whitespace: Whitespace::Counted,
        contents: Some(TYPED.to_vec()),
        revision: None,
    };
    record("blame-file", || (blame_repository(), file_arguments(&request)), TYPED);
    check("blame-file", parse_file);
}

#[test]
fn blame_read_line__golden_blame_of_a_line_with_a_commit_before() {
    // The last line, which has no line break, so that the file of what git printed has no `\r`.
    let request = BlameLineRequest {
        path: PathBuf::from("src/parsé.rs"),
        line: 4,
        whitespace: Whitespace::IgnoredWithMoves,
        contents: None,
    };
    record("blame-line", || (blame_repository(), line_arguments(&request)), b"");
    check("blame-line", git::read::blame::parse_line);
}

#[test]
fn file_history_read_names__golden_changes_of_a_renamed_file() {
    let request = NamesRequest {
        path: String::from("src/parsé.rs"),
    };
    record("file-names", || (blame_repository(), names_arguments(&request)), b"");
    check("file-names", parse_name_changes);
}

#[test]
fn reflog_read__golden_positions_of_head() {
    let request = ReflogRequest {
        reference: String::from("HEAD"),
        count: 10,
    };
    let repository = || {
        let mut repository = log_repository();
        let _ = repository.git(&["reset", "--quiet", "--hard", "HEAD~1"]);
        let _ = repository.commit("Fix: the parser", &[("parser.rs", "3\n")]);
        (repository, positions_arguments(&request))
    };
    record("reflog", repository, b"");
    check("reflog", parse_positions);
}

#[test]
fn bisect_read__golden_marks() {
    let repository = || {
        let mut repository = Repository::new();
        let commits: Vec<String> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|name| repository.commit(name, &[("file.txt", name)]))
            .collect();
        let _ = repository.git(&["bisect", "start"]);
        let _ = repository.git(&["bisect", "bad", &commits[4]]);
        let _ = repository.git(&["bisect", "good", &commits[0]]);
        let _ = repository.git(&["bisect", "skip"]);
        (repository, marks_arguments())
    };
    record("bisect-marks", repository, b"");
    check("bisect-marks", parse_marks);
}

#[test]
fn rebase_read_pushed__golden_remote_branches_of_a_commit() {
    let repository = || {
        let mut repository = Repository::new();
        let commit = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
        let _remote = Remote::new(&repository);
        let _ = repository.git(&["remote", "set-head", "origin", "main"]);
        for branch in ["feature/é", "fix"] {
            let _ = repository.git(&["push", "--quiet", "origin", &format!("HEAD:refs/heads/{branch}")]);
        }
        let request = PushedRequest {
            commit: CommitId::parse(&commit).unwrap(),
        };
        (repository, pushed_arguments(&request))
    };
    record("pushed", repository, b"");
    check("pushed", |output| Ok::<_, GitError>(parse_pushed(output)));
}
