//! `search.read-matches` and `search.read-revision` against throwaway repositories.

use std::ffi::OsString;

use git::Git;
use git::read::search::{MatchesRequest, RevisionRequest, read_matches, read_revision};
use model::search::{Matches, SearchMode};
use testkit::Repository;

use crate::{git, make_history};

/// The commits a search finds in the history of every branch.
fn search(repository: &Repository, mode: SearchMode, text: &str) -> Matches {
    search_in(
        repository,
        mode,
        text,
        &["--date-order", "--branches", "--remotes", "--tags", "HEAD"],
    )
}

/// The commits a search finds in a history.
fn search_in(repository: &Repository, mode: SearchMode, text: &str, selection: &[&str]) -> Matches {
    let request = MatchesRequest {
        mode,
        text: text.to_owned(),
        selection: selection.iter().map(OsString::from).collect(),
    };
    read_matches(&git(repository), &request).unwrap()
}

/// The hashes of what a search found.
fn found(matches: &Matches) -> Vec<&str> {
    matches.commits.iter().map(model::CommitId::as_str).collect()
}

#[test]
fn search_read_matches__finds_the_text_in_messages_not_as_a_pattern() {
    let mut repository = Repository::new();
    let fix = repository.commit("Fix the parser", &[("a.rs", "1\n")]);
    let prefix = repository.commit("Prefix the names", &[("a.rs", "2\n")]);
    let dot = repository.commit("Add f.x", &[("a.rs", "3\n")]);
    assert_eq!(
        found(&search(&repository, SearchMode::Message, "fix")),
        [prefix.as_str(), fix.as_str()]
    );
    assert_eq!(found(&search(&repository, SearchMode::Message, "f.x")), [dot.as_str()]);
}

#[test]
fn search_read_matches__finds_the_authors_of_an_address() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("a.rs", "1\n")]);
    let _ = repository.git(&[
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "Elsewhere",
        "--author",
        "Bo <bo@other.org>",
    ]);
    let last = repository.commit("Fix the parser", &[("a.rs", "2\n")]);
    let matches = search(&repository, SearchMode::Author, "@EXAMPLE.com");
    assert_eq!(found(&matches), [last.as_str(), first.as_str()]);
}

#[test]
fn search_read_matches__finds_the_commits_that_add_or_remove_text_not_those_that_move_it() {
    let mut repository = Repository::new();
    let added = repository.commit("Add", &[("log.rs", "fn parseLog() {}\nfn other() {}\n")]);
    let _moved = repository.commit("Move", &[("log.rs", "fn other() {}\nfn parseLog() {}\n")]);
    let removed = repository.commit("Remove", &[("log.rs", "fn other() {}\n")]);
    let matches = search(&repository, SearchMode::Changes, "parseLog");
    assert_eq!(found(&matches), [removed.as_str(), added.as_str()]);
}

#[test]
fn search_read_matches__finds_changed_lines_by_an_extended_regular_expression() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the words", &[("errors.txt", "error ab\n")]);
    let number = repository.commit("Add the number", &[("numbers.txt", "error 404\n")]);
    let matches = search(&repository, SearchMode::ChangesPattern, "[0-9]{3}");
    assert_eq!(found(&matches), [number.as_str()]);
}

#[test]
fn search_read_matches__refuses_an_invalid_regular_expression_with_gits_error() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("a.rs", "1\n")]);
    let request = MatchesRequest {
        mode: SearchMode::ChangesPattern,
        text: String::from("("),
        selection: vec![OsString::from("HEAD")],
    };
    let error = read_matches(&git(&repository), &request).unwrap_err();
    assert!(error.stderr().unwrap().contains("invalid regex"), "{error}");
}

#[test]
fn search_read_matches__keeps_five_thousand_and_says_there_are_more() {
    let repository = Repository::new();
    make_history(&repository, 6_000, |number| format!("fix {number}"));
    let matches = search(&repository, SearchMode::Message, "fix");
    assert_eq!(matches.commits.len(), 5_000);
    assert!(matches.has_more);
}

#[test]
fn search_read_matches__finds_only_in_the_branches_shown() {
    let mut repository = Repository::new();
    let on_main = repository.commit("Fix the parser", &[("a.rs", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit("Fix on the feature", &[("b.rs", "1\n")]);
    let shown = ["--date-order", "--ignore-missing", "refs/heads/main"];
    let matches = search_in(&repository, SearchMode::Message, "fix", &shown);
    assert_eq!(found(&matches), [on_main.as_str()]);
}

/// The commit a name stands for.
fn revision(repository: &Repository, name: &str) -> Option<String> {
    let request = RevisionRequest {
        revision: name.to_owned(),
    };
    read_revision(&git(repository), &request)
        .unwrap()
        .map(|commit| commit.as_str().to_owned())
}

/// Five commits, `v1.0` on the fourth.
fn five_commits() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let mut commits = Vec::new();
    for number in 1..=5 {
        commits.push(repository.commit(&format!("Commit {number}"), &[("file.txt", &format!("{number}\n"))]));
        if number == 4 {
            repository.tag("v1.0");
        }
    }
    (repository, commits)
}

#[test]
fn search_read_revision__gives_the_commit_of_each_kind_of_name() {
    let (repository, commits) = five_commits();
    assert_eq!(revision(&repository, "HEAD~3").as_ref(), commits.get(1));
    assert_eq!(revision(&repository, "v1.0^").as_ref(), commits.get(2));
    assert_eq!(revision(&repository, "main@{yesterday}").as_ref(), commits.get(4));
    assert_eq!(revision(&repository, &commits[0]).as_ref(), commits.first());
}

#[test]
fn search_read_revision__gives_the_commit_of_an_annotated_tag() {
    let (repository, commits) = five_commits();
    let _ = repository.git(&["tag", "--annotate", "v2.0", "--message", "The second release"]);
    assert_eq!(revision(&repository, "v2.0").as_ref(), commits.get(4));
}

#[test]
fn search_read_revision__gives_none_for_an_option_and_runs_nothing() {
    // Git run in a folder that does not exist fails at once: an answer means that nothing ran.
    let git = Git::new("this folder does not exist");
    let request = RevisionRequest {
        revision: String::from("--all"),
    };
    assert_eq!(read_revision(&git, &request).unwrap(), None);
}

#[test]
fn search_read_revision__gives_none_for_a_blob() {
    let (repository, _) = five_commits();
    let blob = repository.git(&["rev-parse", "HEAD:file.txt"]);
    assert_eq!(revision(&repository, blob.trim_end()), None);
    assert_eq!(revision(&repository, "nope"), None);
}
