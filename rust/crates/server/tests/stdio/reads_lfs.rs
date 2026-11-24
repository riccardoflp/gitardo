//! The reads of Git LFS that version 1.13 adds for milestone M47, asked as the extension asks them. What needs no Git
//! LFS, a pointer said not downloaded and the patterns in the state, runs everywhere; what needs the program, its
//! version, its files and a content read from its store, is skipped where it is missing and says so (Q1198).

use std::process::Command;

use serde_json::json;
use testkit::Repository;

use crate::harness::folder;
use crate::reads::{bytes, opened, result};

/// The pointer Git LFS writes for a content of 2,000 bytes whose id is 64 times `a`.
const POINTER: &str = "version https://git-lfs.github.com/spec/v1\noid sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nsize 2000\n";

/// Whether the program Git LFS is installed; when it is not, the test that needs it says so and stops there.
fn has_lfs(test: &str) -> bool {
    let installed = Command::new("git")
        .args(["lfs", "version"])
        .output()
        .is_ok_and(|output| output.status.success());
    if !installed {
        eprintln!("{test}: skipped, Git LFS is missing");
    }
    installed
}

/// A repository whose `.gitattributes` stores `*.psd` with Git LFS, with the pointer of `art/logo.psd` committed as a
/// clone made without downloading has it, and the same text at `notes.txt`, a path it does not store with Git LFS.
fn with_pointers() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the pointers",
        &[
            (".gitattributes", "*.psd filter=lfs diff=lfs merge=lfs -text\n"),
            ("art/logo.psd", POINTER),
            ("notes.txt", POINTER),
        ],
    );
    repository
}

#[test]
fn a_pointer_whose_content_is_not_in_the_store_is_said_not_downloaded_with_its_size_and_its_id() {
    let repository = with_pointers();
    let mut engine = opened(&repository);
    for revision in [json!("HEAD"), json!(null)] {
        let read = result(
            &mut engine,
            &repository,
            "read/lfs.read-content",
            json!({ "path": "art/logo.psd", "revision": revision }),
        );
        assert_eq!(
            read["stored"],
            json!({ "size": 2000, "oid": "a".repeat(64), "isDownloaded": false }),
            "{revision}: {read}"
        );
        assert_eq!(
            bytes(&read["bytes"]),
            POINTER.as_bytes(),
            "the pointer, never downloaded by a read"
        );
        assert_eq!(read["commit"].is_string(), revision.is_string(), "{read}");
    }
}

#[test]
fn a_text_that_only_looks_like_a_pointer_at_a_path_not_stored_with_git_lfs_is_given_as_it_is() {
    let repository = with_pointers();
    let mut engine = opened(&repository);
    let read = result(
        &mut engine,
        &repository,
        "read/lfs.read-content",
        json!({ "path": "notes.txt", "revision": "HEAD" }),
    );
    assert_eq!(read["stored"], json!(null), "{read}");
    assert_eq!(bytes(&read["bytes"]), POINTER.as_bytes());
    let none = result(
        &mut engine,
        &repository,
        "read/lfs.read-content",
        json!({ "path": "gone.psd", "revision": "HEAD" }),
    );
    assert_eq!(none["bytes"], json!(null), "{none}");
}

#[test]
fn the_state_has_the_patterns_and_the_status_reads_them_with_the_version_when_git_lfs_is_there() {
    let repository = with_pointers();
    let mut engine = opened(&repository);
    let state = result(&mut engine, &repository, "read/repo.read-state", json!({}));
    assert_eq!(
        state["lfs"]["patterns"],
        json!([{ "pattern": "*.psd", "file": ".gitattributes", "line": 1 }]),
        "{}",
        state["lfs"]
    );
    let status = result(&mut engine, &repository, "read/lfs.read-status", json!({}));
    assert_eq!(status["patterns"], state["lfs"]["patterns"]);
    let installed = Command::new("git")
        .args(["lfs", "version"])
        .output()
        .is_ok_and(|output| output.status.success());
    assert_eq!(status["version"].is_string(), installed, "{status}");
}

#[test]
fn download_without_git_lfs_refuses_with_its_message_and_runs_nothing() {
    if Command::new("git")
        .args(["lfs", "version"])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("download_without_git_lfs_refuses_with_its_message_and_runs_nothing: skipped, Git LFS is installed");
        return;
    }
    let repository = with_pointers();
    let mut engine = opened(&repository);
    let finished = engine.ask(
        "feature/run",
        &json!({
            "repository": folder(&repository),
            "feature": "lfs.download",
            "target": { "kind": "lfsFiles", "paths": [{ "path": "art/logo.psd", "isFolder": false }], "commit": null }
        }),
    );
    assert_eq!(finished["result"]["kind"], json!("refused"), "{finished}");
    assert_eq!(
        finished["result"]["message"],
        json!("Git LFS is not installed: install it from git-lfs.com, then try again.")
    );
}

#[test]
fn with_git_lfs_the_files_are_listed_and_a_content_in_the_store_is_given_in_place_of_its_pointer() {
    if !has_lfs("with_git_lfs_the_files_are_listed_and_a_content_in_the_store_is_given_in_place_of_its_pointer") {
        return;
    }
    let mut repository = Repository::new();
    let _ = repository.git(&["lfs", "install", "--local"]);
    let _ = repository.commit(
        "Add the files",
        &[
            (".gitattributes", "*.bin filter=lfs diff=lfs merge=lfs -text\n"),
            ("a.bin", "first content\n"),
        ],
    );
    let mut engine = opened(&repository);
    let status = result(&mut engine, &repository, "read/lfs.read-status", json!({}));
    assert_eq!(status["isSetUp"], json!(true), "{status}");
    let files = result(
        &mut engine,
        &repository,
        "read/lfs.read-files",
        json!({ "version": status["version"] }),
    );
    assert_eq!(files["files"][0]["path"], json!("a.bin"), "{files}");
    assert_eq!(files["files"][0]["size"], json!(14));
    assert_eq!(files["files"][0]["isDownloaded"], json!(true));
    let read = result(
        &mut engine,
        &repository,
        "read/lfs.read-content",
        json!({ "path": "a.bin", "revision": "HEAD" }),
    );
    assert_eq!(read["stored"]["isDownloaded"], json!(true), "{read}");
    assert_eq!(bytes(&read["bytes"]), b"first content\n");
}
