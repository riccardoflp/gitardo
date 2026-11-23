//! The state of a repository and the details of a commit as the engine writes them, with every optional part filled
//! and every list holding one entry: the golden files the extension's types are checked against, field for field and
//! kind for kind, by `extension/test/engineShapes.ts` (stage 6). A field added, removed or renamed here changes the
//! golden file, and then the extension's type check fails until its types say the same.

#![cfg(test)]

use std::path::{Path, PathBuf};

use model::bisect::Bisect;
use model::details::{CommitDetail, Signature};
use model::rebase::{RebaseStop, StopKind};
use model::state::{
    Branch, ChangeKind, ConflictKind, ConflictedFile, FileChange, Head, Identity, Operation, RemoteBranch,
    RepositoryState, Stash, Tag, Upstream, WorkingTree, Worktree,
};
use model::submodule::{Checkout, Distance, Submodule};
use model::{CommitId, Timestamp};
use protocol::reads::CommitDetailJson;
use protocol::state::StateJson;
use serde::Serialize;
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

/// A commit id made of one hexadecimal digit.
fn commit(digit: char) -> CommitId {
    CommitId::parse(&digit.to_string().repeat(40)).unwrap()
}

/// A file renamed, which fills the path it had.
fn renamed(path: &str, original: &str) -> FileChange {
    FileChange {
        path: String::from(path),
        original_path: Some(String::from(original)),
        kind: ChangeKind::Renamed,
    }
}

/// A repository in the middle of a rebase stopped to edit a commit, while a bisect is under way, with a submodule
/// checked out and every other part of the state there once.
fn repository() -> RepositoryState {
    RepositoryState {
        folder: PathBuf::from("/work/app"),
        git_folder: PathBuf::from("/work/app/.git"),
        head: Head {
            commit: Some(commit('a')),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![branch()],
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
        worktrees: vec![worktree()],
        remotes: vec![String::from("origin")],
        identity: Identity {
            name: Some(String::from("Ada Lovelace")),
            email: Some(String::from("ada@example.com")),
        },
        signing: signing(),
        branch_sort: Some(String::from("-committerdate")),
        reflog_subjects: vec![String::from("checkout: moving from feature to main")],
        working_tree: working_tree(),
        operation: Some(Operation::Rebase),
        rebase_stop: Some(RebaseStop {
            kind: StopKind::Edit,
            commit: commit('d'),
            subject: String::from("Add the parser"),
        }),
        bisect: Some(Bisect {
            bad: Some(commit('e')),
            good: vec![commit('f')],
            skipped: vec![commit('1')],
            remaining: Some(4),
        }),
        submodules: vec![submodule()],
        shallow: model::state::Shallow {
            ends: vec![commit('2')],
        },
        skipped: skipped(),
        is_looked_after: true,
        commit_graph: model::upkeep::CommitGraph::default(),
        notes: notes(),
        lfs: model::lfs::LfsStatus::default(),
        rerere: model::rerere::RerereState::default(),
        resolved_as_before: Vec::new(),
    }
}

/// One file hidden from git, skipped in the working tree.
fn skipped() -> Vec<model::state::SkippedFile> {
    vec![model::state::SkippedFile {
        path: String::from("config/local.json"),
        how: model::state::Skipped::Worktree,
    }]
}

/// Every commit signed with a key of SSH.
fn signing() -> model::commit::SigningConfig {
    model::commit::SigningConfig {
        signs_every_commit: true,
        kind: model::commit::SigningKind::Ssh,
        key: Some(String::from("~/.ssh/id_ed25519.pub")),
    }
}

/// One commit with a note.
fn notes() -> model::note::Notes {
    model::note::Notes {
        reference: String::from("refs/notes/commits"),
        tip: Some(commit('7')),
        noted: std::iter::once((commit('a'), commit('8'))).collect(),
    }
}

/// The branch checked out, which follows its remote branch.
fn branch() -> Branch {
    Branch {
        name: String::from("main"),
        commit: commit('a'),
        upstream: Some(Upstream {
            reference: String::from("refs/remotes/origin/main"),
            remote: String::from("origin"),
            remote_branch: String::from("refs/heads/main"),
            ahead: 1,
            behind: 2,
            is_gone: false,
        }),
        is_checked_out: true,
        date: Timestamp(1_700_000_000),
    }
}

/// The one worktree, with its commit, its branch and its count of what is not committed.
fn worktree() -> Worktree {
    Worktree {
        folder: PathBuf::from("/work/app"),
        commit: Some(commit('a')),
        branch: Some(String::from("refs/heads/main")),
        is_main: true,
        is_current: true,
        is_missing: false,
        locked: Some(String::from("On the USB drive")),
        uncommitted: Some(3),
    }
}

/// A rename staged, a rename not staged, a file not tracked and a conflict.
fn working_tree() -> WorkingTree {
    WorkingTree {
        staged: vec![renamed("src/new.rs", "src/old.rs")],
        unstaged: vec![renamed("docs/new.md", "docs/old.md")],
        untracked: vec![String::from("notes.txt")],
        conflicted: vec![ConflictedFile {
            path: String::from("Cargo.lock"),
            kind: ConflictKind::BothModified,
        }],
    }
}

/// A submodule of a submodule, with a branch to follow, a commit staged and a checkout that is one commit ahead.
fn submodule() -> Submodule {
    Submodule {
        name: String::from("inner"),
        folder: String::from("vendor/lib/inner"),
        parent: Some(String::from("vendor/lib")),
        url: Some(String::from("https://example.com/inner.git")),
        branch: Some(String::from("main")),
        recorded: Some(commit('2')),
        staged: Some(commit('3')),
        checkout: Some(Checkout {
            commit: Some(commit('4')),
            branch: Some(String::from("main")),
            uncommitted: 1,
            distance: Some(Distance { ahead: 1, behind: 0 }),
        }),
    }
}

/// A stash made with its untracked files: a commit with an author, a committer, parents and files of both kinds.
fn detail() -> CommitDetail {
    let signature = Signature {
        name: String::from("Ada Lovelace"),
        email: String::from("ada@example.com"),
        date: Timestamp(1_700_000_100),
    };
    CommitDetail {
        id: Some(commit('c')),
        parents: vec![commit('a'), commit('5'), commit('6')],
        author: Some(signature.clone()),
        committer: Some(signature),
        message: String::from("On main: half a fix\n\nThe rest tomorrow."),
        files: vec![renamed("src/new.rs", "src/old.rs")],
        untracked_files: vec![renamed("notes.txt", "notes.old")],
        note: Some(String::from(
            "Reviewed.
Shipped in 2.1.",
        )),
    }
}

#[test]
fn the_state_with_every_part_filled_is_written_as_the_extension_reads_it() {
    assert_golden(golden("state-filled.json"), pretty(&StateJson::from(&repository())));
}

#[test]
fn the_details_of_a_commit_with_every_part_filled_are_written_as_the_extension_reads_them() {
    assert_golden(
        golden("commit-detail-filled.json"),
        pretty(&CommitDetailJson::from(&detail())),
    );
}
