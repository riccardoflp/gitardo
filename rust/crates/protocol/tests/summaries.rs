//! The JSON of what version 1.10 adds for milestone M22: what each folder followed is on (`repo.worktree-summary`), as
//! `read/repo.worktree-summary` answers it and `repository/summaries` says it again, and the change the extension's
//! watchers saw in one of them.

#![cfg(test)]

use std::path::PathBuf;

use model::repo::{SummaryHead, WorktreeSummary};
use model::state::Worktree;
use protocol::messages::{VERSION, is_compatible};
use protocol::state::WorktreeJson;
use protocol::summaries::{
    FolderChangedParams, FolderSummaryJson, SummariesParams, SummariesResult, SummaryHeadJson, SummaryJson,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

/// A summary of each kind of what a folder has checked out.
fn summaries() -> Vec<WorktreeSummary> {
    let of = |head: SummaryHead, changed: u32, upstream: Option<(u32, u32)>| WorktreeSummary {
        head,
        changed,
        upstream,
    };
    vec![
        of(SummaryHead::Branch(String::from("feature/login")), 3, Some((1, 2))),
        of(SummaryHead::Detached(String::from("3f2a1c9")), 0, None),
        of(SummaryHead::NoCommits, 1, None),
        WorktreeSummary::missing(),
    ]
}

#[test]
fn version_1_10_is_spoken_by_an_engine_of_version_1_and_is_not_read_as_1_1() {
    assert_eq!(
        VERSION, "1.11",
        "1.11 added Ask Gitardo (milestone M31) and the number of a pull request (milestone M37)"
    );
    assert!(is_compatible("1.10"));
    assert!(is_compatible("1.9"));
    assert!(is_compatible(VERSION));
}

#[test]
fn what_a_folder_is_on_is_written_with_its_branch_its_files_and_its_commits_to_push_and_to_pull() {
    let written: Vec<SummaryJson> = summaries().iter().map(SummaryJson::from).collect();
    assert_eq!(
        serde_json::to_value(&written).unwrap(),
        json!([
            { "head": { "kind": "branch", "name": "feature/login" }, "changed": 3, "ahead": 1, "behind": 2 },
            { "head": { "kind": "detached", "commit": "3f2a1c9" }, "changed": 0, "ahead": null, "behind": null },
            { "head": { "kind": "noCommits" }, "changed": 1, "ahead": null, "behind": null },
            { "head": { "kind": "missing" }, "changed": 0, "ahead": null, "behind": null }
        ])
    );
    for summary in &written {
        round_trip(summary);
    }
    round_trip(&SummaryHeadJson::Missing);
}

#[test]
fn the_folders_followed_their_summaries_and_a_change_seen_in_one_are_read_back_as_written() {
    round_trip(&SummariesParams {
        folders: vec![String::from("/work/api"), String::from("/work/web")],
    });
    let worktree = Worktree {
        folder: PathBuf::from("/work/api-login"),
        commit: None,
        branch: Some(String::from("feature/login")),
        is_main: false,
        is_current: false,
        is_missing: false,
        uncommitted: None,
    };
    let result = SummariesResult {
        summaries: vec![
            FolderSummaryJson {
                folder: String::from("/work/api"),
                summary: summaries().first().map(SummaryJson::from),
                is_stale: true,
                worktrees: Some(vec![WorktreeJson::from(&worktree)]),
            },
            FolderSummaryJson {
                folder: String::from("/work/api-login"),
                summary: None,
                is_stale: false,
                worktrees: None,
            },
        ],
    };
    let written = serde_json::to_value(&result).unwrap();
    assert_eq!(written["summaries"][0]["isStale"], json!(true), "{written}");
    assert_eq!(
        written["summaries"][0]["worktrees"][0]["branch"],
        json!("feature/login")
    );
    assert_eq!(written["summaries"][1]["summary"], json!(null));
    assert_eq!(written["summaries"][1]["worktrees"], json!(null));
    round_trip(&result);

    let changed: FolderChangedParams = serde_json::from_value(json!({ "folder": "/work/api" })).unwrap();
    assert!(changed.paths.is_empty(), "a change that names no path names none");
    round_trip(&FolderChangedParams {
        folder: String::from("/work/api"),
        paths: vec![String::from("src/a.rs"), String::from(".git/HEAD")],
    });
}
