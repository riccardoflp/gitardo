#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::{Path, PathBuf};

use model::file_search::{Mode, Place, Scope, Search};

use super::{keep_search, search_of};
use crate::persisted::Persisted;

/// The file written with some searches kept, read back.
fn read_back(persisted: &Persisted) -> Persisted {
    Persisted::from_text(&persisted.to_text())
}

#[test]
fn file_search_remember__socket_in_text_with_match_case_an_include_and_the_scope_src_ui_reads_back_as_kept() {
    let search = Search {
        query: String::from("socket"),
        mode: Mode::Text,
        match_case: true,
        include: String::from("*.rs"),
        scope: Scope::Named,
        named: Some(Place {
            path: String::from("src/ui"),
            is_file: false,
        }),
        ..Search::default()
    };
    let mut persisted = Persisted::default();
    keep_search(&mut persisted.file_searches, Path::new("/work/app"), &search);
    let read = read_back(&persisted);
    assert_eq!(search_of(&read.file_searches, Path::new("/work/app")), Some(&search));
    assert!(persisted.to_text().contains("\"fileSearches\""));
}

#[test]
fn file_search_remember__a_search_as_a_first_one_is_is_not_written() {
    let mut persisted = Persisted::default();
    let folder = Path::new("/work/app");
    keep_search(&mut persisted.file_searches, folder, &Search::default());
    assert_eq!(persisted.file_searches, Vec::new());
    assert!(!persisted.to_text().contains("fileSearches"));
    let typed = Search {
        query: String::from("socket"),
        ..Search::default()
    };
    keep_search(&mut persisted.file_searches, folder, &typed);
    keep_search(&mut persisted.file_searches, folder, &Search::default());
    assert_eq!(
        persisted.file_searches,
        Vec::new(),
        "back to a first search, nothing is kept"
    );
}

#[test]
fn file_search_remember__of_21_repositories_searched_the_20_searched_last_are_kept() {
    let mut persisted = Persisted::default();
    for number in 0..21 {
        let search = Search {
            query: format!("text {number}"),
            ..Search::default()
        };
        keep_search(
            &mut persisted.file_searches,
            &PathBuf::from(format!("/work/{number}")),
            &search,
        );
    }
    let read = read_back(&persisted);
    assert_eq!(read.file_searches.len(), 20);
    assert_eq!(search_of(&read.file_searches, Path::new("/work/0")), None);
    assert_eq!(
        search_of(&read.file_searches, Path::new("/work/20")).map(|search| search.query.as_str()),
        Some("text 20")
    );
}

#[test]
fn a_kept_search_in_a_shape_this_version_does_not_know_is_left_out() {
    let text = r#"{"fileSearches": [{"folder": "/work/app", "mode": "symbols", "query": "x"},
        {"folder": "/work/site", "query": "y"}]}"#;
    let read = Persisted::from_text(text);
    assert_eq!(search_of(&read.file_searches, Path::new("/work/app")), None);
    assert_eq!(
        search_of(&read.file_searches, Path::new("/work/site")).map(|search| search.query.as_str()),
        Some("y")
    );
}
