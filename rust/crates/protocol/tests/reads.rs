//! The JSON of the reads of version 1.4, from stage 5: each parameter and answer read back as it was written, every
//! variant of their enums in its JSON form, and the engine's types made into their JSON. Those of the graph, from
//! version 1.5, are in `graph.rs`.

#![cfg(test)]

use std::sync::Arc;

use model::blame::{BlameCommit, BlameLine, PreviousVersion};
use model::commit::{BranchLog, Commit};
use model::commit_page::{DiffLine, FilePatch, Hunk, LineKind, PatchLines};
use model::compare::{ComparePatch, MergeOutlook, Side};
use model::details::{CommitDetail, Signature};
use model::file_history::{CommittedLines, LineRange};
use model::state::{ChangeKind, FileChange};
use model::{CommitId, Timestamp};
use protocol::reads::{
    BlameLineJson, BranchLogJson, BytesJson, CommitAskedJson, CommitDetailJson, CommitDetailParams, CommittedLinesJson,
    CompareModeJson, ComparePatchParams, CompareSidesParams, ComparisonSideJson, DiffSidesParams, FileVersionJson,
    MergeOutlookJson, PatchFormJson, PatchJson, PullRequestsJson, VersionJson, methods,
};
use protocol::state::{ChangeKindJson, FileChangeJson};
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

#[test]
fn every_read_has_its_method_once_the_graphs_since_version_1_5_and_the_changes_to_read_since_1_8() {
    assert_eq!(methods::ALL.len(), 38);
    assert!(methods::ALL.contains(&"read/repo.watch-changes"));
    assert!(
        !methods::ALL.contains(&methods::GRAPH_ROWS),
        "the pages of a read are a notification, not a read"
    );
    for (index, method) in methods::ALL.iter().enumerate() {
        assert!(method.starts_with("read/"), "{method}");
        assert!(
            !methods::ALL.iter().skip(index + 1).any(|other| other == method),
            "{method}"
        );
    }
    for graph in ["graph.read-commits", "graph.read-position", "search.read-matches"] {
        assert!(methods::ALL.contains(&format!("read/{graph}").as_str()), "{graph}");
    }
}

#[test]
fn a_commit_is_asked_by_its_id_by_its_stash_or_as_the_uncommitted_changes() {
    let read = |value: Value| serde_json::from_value::<CommitDetailParams>(value).unwrap().asked;
    assert_eq!(
        read(json!({ "repository": "/r", "commit": "a".repeat(40) })),
        CommitAskedJson::Commit { commit: "a".repeat(40) }
    );
    assert_eq!(
        read(json!({ "repository": "/r", "stash": 2 })),
        CommitAskedJson::Stash { stash: 2 }
    );
    assert_eq!(
        read(json!({ "repository": "/r", "uncommitted": true })),
        CommitAskedJson::Uncommitted { uncommitted: true }
    );
}

#[test]
fn the_details_of_a_commit_have_the_fields_of_model_in_camel_case() {
    let signature = Signature {
        name: String::from("Ada"),
        email: String::from("ada@example.com"),
        date: Timestamp(1_700_000_000),
    };
    let detail = CommitDetail {
        id: Some(commit('a')),
        parents: vec![commit('b')],
        author: Some(signature.clone()),
        committer: Some(signature),
        message: String::from("Add the parser"),
        files: vec![FileChange {
            path: String::from("b.rs"),
            original_path: Some(String::from("a.rs")),
            kind: ChangeKind::Renamed,
        }],
        untracked_files: Vec::new(),
    };
    let json = serde_json::to_value(CommitDetailJson::from(&detail)).unwrap();
    assert_eq!(json["author"]["date"], json!(1_700_000_000));
    assert_eq!(json["files"][0]["originalPath"], json!("a.rs"));
    assert_eq!(json["untrackedFiles"], json!([]));
    round_trip(&CommitDetailJson::from(&detail));
}

#[test]
fn a_patch_is_files_unmatched_or_text_and_its_lines_say_what_they_are() {
    let patch = FilePatch {
        path: String::from("a.txt"),
        original_path: None,
        kind: ChangeKind::Modified,
        hunks: vec![Hunk {
            header: String::from("@@ -1 +1 @@"),
            lines: [
                LineKind::Context,
                LineKind::Added,
                LineKind::Removed,
                LineKind::NoNewline,
            ]
            .into_iter()
            .map(|kind| DiffLine {
                kind,
                text: String::from("x"),
                before: Some(1),
                after: None,
            })
            .collect(),
        }],
        added: 1,
        removed: 1,
        is_binary: false,
        is_past_limit: false,
    };
    let files = PatchJson::from(&PatchLines::Files(vec![patch]));
    let json = serde_json::to_value(&files).unwrap();
    assert_eq!(json["kind"], json!("files"));
    let kinds: Vec<&Value> = (0..4)
        .map(|line| &json["files"][0]["hunks"][0]["lines"][line]["kind"])
        .collect();
    assert_eq!(
        kinds,
        [
            &json!("context"),
            &json!("added"),
            &json!("removed"),
            &json!("noNewline")
        ]
    );
    assert_eq!(
        serde_json::to_value(PatchJson::from(&PatchLines::Unmatched)).unwrap(),
        json!({ "kind": "unmatched" })
    );
    let text = PatchJson::from(&ComparePatch::Text(b"diff\n".to_vec()));
    assert_eq!(
        serde_json::to_value(&text).unwrap(),
        json!({ "kind": "text", "text": { "base64": "ZGlmZgo=" } })
    );
    for patch in [files, text] {
        round_trip(&patch);
    }
}

#[test]
fn a_comparison_names_its_sides_and_says_what_a_merge_would_do() {
    let params = CompareSidesParams {
        repository: String::from("/r"),
        first: ComparisonSideJson::Commit { commit: "a".repeat(40) },
        second: ComparisonSideJson::WorkingTree,
        mode: CompareModeJson::SinceDiverged,
        path: None,
    };
    round_trip(&params);
    assert_eq!(
        serde_json::to_value(&params).unwrap()["second"],
        json!({ "kind": "workingTree" })
    );
    assert_eq!(serde_json::to_value(params.mode).unwrap(), json!("sinceDiverged"));
    let request = git::read::compare::CompareRequest::try_from(&params).unwrap();
    assert_eq!(
        (request.first, request.second),
        (Side::Commit(commit('a')), Side::WorkingTree)
    );
    let not_a_commit = CompareSidesParams {
        first: ComparisonSideJson::Commit {
            commit: String::from("main"),
        },
        ..params
    };
    assert!(git::read::compare::CompareRequest::try_from(&not_a_commit).is_err());
    for (outlook, kind) in [
        (MergeOutlook::NotAsked, "notAsked"),
        (MergeOutlook::Clean, "clean"),
        (MergeOutlook::Conflicts(vec![String::from("a.txt")]), "conflicts"),
        (MergeOutlook::Unknown, "unknown"),
    ] {
        let json = MergeOutlookJson::from(&outlook);
        assert_eq!(serde_json::to_value(&json).unwrap()["kind"], json!(kind));
        round_trip(&json);
    }
    let patch = ComparePatchParams {
        repository: String::from("/r"),
        first: ComparisonSideJson::Commit { commit: "a".repeat(40) },
        second: ComparisonSideJson::Commit { commit: "b".repeat(40) },
        mode: CompareModeJson::Everything,
        path: Some(String::from("src")),
        merge_base: None,
        files: vec![FileChangeJson {
            path: String::from("src/a.rs"),
            original_path: None,
            kind: ChangeKindJson::Added,
        }],
        form: PatchFormJson::Text,
    };
    round_trip(&patch);
    assert!(patch.comparison().is_ok());
}

#[test]
fn each_version_of_a_file_has_its_kind_and_the_bytes_not_saved_travel_in_base64() {
    let versions = [
        FileVersionJson::Absent,
        FileVersionJson::Index,
        FileVersionJson::Commit { commit: "c".repeat(40) },
        FileVersionJson::Base,
        FileVersionJson::WorkingTree {
            unsaved: Some(BytesJson::of(b"edited\n")),
        },
    ];
    let kinds: Vec<Value> = versions
        .iter()
        .map(|version| serde_json::to_value(version).unwrap()["kind"].clone())
        .collect();
    assert_eq!(
        kinds,
        [
            json!("absent"),
            json!("index"),
            json!("commit"),
            json!("base"),
            json!("workingTree")
        ]
    );
    for version in versions {
        let side = VersionJson {
            version,
            path: String::from("a.txt"),
        };
        assert!(git::read::diff::VersionRequest::try_from(&side).is_ok());
        round_trip(&DiffSidesParams {
            repository: String::from("/r"),
            left: side.clone(),
            right: side,
        });
    }
}

#[test]
fn a_blamed_line_is_committed_with_its_commit_or_not_committed() {
    let committed = BlameLine::Committed(Arc::new(BlameCommit {
        id: commit('d'),
        author: String::from("Ada"),
        author_time: Timestamp(5),
        summary: String::from("Add a"),
        previous: Some(PreviousVersion {
            commit: commit('e'),
            path: String::from("old.txt"),
        }),
    }));
    let json = serde_json::to_value(BlameLineJson::from(&committed)).unwrap();
    assert_eq!(json["kind"], json!("committed"));
    assert_eq!(json["authorTime"], json!(5));
    assert_eq!(json["previous"]["path"], json!("old.txt"));
    assert_eq!(
        serde_json::to_value(BlameLineJson::from(&BlameLine::NotCommitted)).unwrap(),
        json!({ "kind": "notCommitted" })
    );
    round_trip(&BlameLineJson::from(&committed));
    let lines = CommittedLines::Committed {
        commit: commit('f'),
        path: String::from("a.txt"),
        lines: Some(LineRange { start: 2, end: 3 }),
    };
    round_trip(&CommittedLinesJson::from(&lines));
    assert_eq!(
        serde_json::to_value(CommittedLinesJson::from(&CommittedLines::InNoCommit)).unwrap(),
        json!({ "kind": "inNoCommit" })
    );
}

#[test]
fn the_log_of_what_is_checked_out_has_its_commits_and_what_is_not_pushed() {
    let log = BranchLog {
        commits: vec![Commit {
            id: commit('a'),
            parents: vec![commit('b')],
            author_name: String::from("Ada"),
            author_email: String::from("ada@example.com"),
            author_date: Timestamp(9),
            subject: String::from("Add a"),
        }],
        has_more: true,
        unpushed: Some(vec![commit('a')]),
        upstream: Some(String::from("origin/main")),
    };
    let json = BranchLogJson::from(&log);
    round_trip(&json);
    let value = serde_json::to_value(&json).unwrap();
    assert_eq!(value["commits"][0]["authorEmail"], json!("ada@example.com"));
    assert_eq!(value["hasMore"], json!(true));
}

#[test]
fn the_pull_requests_are_read_or_say_why_not() {
    for (answer, kind) in [
        (PullRequestsJson::NotOnGitHub, "notOnGitHub"),
        (PullRequestsJson::NoToken, "noToken"),
        (
            PullRequestsJson::Failed {
                message: String::from("GitHub answered 401 Unauthorized"),
                is_unauthorized: true,
            },
            "failed",
        ),
        (PullRequestsJson::from(&model::github::PullRequests::default()), "read"),
    ] {
        assert_eq!(serde_json::to_value(&answer).unwrap()["kind"], json!(kind));
        round_trip(&answer);
    }
}
