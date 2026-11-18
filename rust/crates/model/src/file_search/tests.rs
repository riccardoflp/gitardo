#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::single_range_in_vec_init, reason = "a line with one match has one range")]

use super::{
    Asked, Filters, Found, Mode, NameFound, Place, RankRequest, ResultRow, Scope, Search, Section, TextMatch, counts,
    folders_of, list_results, names_found, open_with, pathspecs, rank_names,
};

/// The pathspecs of what **Include** and **Exclude** hold, with **Hidden** on, so that only theirs are given.
fn specs(include: &str, exclude: &str) -> Vec<String> {
    pathspecs(&Filters {
        include,
        exclude,
        hidden: true,
    })
}

/// Names, owned.
fn names(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// The paths a query finds among some names, the best first.
fn ranked(paths: &[&str], query: &str, strict: bool) -> Vec<String> {
    let names = names(paths);
    let request = RankRequest {
        query,
        strict,
        most: 100,
    };
    names_found(&names, rank_names(&names, &request))
        .into_iter()
        .map(|found| found.path)
        .collect()
}

/// A name found, with no letter marked.
fn name_found(path: &str) -> NameFound {
    NameFound {
        path: path.to_owned(),
        marks: Vec::new(),
    }
}

/// A line of text found.
fn line(path: &str, number: usize) -> TextMatch {
    TextMatch {
        path: path.to_owned(),
        line: number,
        text: String::from("socket"),
        marks: vec![0..6],
    }
}

#[test]
fn file_search_pathspecs__a_name_is_looked_for_anywhere_as_a_file_and_as_a_folder() {
    assert_eq!(specs("src", ""), [":(glob)**/src", ":(glob)**/src/**"]);
}

#[test]
fn file_search_pathspecs__a_path_is_read_from_the_top() {
    assert_eq!(
        specs("./src, src/app", ""),
        [":(glob)src", ":(glob)src/**", ":(glob)src/app", ":(glob)src/app/**"]
    );
    assert_eq!(specs("/src", ""), [":(glob)src", ":(glob)src/**"]);
}

#[test]
fn file_search_pathspecs__a_pattern_that_ends_with_a_slash_finds_folders_only() {
    assert_eq!(specs("", "dist/"), [":(exclude,glob)**/dist/**"]);
    assert_eq!(specs("./dist/", ""), [":(glob)dist/**"]);
}

#[test]
fn file_search_pathspecs__a_comma_in_braces_separates_alternatives_not_patterns() {
    assert_eq!(
        specs("*.{js,ts}, docs", ""),
        [
            ":(glob)**/*.js",
            ":(glob)**/*.js/**",
            ":(glob)**/*.ts",
            ":(glob)**/*.ts/**",
            ":(glob)**/docs",
            ":(glob)**/docs/**"
        ]
    );
    assert_eq!(specs("{a,b{c,d}}", "").len(), 6);
    assert_eq!(specs("a{b", ""), [":(glob)**/a{b", ":(glob)**/a{b/**"]);
}

#[test]
fn file_search_pathspecs__hidden_off_leaves_out_every_name_that_starts_with_a_dot() {
    let off = pathspecs(&Filters {
        include: "",
        exclude: "",
        hidden: false,
    });
    assert_eq!(off, [":(exclude,glob)**/.*", ":(exclude,glob)**/.*/**"]);
    assert_eq!(specs("", ""), Vec::<String>::new());
}

#[test]
fn file_search_pathspecs__backslashes_are_slashes_and_empty_patterns_give_nothing() {
    assert_eq!(specs("src\\ui", ""), [":(glob)src/ui", ":(glob)src/ui/**"]);
    assert_eq!(specs(" , ", " ,,"), Vec::<String>::new());
    assert_eq!(specs("./, ., /", ""), Vec::<String>::new());
}

#[test]
fn file_search_rank_names__letters_in_order_match_and_are_marked_at_the_starts_of_words() {
    let names = names(&["src/hooks/useSocket.ts", "README.md"]);
    let request = RankRequest {
        query: "usock",
        strict: false,
        most: 10,
    };
    let found = rank_names(&names, &request);
    assert_eq!(found.total, 1);
    let first = found.names.first().unwrap();
    assert_eq!(first.index, 0);
    let marked: String = first
        .marks
        .iter()
        .map(|at| names[0][*at..].chars().next().unwrap())
        .collect();
    assert_eq!(marked, "uSock");
}

#[test]
fn file_search_rank_names__every_term_has_to_match() {
    let paths = ["src/hooks/useSocket.ts", "src/app.rs"];
    assert_eq!(ranked(&paths, "use sock", false), ["src/hooks/useSocket.ts"]);
    assert_eq!(ranked(&paths, "use zzz", false), Vec::<String>::new());
}

#[test]
fn file_search_rank_names__the_whole_name_then_its_start_then_inside_it_then_the_path() {
    let paths = [
        "src/list/mod.rs",
        "src/playlist.rs",
        "src/ui/list_view.rs",
        "src/ui/list.rs",
        "src/other.rs",
    ];
    assert_eq!(
        ranked(&paths, "list", false),
        [
            "src/ui/list.rs",
            "src/ui/list_view.rs",
            "src/playlist.rs",
            "src/list/mod.rs"
        ]
    );
}

#[test]
fn file_search_rank_names__the_case_of_what_is_typed_does_not_matter() {
    let paths = [
        "src/list/mod.rs",
        "src/playlist.rs",
        "src/ui/list_view.rs",
        "src/ui/List.rs",
    ];
    assert_eq!(ranked(&paths, "LIST", false), ranked(&paths, "list", false));
    assert_eq!(ranked(&paths, "LIST", false).first().unwrap(), "src/ui/List.rs");
}

#[test]
fn file_search_rank_names__a_term_with_a_slash_is_read_against_the_whole_path() {
    let paths = ["src/list/ui.rs", "src/ui/list.rs"];
    assert_eq!(ranked(&paths, "ui/list", false), ["src/ui/list.rs"]);
    assert_eq!(ranked(&paths, "ui\\list", false), ["src/ui/list.rs"]);
}

#[test]
fn file_search_rank_names__letters_apart_along_the_path_match_in_files_and_not_in_all() {
    let paths = ["src/render/ai/list.rs"];
    assert_eq!(ranked(&paths, "rail", false), ["src/render/ai/list.rs"]);
    assert_eq!(ranked(&paths, "rail", true), Vec::<String>::new());
    assert_eq!(ranked(&paths, "render", true), ["src/render/ai/list.rs"]);
}

#[test]
fn file_search_rank_names__the_best_are_kept_and_all_that_matched_are_counted() {
    let paths: Vec<String> = (0..30)
        .map(|number| format!("src/deep/folder/item{number:02}.rs"))
        .collect();
    let mut names = paths.clone();
    names.push(String::from("src/item.rs"));
    names.push(String::from("src/other.rs"));
    let request = RankRequest {
        query: "item",
        strict: false,
        most: 20,
    };
    let found = rank_names(&names, &request);
    assert_eq!((found.names.len(), found.total), (20, 31));
    assert_eq!(found.names.first().unwrap().index, 30);
}

#[test]
fn file_search_rank_names__with_nothing_typed_the_names_come_in_their_order() {
    let names = names(&["b.rs", "a.rs", "c.rs"]);
    let request = RankRequest {
        query: "  ",
        strict: true,
        most: 2,
    };
    let found = rank_names(&names, &request);
    assert_eq!(found.total, 3);
    assert_eq!(found.names.iter().map(|name| name.index).collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn file_search_rank_names__the_folders_are_those_of_the_files_each_once_in_order() {
    let files = names(&[
        "src/ui/list.rs",
        "README.md",
        "src/ui/row.rs",
        "docs/a b/é.md",
        "src/app.rs",
    ]);
    assert_eq!(folders_of(&files), ["docs", "docs/a b", "src", "src/ui"]);
}

#[test]
fn file_search_list_results__in_files_the_rows_are_the_files_kept_in_their_order() {
    let found = Found {
        files: vec![name_found("src/a.rs"), name_found("src/b.rs")],
        folders: vec![name_found("src")],
        ..Found::default()
    };
    assert_eq!(
        list_results(&found, Mode::Files),
        [ResultRow::File(0), ResultRow::File(1)]
    );
    assert_eq!(list_results(&found, Mode::Folders), [ResultRow::Folder(0)]);
    assert_eq!(list_results(&found, Mode::Text), Vec::new());
}

#[test]
fn file_search_list_results__in_text_the_lines_are_under_a_row_of_their_file_that_counts_them() {
    let found = Found {
        text: vec![line("src/a.rs", 3), line("src/a.rs", 9), line("src/b.rs", 1)],
        ..Found::default()
    };
    assert_eq!(
        list_results(&found, Mode::Text),
        [
            ResultRow::TextFile { first: 0, lines: 2 },
            ResultRow::Line(0),
            ResultRow::Line(1),
            ResultRow::TextFile { first: 2, lines: 1 },
            ResultRow::Line(2),
        ]
    );
    assert_eq!(counts(&found), (3, 2));
}

#[test]
fn file_search_list_results__in_all_the_sections_are_under_their_headings_in_order() {
    let found = Found {
        folders: vec![name_found("src")],
        files: vec![name_found("src/a.rs")],
        text: vec![line("src/a.rs", 3)],
        ..Found::default()
    };
    assert_eq!(
        list_results(&found, Mode::All),
        [
            ResultRow::Heading(Section::Folders),
            ResultRow::Folder(0),
            ResultRow::Heading(Section::Files),
            ResultRow::File(0),
            ResultRow::Heading(Section::Text),
            ResultRow::TextFile { first: 0, lines: 1 },
            ResultRow::Line(0),
        ]
    );
}

#[test]
fn file_search_list_results__in_all_a_section_with_nothing_has_no_heading() {
    let found = Found {
        files: vec![name_found("src/a.rs")],
        ..Found::default()
    };
    assert_eq!(
        list_results(&found, Mode::All),
        [ResultRow::Heading(Section::Files), ResultRow::File(0)]
    );
    assert_eq!(list_results(&Found::default(), Mode::All), Vec::new());
}

#[test]
fn file_search_list_results__in_all_fifty_folders_and_a_hundred_files_are_listed() {
    let found = Found {
        folders: (0..80).map(|number| name_found(&format!("folder{number}"))).collect(),
        files: (0..300).map(|number| name_found(&format!("file{number}.rs"))).collect(),
        ..Found::default()
    };
    let rows = list_results(&found, Mode::All);
    let folders = rows.iter().filter(|row| matches!(row, ResultRow::Folder(_))).count();
    let files = rows.iter().filter(|row| matches!(row, ResultRow::File(_))).count();
    assert_eq!((folders, files), (50, 100));
    assert_eq!(list_results(&found, Mode::Files).len(), 300);
}

/// A search with something typed and a filter, as it was left.
fn left() -> Search {
    Search {
        query: String::from("before"),
        mode: Mode::Text,
        include: String::from("*.rs"),
        ..Search::default()
    }
}

/// **Search here** on a folder or a file.
fn here(path: &str, is_file: bool) -> Asked {
    Asked::Here(Some(Place {
        path: path.to_owned(),
        is_file,
    }))
}

#[test]
fn file_search_open__text_selected_on_one_line_becomes_what_is_looked_for() {
    let asked = Asked::Page {
        selected: Some(String::from("parse_log")),
    };
    let search = open_with(&left(), &asked);
    assert_eq!(search.query, "parse_log");
    assert_eq!((search.mode, search.include.as_str()), (Mode::Text, "*.rs"));
}

#[test]
fn file_search_open__a_selection_of_two_lines_or_none_leaves_what_the_box_held() {
    for selected in [
        None,
        Some(String::new()),
        Some(String::from("  ")),
        Some(String::from("one\ntwo")),
    ] {
        assert_eq!(open_with(&left(), &Asked::Page { selected }), left());
    }
}

#[test]
fn file_search_open__search_here_on_a_folder_adds_its_scope_and_chooses_it() {
    let search = open_with(&left(), &here("src/ui", false));
    assert_eq!(search.scope, Scope::Named);
    assert_eq!(search.place().map(|place| place.path.as_str()), Some("src/ui"));
    assert_eq!(
        (search.query.as_str(), search.mode, search.include.as_str()),
        ("before", Mode::Text, "*.rs")
    );
}

#[test]
fn file_search_open__search_here_on_a_file_makes_that_file_the_scope() {
    let search = open_with(&left(), &here("src/app.rs", true));
    assert_eq!(
        search.place().map(|place| (place.path.as_str(), place.is_file)),
        Some(("src/app.rs", true))
    );
}

#[test]
fn file_search_open__search_here_on_the_top_row_chooses_the_repository() {
    let named = open_with(&left(), &here("src/ui", false));
    let search = open_with(&named, &Asked::Here(None));
    assert_eq!(search.scope, Scope::Repository);
    assert_eq!(search.place(), None);
    assert_eq!(search.named.map(|place| place.path), Some(String::from("src/ui")));
}

#[test]
fn file_search_open__the_next_search_here_takes_the_place_of_the_one_before() {
    let first = open_with(&left(), &here("src/ui", false));
    let second = open_with(&first, &here("docs", false));
    assert_eq!(second.named.map(|place| place.path), Some(String::from("docs")));
    assert_eq!(second.scope, Scope::Named);
}
