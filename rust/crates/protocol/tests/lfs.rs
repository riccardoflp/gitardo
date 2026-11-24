//! The JSON of what version 1.13 adds for Git LFS (milestone M47): the reads of its status, its files, its locks and
//! the content of a pointer, the targets of `lfs.download` and `lfs.unlock`, and the answer `names` an operation
//! offered is given.

#![cfg(test)]

use model::contract::{Answer, Target};
use model::lfs::{LfsDownload, LfsLock, LfsLocks, LfsPath};
use protocol::feature::{AnswerJson, TargetJson};
use protocol::reads::{LfsContentJson, LfsContentParams, LfsLocksJson, LfsLocksParams, StoredPointerJson, methods};
use protocol::state::StateJson;
use serde_json::{Value, json};

fn lock(is_ours: Option<bool>) -> LfsLock {
    LfsLock {
        id: String::from("7"),
        path: String::from("art/logo.psd"),
        owner: String::from("reviewer"),
        locked_at: String::from("2026-11-24T10:00:00Z"),
        is_ours,
    }
}

#[test]
fn the_four_reads_of_git_lfs_are_reads_of_version_1_13() {
    for method in [
        "read/lfs.read-status",
        "read/lfs.read-files",
        "read/lfs.read-locks",
        "read/lfs.read-content",
    ] {
        assert!(methods::ALL.contains(&method), "{method}");
    }
}

#[test]
fn the_locks_are_written_with_whose_they_are_or_as_a_server_without_them() {
    let read = LfsLocksJson::from(&LfsLocks::Read(vec![lock(Some(true)), lock(None)]));
    assert_eq!(
        serde_json::to_value(&read).unwrap(),
        json!({ "kind": "read", "locks": [
            { "id": "7", "path": "art/logo.psd", "owner": "reviewer", "lockedAt": "2026-11-24T10:00:00Z", "isOurs": true },
            { "id": "7", "path": "art/logo.psd", "owner": "reviewer", "lockedAt": "2026-11-24T10:00:00Z", "isOurs": null }
        ] })
    );
    let unsupported = LfsLocksJson::from(&LfsLocks::Unsupported(String::from("locking API not supported")));
    assert_eq!(
        serde_json::to_value(&unsupported).unwrap(),
        json!({ "kind": "unsupported", "message": "locking API not supported" })
    );
    let params: LfsLocksParams = serde_json::from_value(json!({ "repository": "/r" })).unwrap();
    assert_eq!(params.remote, None);
}

#[test]
fn a_side_not_downloaded_is_its_pointer_with_its_size_its_id_and_that_it_is_not_downloaded() {
    let answer = LfsContentJson {
        bytes: None,
        commit: Some("a".repeat(40)),
        stored: Some(StoredPointerJson {
            size: 2_000,
            oid: "b".repeat(64),
            is_downloaded: false,
        }),
    };
    let written = serde_json::to_value(&answer).unwrap();
    assert_eq!(
        written["stored"],
        json!({ "size": 2000, "oid": "b".repeat(64), "isDownloaded": false })
    );
    let index: LfsContentParams = serde_json::from_value(json!({ "repository": "/r", "path": "a.psd" })).unwrap();
    assert_eq!(index.revision, None, "no revision is the index");
}

#[test]
fn the_files_to_download_and_a_lock_go_both_ways_between_their_json_and_the_engines_target() {
    let download = Target::LfsFiles(Box::new(LfsDownload {
        paths: vec![
            LfsPath {
                path: String::from("art/logo.psd"),
                is_folder: false,
            },
            LfsPath {
                path: String::from("art"),
                is_folder: true,
            },
        ],
        commit: None,
    }));
    let json_of = TargetJson::of(&download).unwrap();
    assert_eq!(
        serde_json::to_value(&json_of).unwrap(),
        json!({ "kind": "lfsFiles", "paths": [
            { "path": "art/logo.psd", "isFolder": false },
            { "path": "art", "isFolder": true }
        ], "commit": null })
    );
    assert_eq!(Target::try_from(json_of).unwrap(), download);
    let every: TargetJson = serde_json::from_value(json!({ "kind": "lfsFiles", "paths": [] })).unwrap();
    assert_eq!(Target::try_from(every).unwrap(), Target::LfsFiles(Box::default()));
    let unlock = Target::LfsLock(Box::new(lock(Some(false))));
    let json_of = TargetJson::of(&unlock).unwrap();
    assert_eq!(serde_json::to_value(&json_of).unwrap()["kind"], json!("lfsLock"));
    assert_eq!(Target::try_from(json_of).unwrap(), unlock);
    let wrong: TargetJson =
        serde_json::from_value(json!({ "kind": "lfsFiles", "paths": [], "commit": "main" })).unwrap();
    assert!(Target::try_from(wrong).is_err(), "a commit is its full id");
}

#[test]
fn names_read_before_an_operation_is_offered_travel_as_the_answer_names() {
    let names = Answer::Names(vec![String::from("logo.psd")]);
    let written = AnswerJson::of(&names).unwrap();
    assert_eq!(
        serde_json::to_value(&written).unwrap(),
        json!({ "kind": "names", "names": ["logo.psd"] })
    );
    assert_eq!(Answer::from(written), names);
}

#[test]
fn a_state_without_lfs_from_an_older_engine_reads_as_git_lfs_unknown_and_nothing_stored() {
    let mut state = serde_json::to_value(StateJson::from(&model::state::RepositoryState::default())).unwrap();
    assert_eq!(
        state["lfs"],
        json!({ "version": null, "isSetUp": false, "patterns": [] })
    );
    if let Value::Object(fields) = &mut state {
        let _ = fields.remove("lfs");
    }
    let read: StateJson = serde_json::from_value(state).unwrap();
    assert_eq!(read.lfs.version, None);
}
