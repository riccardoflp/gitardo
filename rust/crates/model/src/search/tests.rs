#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{BoxMode, Direction, hash_prefix, loaded, step};
use crate::commit::Commit;
use crate::graph::Row;
use crate::{CommitId, Timestamp};

/// Fourteen matches, and a commit that is none of them.
fn matches() -> (Vec<CommitId>, CommitId) {
    let commit = |number: usize| CommitId::parse(&format!("{number:040x}")).unwrap();
    ((1..=14).map(commit).collect(), commit(99))
}

/// A commit row with a hash, a subject and an author.
fn row(hash: &str, subject: &str, author: &str) -> Row {
    Row::Commit(Commit {
        id: CommitId::parse(&format!("{hash:0<40}")).unwrap(),
        parents: Vec::new(),
        author_name: author.to_owned(),
        author_email: String::from("someone@example.com"),
        author_date: Timestamp(0),
        subject: subject.to_owned(),
    })
}

/// The subjects of what a search of the rows loaded found.
fn found(rows: &[Row], text: &str) -> Vec<String> {
    let ids = loaded(rows, text);
    rows.iter()
        .filter_map(Row::commit)
        .filter(|commit| ids.contains(&commit.id))
        .map(|commit| commit.subject.clone())
        .collect()
}

#[test]
fn search_loaded__finds_the_text_in_subjects_and_authors_without_regard_to_case() {
    let rows = [
        row("1111", "Fix the parser", "Dana Ruiz"),
        row("2222", "Add the readme", "Parser Bot"),
        row("3333", "Add the rail", "Dana Ruiz"),
    ];
    assert_eq!(found(&rows, "  PARSER "), ["Fix the parser", "Add the readme"]);
    assert_eq!(found(&rows, ""), Vec::<String>::new());
    assert_eq!(found(&rows, "   "), Vec::<String>::new());
}

#[test]
fn search_loaded__finds_a_hash_from_its_fourth_character() {
    let rows = [
        row("3f2a1c9", "Read the settings", "Dana Ruiz"),
        row("abcdef", "Mention 3f2 in the message", "Dana Ruiz"),
    ];
    assert_eq!(found(&rows, "3f2a"), ["Read the settings"]);
    assert_eq!(found(&rows, "3F2A"), ["Read the settings"]);
    assert_eq!(found(&rows, "3f2"), ["Mention 3f2 in the message"]);
}

#[test]
fn search_loaded__finds_a_word_of_hexadecimal_digits_by_its_hash_only_from_four() {
    let rows = [
        row("add1", "Remove the parser", "Dana Ruiz"),
        row("4444", "Add the rail", "Dana Ruiz"),
    ];
    assert_eq!(found(&rows, "add"), ["Add the rail"]);
    assert_eq!(found(&rows, "add1"), ["Remove the parser"]);
}

#[test]
fn search_loaded__does_not_find_the_draft_of_the_uncommitted_changes() {
    let mut stash = row("5555", "On main: Uncommitted work", "Dana Ruiz");
    if let Row::Commit(commit) = stash {
        stash = Row::Stash { commit, index: 0 };
    }
    let rows = [
        Row::Uncommitted { parent: None, files: 3 },
        row("6666", "Add the rail", "Dana Ruiz"),
        stash,
    ];
    assert_eq!(found(&rows, "Uncommitted"), ["On main: Uncommitted work"]);
}

#[test]
fn the_modes_of_the_box_are_in_the_order_of_the_list_with_the_loaded_commits_first() {
    assert_eq!(BoxMode::ALL.first(), Some(&BoxMode::default()));
    assert_eq!(BoxMode::Loaded.history(), None);
    assert_eq!(hash_prefix(" ABCDEF12 ").as_deref(), Some("abcdef12"));
    assert_eq!(hash_prefix("abc"), None);
    assert_eq!(hash_prefix("abcg"), None);
}

#[test]
fn search_step__goes_from_the_last_to_the_first() {
    let (matches, _) = matches();
    assert_eq!(
        step(&matches, matches.get(13), Direction::Next),
        matches.first().cloned()
    );
    assert_eq!(step(&matches, matches.get(4), Direction::Next), matches.get(5).cloned());
}

#[test]
fn search_step__goes_from_the_first_to_the_last() {
    let (matches, _) = matches();
    assert_eq!(
        step(&matches, matches.first(), Direction::Previous),
        matches.get(13).cloned()
    );
}

#[test]
fn search_step__starts_at_an_end_when_the_selected_commit_is_no_match() {
    let (matches, other) = matches();
    assert_eq!(
        step(&matches, Some(&other), Direction::Previous),
        matches.get(13).cloned()
    );
    assert_eq!(step(&matches, Some(&other), Direction::Next), matches.first().cloned());
    assert_eq!(step(&matches, None, Direction::Next), matches.first().cloned());
    assert_eq!(step(&[], Some(&other), Direction::Next), None);
}
