//! `file-search.read-names` and `file-search.read-text` against throwaway repositories.

#![allow(clippy::single_range_in_vec_init, reason = "a line with one match has one range")]

use std::fs;

use git::read::file_search::{NamesRequest, TextEnd, TextRequest, read_names, read_text};
use git::{Git, GitError, Taken};
use model::file_search::{Filters, TextMatch, pathspecs};
use testkit::Repository;

use crate::write;

/// A repository with files git tracks, one it does not track, and one it ignores.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let files = [
        (".gitignore", "node_modules/\n"),
        (".github/workflows/ci.yml", "socket: yes\n"),
        (".hidden", "socket\n"),
        ("docs/a b/é.md", "nothing here\n"),
        ("src/a.rs", "héllo socket\na.b\naxb\n"),
        ("src/b.rs", "open the Socket\nsock\n"),
        (
            "src/hooks/useSocket.ts",
            "export function useSocket() {}\nlet a = useSocket(); let b = useSocket();\n",
        ),
        ("src/ui/list.rs", "fn list() {}\n"),
    ];
    let _ = repository.commit("Add the files", &files);
    write(&repository, "notes.txt", "untracked socket\n");
    write(&repository, "node_modules/pkg/index.js", "ignored socket\n");
    repository
}

/// The names read with **Hidden** on and no other filter.
fn names(repository: &Repository, request: &NamesRequest) -> Vec<String> {
    read_names(&Git::new(repository.path()), request).unwrap()
}

/// A request for a text in every file, hidden ones too.
fn request(text: &str) -> TextRequest {
    TextRequest {
        text: text.to_owned(),
        most: 1_000,
        ..TextRequest::default()
    }
}

/// What a search of the text found, and how it ended.
fn found(repository: &Repository, request: &TextRequest) -> (Vec<TextMatch>, TextEnd) {
    let mut lines = Vec::new();
    let end = read_text(&Git::new(repository.path()), request, &mut |part| {
        lines.extend(part);
        Taken::More
    })
    .unwrap();
    (lines, end)
}

/// The lines found, as `path:line`.
fn places(repository: &Repository, request: &TextRequest) -> Vec<String> {
    let (lines, _) = found(repository, request);
    lines
        .iter()
        .map(|line| format!("{}:{}", line.path, line.line))
        .collect()
}

#[test]
fn file_search_read_names__tracked_and_untracked_files_are_listed_once_in_order_and_ignored_ones_are_not() {
    let repository = repository();
    assert_eq!(
        names(&repository, &NamesRequest::default()),
        [
            ".github/workflows/ci.yml",
            ".gitignore",
            ".hidden",
            "docs/a b/é.md",
            "notes.txt",
            "src/a.rs",
            "src/b.rs",
            "src/hooks/useSocket.ts",
            "src/ui/list.rs",
        ]
    );
}

#[test]
fn file_search_read_names__what_git_ignores_is_listed_when_asked() {
    let repository = repository();
    let request = NamesRequest {
        ignored: true,
        ..NamesRequest::default()
    };
    assert!(names(&repository, &request).contains(&String::from("node_modules/pkg/index.js")));
}

#[test]
fn file_search_read_names__a_tracked_file_deleted_from_the_working_tree_is_not_listed() {
    let repository = repository();
    fs::remove_file(repository.path().join("src/b.rs")).unwrap();
    let names = names(&repository, &NamesRequest::default());
    assert!(!names.contains(&String::from("src/b.rs")));
    assert!(names.contains(&String::from("src/a.rs")));
}

#[test]
fn file_search_read_names__run_in_a_folder_only_its_files_are_listed_by_their_paths_from_the_top() {
    let repository = repository();
    let request = NamesRequest {
        folder: Some(String::from("src")),
        ..NamesRequest::default()
    };
    assert_eq!(
        names(&repository, &request),
        ["src/a.rs", "src/b.rs", "src/hooks/useSocket.ts", "src/ui/list.rs"]
    );
}

#[test]
fn file_search_read_names__the_pathspecs_of_hidden_off_leave_out_the_names_with_a_dot() {
    let repository = repository();
    let filters = Filters {
        include: "",
        exclude: "",
        hidden: false,
    };
    let request = NamesRequest {
        pathspecs: pathspecs(&filters),
        ..NamesRequest::default()
    };
    let names = names(&repository, &request);
    assert!(!names.iter().any(|name| name.starts_with('.')), "{names:?}");
    assert!(names.contains(&String::from("src/ui/list.rs")));
    let rust = Filters {
        include: "*.rs",
        exclude: "ui/",
        hidden: false,
    };
    let request = NamesRequest {
        pathspecs: pathspecs(&rust),
        ..NamesRequest::default()
    };
    assert_eq!(
        read_names(&Git::new(repository.path()), &request).unwrap(),
        ["src/a.rs", "src/b.rs"]
    );
}

#[test]
fn file_search_read_names__a_path_with_a_space_and_an_accent_is_listed_as_it_is_written() {
    let repository = repository();
    assert!(names(&repository, &NamesRequest::default()).contains(&String::from("docs/a b/é.md")));
}

#[test]
fn file_search_read_text__lines_of_tracked_and_untracked_files_are_found_whatever_the_case_and_ignored_ones_not() {
    let repository = repository();
    assert_eq!(
        places(&repository, &request("socket")),
        [
            ".github/workflows/ci.yml:1",
            ".hidden:1",
            "notes.txt:1",
            "src/a.rs:1",
            "src/b.rs:1",
            "src/hooks/useSocket.ts:1",
            "src/hooks/useSocket.ts:2",
        ]
    );
}

#[test]
fn file_search_read_text__with_match_case_the_capitals_have_to_be_as_typed() {
    let repository = repository();
    let mut request = request("Socket");
    request.match_case = true;
    assert_eq!(
        places(&repository, &request),
        ["src/b.rs:1", "src/hooks/useSocket.ts:1", "src/hooks/useSocket.ts:2"]
    );
}

#[test]
fn file_search_read_text__with_whole_word_a_part_of_a_word_is_not_found() {
    let repository = repository();
    let mut request = request("sock");
    request.whole_word = true;
    assert_eq!(places(&repository, &request), ["src/b.rs:2"]);
}

#[test]
fn file_search_read_text__a_dot_is_a_dot_unless_the_text_is_a_regular_expression() {
    let repository = repository();
    let mut request = request("a.b");
    assert_eq!(places(&repository, &request), ["src/a.rs:2"]);
    request.regex = true;
    assert_eq!(places(&repository, &request), ["src/a.rs:2", "src/a.rs:3"]);
}

#[test]
fn file_search_read_text__a_line_with_two_matches_is_given_once_with_both() {
    let repository = repository();
    let (lines, _) = found(&repository, &request("usesocket"));
    let line = lines.iter().find(|line| line.line == 2).unwrap();
    assert_eq!(line.text, "let a = useSocket(); let b = useSocket();");
    assert_eq!(line.marks, [8..17, 29..38]);
    assert_eq!(lines.iter().filter(|line| line.line == 2).count(), 1);
}

#[test]
fn file_search_read_text__a_match_after_an_accent_is_where_its_bytes_are() {
    let repository = repository();
    let (lines, _) = found(&repository, &request("socket"));
    let line = lines.iter().find(|line| line.path == "src/a.rs").unwrap();
    assert_eq!(line.text, "héllo socket");
    assert_eq!(line.marks, [7..13]);
    assert_eq!(&line.text[7..13], "socket");
}

#[test]
fn file_search_read_text__a_regular_expression_git_cannot_read_is_an_error_with_its_words() {
    let repository = repository();
    let mut request = request("(");
    request.regex = true;
    let error = read_text(&Git::new(repository.path()), &request, &mut |_| Taken::More).unwrap_err();
    let GitError::Failed { stderr, .. } = &error else {
        panic!("{error:?}");
    };
    assert!(stderr.contains("fatal:"), "{stderr}");
    request.regex = false;
    assert_eq!(
        places(&repository, &request),
        [
            "src/hooks/useSocket.ts:1",
            "src/hooks/useSocket.ts:2",
            "src/ui/list.rs:1"
        ]
    );
}

#[test]
fn file_search_read_text__a_text_no_file_holds_gives_no_line_and_no_error() {
    let repository = repository();
    let (lines, end) = found(&repository, &request("nowhere to be found"));
    assert_eq!((lines.len(), end.more), (0, false));
    assert_eq!(found(&repository, &request("")).0, Vec::new());
}

#[test]
fn file_search_read_text__as_many_lines_as_asked_are_given_and_the_read_says_there_are_more() {
    let repository = repository();
    let ten: Vec<String> = (1..=10).map(|number| format!("needle {number}\n")).collect();
    write(&repository, "many.txt", &ten.concat());
    let mut request = request("needle");
    request.most = 3;
    let (lines, end) = found(&repository, &request);
    assert_eq!(lines.iter().map(|line| line.line).collect::<Vec<_>>(), [1, 2, 3]);
    assert!(end.more);
    request.most = 10;
    let (lines, end) = found(&repository, &request);
    assert_eq!((lines.len(), end.more), (10, false));
}

#[test]
fn file_search_read_text__what_git_ignores_is_searched_when_asked() {
    let repository = repository();
    let mut request = request("ignored socket");
    assert_eq!(places(&repository, &request), Vec::<String>::new());
    request.ignored = true;
    assert_eq!(places(&repository, &request), ["node_modules/pkg/index.js:1"]);
}

#[test]
fn file_search_read_text__given_some_files_alone_the_others_are_not_searched() {
    let repository = repository();
    let mut request = request("socket");
    request.files = Some(vec![String::from("src/a.rs"), String::from("notes.txt")]);
    assert_eq!(places(&repository, &request), ["notes.txt:1", "src/a.rs:1"]);
    request.files = Some(Vec::new());
    assert_eq!(places(&repository, &request), Vec::<String>::new());
    request.files = None;
    request.folder = Some(String::from("src/hooks"));
    assert_eq!(
        places(&repository, &request),
        ["src/hooks/useSocket.ts:1", "src/hooks/useSocket.ts:2"]
    );
}

#[test]
fn file_search_read_text__a_long_line_is_cut_with_its_match_marked() {
    let repository = repository();
    let long = format!("{}needle{}\n", "x".repeat(499), "y".repeat(495));
    write(&repository, "long.txt", &long);
    let (lines, _) = found(&repository, &request("needle"));
    let line = lines.first().unwrap();
    assert_eq!(line.text.chars().count(), 240);
    assert_eq!(line.marks, [60..66]);
    assert_eq!(&line.text[60..66], "needle");
}

#[test]
fn file_search_read_text__a_file_with_a_nul_byte_is_not_searched() {
    let repository = repository();
    write(&repository, "binary.dat", "needle\0needle\n");
    write(&repository, "plain.txt", "needle\n");
    assert_eq!(places(&repository, &request("needle")), ["plain.txt:1"]);
}

#[test]
fn file_search_read_text__lines_that_come_in_many_parts_are_each_given_once_and_whole() {
    let repository = repository();
    let many: Vec<String> = (1..=4_000)
        .map(|number| format!("needle of the line {number} of a file long enough\n"))
        .collect();
    write(&repository, "many.txt", &many.concat());
    let mut request = request("needle");
    request.most = 5_000;
    let mut parts = 0;
    let mut lines = Vec::new();
    let end = read_text(&Git::new(repository.path()), &request, &mut |part| {
        parts += 1;
        lines.extend(part);
        Taken::More
    })
    .unwrap();
    assert!(parts > 1, "{parts}");
    assert!(!end.more);
    assert_eq!(lines.len(), 4_000);
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        assert_eq!(line.line, number);
        assert_eq!(line.text, format!("needle of the line {number} of a file long enough"));
        assert_eq!(line.marks, [0..6]);
    }
}
