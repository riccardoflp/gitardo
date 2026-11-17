//! The JSON of the reads of the graph, from version 1.5 (stage 7): the timeline as the extension asks for it, every
//! variant of its enums in its JSON form, the timeline the engine reads from it, and the rows of a page.

#![cfg(test)]

use std::ffi::OsString;

use engine::timeline::TimelineRequest;
use git::read::graph::{CommitPage, Parents};
use model::commit::Commit;
use model::file_history::LineRange;
use model::filter::{BranchesShown, Kinds};
use model::graph::Row;
use model::{CommitId, Timestamp};
use protocol::reads::{
    BranchesShownJson, CommitPageJson, CommitsParams, MatchesParams, NarrowingJson, ParentsJson, PositionParams,
    SearchModeJson, TimelineJson,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

fn commit(letter: char) -> CommitId {
    CommitId::parse(&letter.to_string().repeat(40)).unwrap()
}

/// A timeline of the extension, read from its JSON.
fn timeline(value: Value) -> TimelineJson {
    serde_json::from_value(value).unwrap()
}

/// The words of some arguments.
fn words(arguments: &[OsString]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_timeline_needs_only_its_page_size_and_every_branch_is_shown_from_the_first_page() {
    let read = timeline(json!({ "pageSize": 1000 }));
    assert_eq!(
        read,
        TimelineJson {
            shown: None,
            parents: ParentsJson::All,
            alone: None,
            narrowing: None,
            page_size: 1000,
            pages: 1,
            skip: 0,
            stashes: false,
        }
    );
    let request = TimelineRequest::try_from(&read).unwrap();
    assert_eq!(request.page_size.get(), 1000);
    assert_eq!((request.pages, request.parents, request.shown), (1, Parents::All, None));
    let clamped = TimelineRequest::try_from(&timeline(json!({ "pageSize": 20, "pages": 0 }))).unwrap();
    assert_eq!(
        (clamped.page_size.get(), clamped.pages),
        (100, 1),
        "as gitardo.graph.pageSize is read"
    );
}

#[test]
fn every_kind_of_branches_shown_has_its_json_and_is_the_engines_choice() {
    let cases = [
        (json!({ "kind": "current" }), BranchesShown::Current),
        (json!({ "kind": "upstream" }), BranchesShown::Upstream),
        (
            json!({ "kind": "kinds", "local": true, "remote": false, "tags": true }),
            BranchesShown::Kinds(Kinds {
                local: true,
                remote: false,
                tags: true,
            }),
        ),
        (
            json!({ "kind": "references", "names": ["refs/heads/main"] }),
            BranchesShown::References(vec![String::from("refs/heads/main")]),
        ),
    ];
    for (value, shown) in cases {
        let read: BranchesShownJson = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read).unwrap(), value);
        assert_eq!(BranchesShown::from(&read), shown);
        round_trip(&read);
    }
    assert_eq!(serde_json::to_value(ParentsJson::First).unwrap(), json!("first"));
    for (mode, word) in [
        (SearchModeJson::Message, "message"),
        (SearchModeJson::Author, "author"),
        (SearchModeJson::Changes, "changes"),
        (SearchModeJson::ChangesPattern, "changesPattern"),
    ] {
        assert_eq!(serde_json::to_value(mode).unwrap(), json!(word));
    }
}

#[test]
fn each_narrowing_gives_the_arguments_of_the_desktop_and_the_lines_list_their_commit_alone() {
    let file = timeline(json!({
        "pageSize": 100,
        "stashes": true,
        "narrowing": { "kind": "file", "path": "b.txt", "earlier": ["a.txt"] }
    }));
    let request = TimelineRequest::try_from(&file).unwrap();
    assert_eq!(
        words(&request.narrowing),
        ["--parents", "--", ":(literal)b.txt", ":(literal)a.txt"]
    );
    assert!(!request.shows_stashes, "no stash in the history of a file");

    let lines = timeline(json!({
        "pageSize": 100,
        "narrowing": {
            "kind": "lines",
            "path": "a.txt",
            "lines": { "start": 2, "end": 4 },
            "commit": "c".repeat(40),
            "inEditor": { "start": 3, "end": 5 }
        }
    }));
    let request = TimelineRequest::try_from(&lines).unwrap();
    assert_eq!(words(&request.narrowing), ["--parents", "--no-patch", "-L2,4:a.txt"]);
    assert_eq!(request.alone, Some(commit('c')));
    let Some(NarrowingJson::Lines {
        in_editor,
        lines: range,
        ..
    }) = &lines.narrowing
    else {
        panic!("{lines:?}");
    };
    assert_eq!(
        (LineRange::from(*range).start, in_editor.map(|range| range.end)),
        (2, Some(5))
    );

    let folder = timeline(json!({ "pageSize": 100, "narrowing": { "kind": "folder", "path": "src" } }));
    assert_eq!(
        words(&TimelineRequest::try_from(&folder).unwrap().narrowing),
        ["--parents", "--", ":(literal)src"]
    );

    let search = timeline(json!({
        "pageSize": 100,
        "narrowing": { "kind": "search", "mode": "author", "text": "Ada" }
    }));
    assert_eq!(
        words(&TimelineRequest::try_from(&search).unwrap().narrowing),
        ["--fixed-strings", "--regexp-ignore-case", "--author=Ada"]
    );
    for narrowed in [file, lines, folder, search] {
        round_trip(&narrowed);
    }

    let short = timeline(json!({ "pageSize": 100, "alone": "abc1234" }));
    assert!(TimelineRequest::try_from(&short).is_err(), "a commit is its full id");
}

#[test]
fn the_parameters_of_the_three_reads_carry_the_timeline() {
    let commits: CommitsParams =
        serde_json::from_value(json!({ "repository": "/r", "pageSize": 500, "pages": 3, "skip": 999 })).unwrap();
    assert_eq!((commits.timeline.pages, commits.timeline.skip), (3, 999));
    round_trip(&commits);
    let position: PositionParams = serde_json::from_value(json!({
        "repository": "/r",
        "commit": "a".repeat(40),
        "timeline": { "pageSize": 1000, "shown": { "kind": "current" } }
    }))
    .unwrap();
    round_trip(&position);
    let matches: MatchesParams = serde_json::from_value(json!({
        "repository": "/r",
        "text": "fix",
        "mode": "message",
        "timeline": { "pageSize": 1000 }
    }))
    .unwrap();
    assert_eq!(matches.mode, SearchModeJson::Message);
    round_trip(&matches);
}

#[test]
fn a_page_has_its_commits_and_stashes_in_the_order_read() {
    let made = Commit {
        id: commit('a'),
        parents: vec![commit('b')],
        author_name: String::from("Ada"),
        author_email: String::from("ada@example.com"),
        author_date: Timestamp(60),
        subject: String::from("Fix"),
    };
    let page = CommitPage {
        rows: vec![
            Row::Stash {
                commit: Commit {
                    id: commit('c'),
                    ..made.clone()
                },
                index: 0,
            },
            Row::Commit(made),
            Row::Uncommitted {
                parent: Some(commit('a')),
                files: 2,
            },
        ],
        has_more: true,
        follows: false,
    };
    let written = serde_json::to_value(CommitPageJson::from(&page)).unwrap();
    assert_eq!(written["hasMore"], json!(true));
    assert_eq!(written["rows"][0]["kind"], json!("stash"));
    assert_eq!(written["rows"][0]["index"], json!(0));
    assert_eq!(
        written["rows"][1],
        json!({
            "kind": "commit",
            "commit": {
                "id": "a".repeat(40),
                "parents": ["b".repeat(40)],
                "authorName": "Ada",
                "authorEmail": "ada@example.com",
                "authorDate": 60,
                "subject": "Fix"
            }
        })
    );
    assert_eq!(
        written["rows"][2],
        json!({ "kind": "uncommitted", "parent": "a".repeat(40), "files": 2 })
    );
    round_trip(&CommitPageJson::from(&page));
}
