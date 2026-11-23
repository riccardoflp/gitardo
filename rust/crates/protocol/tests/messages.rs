//! The JSON of the messages of stage 1: golden files that a person looked at, and every message read back as it was
//! written.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine::{LogEntry, LogKind};
use model::state::{
    Branch, ChangeKind, ConflictKind, ConflictedFile, FileChange, Head, Identity, Operation, RemoteBranch,
    RepositoryState, Stash, Tag, Upstream, WorkingTree, Worktree,
};
use model::{CommitId, Timestamp};
use protocol::messages::{
    InitializeParams, InitializeResult, LogEntryJson, LogLevelName, OpenResult, ReadStateParams, StateParams,
    StateReason, is_compatible,
};
use protocol::state::StateJson;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use testkit::assert_golden;

/// The golden file of a test.
fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name)
}

/// The JSON of a value as the golden files keep it: indented, with a line break at the end.
fn pretty(value: &impl Serialize) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

/// A commit id made of one hexadecimal digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A repository with a branch that follows another, a remote branch, a tag, a stash, two worktrees, changes of every
/// kind and a merge stopped half way.
fn repository() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("/work/app"),
        git_folder: PathBuf::from("/work/app/.git"),
        head: Head {
            commit: Some(commit('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![Branch {
            name: String::from("main"),
            commit: commit('a'),
            upstream: Some(Upstream {
                reference: String::from("refs/remotes/origin/main"),
                remote: String::from("origin"),
                remote_branch: String::from("main"),
                ahead: 1,
                behind: 2,
                is_gone: false,
            }),
            is_checked_out: true,
            date: Timestamp(1_700_000_000),
        }],
        remote_branches: vec![RemoteBranch {
            name: String::from("origin/main"),
            remote: String::from("origin"),
            commit: commit('b'),
            date: Timestamp(1_699_000_000),
        }],
        tags: vec![Tag {
            name: String::from("v1.0"),
            commit: commit('b'),
            date: Timestamp(1_699_000_000),
        }],
        stashes: vec![Stash {
            commit: commit('c'),
            index: 0,
            message: String::from("On main: half a fix"),
            date: Timestamp(1_700_000_100),
        }],
        worktrees: worktrees(),
        remotes: vec![String::from("origin")],
        identity: Identity {
            name: Some(String::from("Ada Lovelace")),
            email: Some(String::from("ada@example.com")),
        },
        signing: model::commit::SigningConfig::default(),
        branch_sort: Some(String::from("-committerdate")),
        reflog_subjects: vec![String::from("merge feature: Merge made by the 'ort' strategy.")],
        working_tree: working_tree(),
        operation: Some(Operation::Merge),
        rebase_stop: None,
        bisect: None,
        submodules: Vec::new(),
        shallow: model::state::Shallow::default(),
        skipped: Vec::new(),
        notes: model::note::Notes::default(),
        lfs: model::lfs::LfsStatus::default(),
        rerere: model::rerere::RerereState::default(),
        ..RepositoryState::default()
    }
}

/// The main worktree, and one whose folder is gone.
fn worktrees() -> Vec<Worktree> {
    vec![
        Worktree {
            folder: PathBuf::from("/work/app"),
            commit: Some(commit('a')),
            branch: Some(String::from("main")),
            is_main: true,
            is_current: true,
            is_missing: false,
            locked: None,
            uncommitted: None,
        },
        Worktree {
            folder: PathBuf::from("/work/app-fix"),
            commit: None,
            branch: None,
            is_main: false,
            is_current: false,
            is_missing: true,
            locked: Some(String::from("On the USB drive")),
            uncommitted: Some(3),
        },
    ]
}

/// A rename staged, a change not staged, a file not tracked and a conflict.
fn working_tree() -> WorkingTree {
    WorkingTree {
        staged: vec![FileChange {
            path: String::from("src/new.rs"),
            original_path: Some(String::from("src/old.rs")),
            kind: ChangeKind::Renamed,
        }],
        unstaged: vec![FileChange {
            path: String::from("README.md"),
            original_path: None,
            kind: ChangeKind::Modified,
        }],
        untracked: vec![String::from("notes.txt")],
        conflicted: vec![ConflictedFile {
            path: String::from("Cargo.lock"),
            kind: ConflictKind::BothModified,
        }],
    }
}

#[test]
fn the_state_of_a_repository_is_written_as_the_protocol_says_and_read_back() {
    let state = StateJson::from(&repository());
    assert_golden(golden("state.json"), pretty(&state));
    round_trip(&state);
    round_trip(&OpenResult { state: state.clone() });
    round_trip(&StateParams {
        repository: String::from("/work/app"),
        state,
        reason: StateReason::Changed,
    });
}

#[test]
fn a_line_of_the_log_is_a_word_or_a_read_with_how_long_it_took() {
    let step = LogEntry {
        time: Timestamp(5),
        kind: LogKind::Step,
        line: String::from("git checkout -b fix/login HEAD"),
    };
    let read = LogEntry {
        time: Timestamp(6),
        kind: LogKind::Read(Duration::from_millis(12)),
        line: String::from("git status"),
    };
    let entries = [LogEntryJson::from(&step), LogEntryJson::from(&read)];
    assert_eq!(
        serde_json::to_value(&entries).unwrap(),
        json!([
            { "time": 5, "kind": "step", "line": "git checkout -b fix/login HEAD" },
            { "time": 6, "kind": { "kind": "read", "milliseconds": 12 }, "line": "git status" }
        ])
    );
    for entry in &entries {
        round_trip(entry);
    }
}

#[test]
fn initialize_takes_what_the_extension_leaves_out_as_its_default() {
    let params: InitializeParams = serde_json::from_value(json!({
        "protocol": "1.0",
        "client": { "name": "gitardo-vscode", "version": "0.12.0" },
        "settings": { "graph.pageSize": 500 },
        "host": { "clipboard": true }
    }))
    .unwrap();
    assert_eq!(params.log_level, LogLevelName::Info);
    assert!(params.host.clipboard && !params.host.secrets);
    assert_eq!(params.settings.get("graph.pageSize"), Some(&json!(500)));
    round_trip(&params);
    let answer: InitializeResult = serde_json::from_value(json!({
        "protocol": "1.0",
        "engine": { "version": "0.1.0", "git": null }
    }))
    .unwrap();
    round_trip(&answer);
}

#[test]
fn a_version_is_spoken_when_its_major_number_is_the_engines() {
    assert!(is_compatible("1.0"));
    assert!(is_compatible("1.7"));
    assert!(is_compatible("1.8"));
    assert!(!is_compatible("2.0"));
    assert!(!is_compatible("10.0"));
    assert!(!is_compatible(""));
}

#[test]
fn a_read_of_the_state_is_a_refresh_only_when_it_says_so() {
    let asked: ReadStateParams = serde_json::from_value(json!({ "repository": "/work/app" })).unwrap();
    assert!(!asked.refresh, "an extension before 1.8 never says it");
    let refresh: ReadStateParams =
        serde_json::from_value(json!({ "repository": "/work/app", "refresh": true })).unwrap();
    assert!(refresh.refresh);
}
