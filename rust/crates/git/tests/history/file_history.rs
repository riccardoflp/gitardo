//! `file-history.read-names` and `file-history.read-committed-lines` against throwaway repositories.

use git::read::file_history::{CommittedLinesRequest, NamesRequest, names_arguments, read_committed_lines, read_names};
use model::file_history::{CommittedLines, LineRange};
use testkit::Repository;

use crate::{git, write};

/// The names of a file.
fn names(repository: &Repository, path: &str) -> Vec<String> {
    let request = NamesRequest { path: path.to_owned() };
    read_names(&git(repository), &request).unwrap()
}

/// Ten lines, each different enough for git to follow the file when one of them changes.
const PARSER: &str = "fn one() {}\nfn two() {}\nfn three() {}\nfn four() {}\nfn five() {}\nfn six() {}\n\
fn seven() {}\nfn eight() {}\nfn nine() {}\nfn ten() {}\n";

#[test]
fn file_history_read_names__follows_a_rename_back_to_where_the_file_was_added() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("src/a.rs", PARSER)]);
    let _ = repository.git(&["mv", "--", "src/a.rs", "src/b.rs"]);
    let _ = repository.commit("Rename the parser", &[]);
    let _ = repository.commit(
        "Change the parser",
        &[("src/b.rs", &format!("{PARSER}fn eleven() {{}}\n"))],
    );
    assert_eq!(names(&repository, "src/b.rs"), ["src/b.rs", "src/a.rs"]);
}

#[test]
fn file_history_read_names__gives_a_copy_its_own_names_only() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add x", &[("x.rs", PARSER)]);
    let _ = repository.commit("Copy x", &[("y.rs", PARSER)]);
    let _ = repository.git(&["mv", "--", "y.rs", "z.rs"]);
    let _ = repository.commit("Rename the copy", &[]);
    assert_eq!(names(&repository, "z.rs"), ["z.rs", "y.rs"]);
}

#[test]
fn file_history_read_names__gives_the_name_alone_for_a_file_never_renamed() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", PARSER)]);
    let _ = repository.commit("Change the parser", &[("parser.rs", "fn one() {}\n")]);
    assert_eq!(names(&repository, "parser.rs"), ["parser.rs"]);
    assert_eq!(names(&Repository::new(), "parser.rs"), ["parser.rs"]);
}

#[test]
fn file_history_read_names__passes_a_name_with_a_bracket_literally() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the pages",
        &[("pages/[id].tsx", "one\n"), ("pages/i.tsx", "two\n")],
    );
    let _ = repository.commit("Change a page", &[("pages/i.tsx", "three\n")]);
    let _ = repository.commit("Change the other page", &[("pages/[id].tsx", "four\n")]);
    let request = NamesRequest {
        path: String::from("pages/[id].tsx"),
    };
    assert!(
        names_arguments(&request)
            .iter()
            .any(|word| word == ":(literal)pages/[id].tsx")
    );
    assert_eq!(names(&repository, "pages/[id].tsx"), ["pages/[id].tsx"]);
}

/// Where lines of a file are in the commit checked out.
fn committed_lines(repository: &Repository, request: &CommittedLinesRequest) -> CommittedLines {
    read_committed_lines(&git(repository), request).unwrap()
}

/// A request for lines of a file, with its text as the File page has it.
fn request(path: &str, text: &str, start: usize, end: usize) -> CommittedLinesRequest {
    CommittedLinesRequest {
        path: path.to_owned(),
        renamed_from: None,
        text: text.to_owned(),
        lines: LineRange { start, end },
    }
}

#[test]
fn file_history_read_committed_lines__gives_the_same_lines_of_head_for_a_file_as_committed() {
    let mut repository = Repository::new();
    let head = repository.commit("Add the parser", &[("parser.rs", PARSER)]);
    let found = committed_lines(&repository, &request("parser.rs", PARSER, 4, 9));
    let expected = CommittedLines::Committed {
        commit: model::CommitId::parse(&head).unwrap(),
        path: String::from("parser.rs"),
        lines: Some(LineRange { start: 4, end: 9 }),
    };
    assert_eq!(found, expected);
}

#[test]
fn file_history_read_committed_lines__finds_no_commit_for_a_file_added_and_not_committed() {
    let mut repository = Repository::new();
    assert_eq!(
        committed_lines(&repository, &request("new.rs", "one\n", 1, 1)),
        CommittedLines::InNoCommit
    );
    let _ = repository.commit("Add the parser", &[("parser.rs", PARSER)]);
    write(&repository, "new.rs", "one\n");
    let _ = repository.git(&["add", "--", "new.rs"]);
    assert_eq!(
        committed_lines(&repository, &request("new.rs", "one\n", 1, 1)),
        CommittedLines::InNoCommit
    );
}

#[test]
fn file_history_read_committed_lines__looks_for_a_file_renamed_in_the_index_under_its_old_name() {
    let mut repository = Repository::new();
    let head = repository.commit("Add the parser", &[("old.rs", PARSER)]);
    let _ = repository.git(&["mv", "--", "old.rs", "new.rs"]);
    let renamed = CommittedLinesRequest {
        renamed_from: Some(String::from("old.rs")),
        ..request("new.rs", &format!("fn zero() {{}}\n{PARSER}"), 2, 3)
    };
    let expected = CommittedLines::Committed {
        commit: model::CommitId::parse(&head).unwrap(),
        path: String::from("old.rs"),
        lines: Some(LineRange { start: 1, end: 2 }),
    };
    assert_eq!(committed_lines(&repository, &renamed), expected);
}
