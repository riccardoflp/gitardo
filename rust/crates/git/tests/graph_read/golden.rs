//! The parsers of `graph.read-commits` and `graph.read-position` on what git printed in awkward repositories, kept in
//! `tests/golden/graph`: `<case>.out` is what one command printed, `<case>.expected` what its parser makes of it.
//!
//! With `GITARDO_GOLDEN_RECORD=1`, each test makes its repository, runs the command with the arguments of the read and
//! records both files again; otherwise the parsers read the files of the repository, without git.

use std::ffi::OsString;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use git::GitError;
use git::read::graph::{
    CommitsRequest, History, Parents, PositionRequest, commits_arguments, parse_position, parse_stash_commits,
    position_arguments, stashes_arguments,
};
use git::read::log_records::parse_commits;
use git::read::repo::{parse_stashes, stashes_arguments as stashes_list_arguments};
use model::CommitId;
use model::graph::PageSize;
use model::state::Stash;
use testkit::{Repository, assert_golden};

/// The file of a case, with its extension.
fn golden_file(case: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/graph")
        .join(format!("{case}.{extension}"))
}

/// Whether the golden files are being recorded again.
fn is_recording() -> bool {
    std::env::var_os("GITARDO_GOLDEN_RECORD").is_some_and(|value| value == "1")
}

/// When recording, runs a read's arguments in a repository with the configuration every read gets, keeps what git
/// printed in `<case>.out`, and returns it.
fn record(case: &str, repository: &Repository, arguments: &[OsString]) -> String {
    let mut words = vec!["-c", "core.quotepath=false"];
    words.extend(arguments.iter().map(|argument| argument.to_str().unwrap()));
    let printed = repository.git(&words);
    assert_golden(golden_file(case, "out"), &printed);
    printed
}

/// Parses `<case>.out` and compares what the parser made of it with `<case>.expected`.
fn check<Parsed: Debug>(case: &str, parse: impl Fn(&[u8]) -> Result<Parsed, GitError>) {
    let output = fs::read(golden_file(case, "out")).unwrap();
    let parsed = parse(&output).unwrap();
    assert_golden(golden_file(case, "expected"), format!("{parsed:#?}\n"));
}

/// The history of everything, as `filter.log-arguments` gives it.
fn everything() -> History {
    History {
        selection: ["--date-order", "--branches", "--remotes", "--tags", "HEAD"]
            .iter()
            .map(OsString::from)
            .collect(),
        narrowing: Vec::new(),
        parents: Parents::All,
    }
}

/// Two root commits, a merge of three parents, and messages with a tab, accents, a body, brackets and a leading dash.
fn awkward_repository() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    for name in ["left", "right"] {
        repository.branch(name);
        repository.checkout(name);
        let message = format!("Change\ton {name}: café [draft]\n\nThe body\nof two lines");
        let _ = repository.commit(&message, &[(&format!("{name}.txt"), "1\n")]);
        repository.checkout("main");
    }
    let _ = repository.commit("-leading dash, 解析", &[("main.txt", "1\n")]);
    let _ = repository.git(&["merge", "--quiet", "--no-ff", "--no-edit", "left", "right"]);
    let _ = repository.git(&["checkout", "--quiet", "--orphan", "other-root"]);
    let _ = repository.commit("Start again", &[("other.txt", "1\n")]);
    repository.checkout("main");
    repository
}

/// A request of one page of everything.
fn page_of_everything() -> CommitsRequest {
    CommitsRequest {
        history: everything(),
        page_size: PageSize::DEFAULT,
        count: 0,
        skip: 0,
        stashes: Vec::new(),
    }
    .with_pages(1)
}

#[test]
fn graph_read_commits__golden_roots_a_merge_of_three_parents_and_awkward_messages() {
    if is_recording() {
        let _ = record(
            "commits-awkward",
            &awkward_repository(),
            &commits_arguments(&page_of_everything()),
        );
    }
    check("commits-awkward", parse_commits);
}

/// The stashes of the stash repository, as `repo.read-state` reads them.
fn stash_list() -> Vec<Stash> {
    parse_stashes(&fs::read(golden_file("stash-list", "out")).unwrap()).unwrap()
}

#[test]
fn graph_read_commits__golden_stashes_one_with_untracked_files() {
    if is_recording() {
        let mut repository = Repository::new();
        let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
        fs::write(repository.path().join("parser.rs"), "2\n").unwrap();
        let _ = repository.git(&["stash", "push", "--quiet", "--message", "before the merge: [draft] é"]);
        fs::write(repository.path().join("-new.txt"), "new\n").unwrap();
        let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked"]);
        let list = record("stash-list", &repository, &stashes_list_arguments());
        let stashes = parse_stashes(list.as_bytes()).unwrap();
        let _ = record("stashes", &repository, &stashes_arguments(&stashes));
    }
    let stashes = stash_list();
    check("stashes", |output| parse_stash_commits(output, &stashes));
}

#[test]
fn graph_read_position__golden_the_line_of_a_hash() {
    let commit = CommitId::parse("0000000000000000000000000000000000000000").unwrap();
    let request = PositionRequest {
        history: everything(),
        commit,
    };
    if is_recording() {
        let _ = record("position", &awkward_repository(), &position_arguments(&request));
    }
    let output = fs::read(golden_file("position", "out")).unwrap();
    let text = String::from_utf8(output.clone()).unwrap();
    let third = CommitId::parse(text.lines().nth(2).unwrap()).unwrap();
    assert_eq!(parse_position(&output, &third), Some(2));
    assert_eq!(parse_position(&output, &request.commit), None);
}
