//! What the folders the extension's Repositories view lists are on (`repo.worktree-summary`, milestone M22):
//! `read/repo.worktree-summary` answered once each folder asked and each worktree it lists was read,
//! `repository/folderChanged` reading one again, and `repository/summaries` saying what the read found.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use testkit::{Remote, Repository};

use crate::harness::{Engine, commands_started, folder, repository};

/// The method of the read.
const SUMMARY: &str = "read/repo.worktree-summary";
/// The notification of the engine.
const SUMMARIES: &str = "repository/summaries";
/// The notification of the extension.
const CHANGED: &str = "repository/folderChanged";

/// Whether two folders are the same, however the system and git spell them.
fn same(first: &str, second: &Path) -> bool {
    let key = |text: &str| text.replace('\\', "/").trim_end_matches('/').to_lowercase();
    key(first) == key(&second.to_string_lossy())
}

/// What an answer or a notification says of a folder.
fn said_of(summaries: &Value, wanted: &Path) -> Value {
    summaries
        .as_array()
        .unwrap()
        .iter()
        .find(|said| same(said["folder"].as_str().unwrap(), wanted))
        .cloned()
        .unwrap_or_else(|| panic!("{} is among {summaries}", wanted.display()))
}

/// Adds a worktree of a repository beside it, on a new branch, and gives its folder.
fn worktree(repository: &Repository, name: &str, branch: &str) -> PathBuf {
    let other = repository.path().with_file_name(name);
    let other_text = other.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", "-b", branch, &other_text]);
    other
}

#[test]
fn repo_worktree_summary__a_worktree_on_a_branch_with_files_changed_and_a_commit_to_push_says_all_three() {
    let mut repository = repository();
    let _remote = Remote::new(&repository);
    let _third = repository.commit("Change a again", &[("a.txt", "a, a third time\n"), ("b.txt", "b\n")]);
    fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    fs::write(repository.path().join("b.txt"), "changed too\n").unwrap();
    fs::write(repository.path().join("new.txt"), "new\n").unwrap();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let answer = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository)] }));
    let said = said_of(&answer["result"]["summaries"], repository.path());
    assert_eq!(
        said["summary"],
        json!({ "head": { "kind": "branch", "name": "main" }, "changed": 3, "ahead": 1, "behind": 0 }),
        "{answer}"
    );
    assert_eq!(said["isStale"], json!(false));
}

#[test]
fn repo_worktree_summary__a_detached_worktree_says_its_short_commit_and_a_folder_that_is_gone_says_missing() {
    let repository = repository();
    let commit = repository.git(&["rev-parse", "HEAD"]);
    repository.checkout(commit.trim());
    let gone = repository.path().with_file_name("gone");
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let answer = engine.ask(
        SUMMARY,
        &json!({ "folders": [folder(&repository), gone.to_string_lossy()] }),
    );
    let summaries = &answer["result"]["summaries"];
    let short: String = commit.trim().chars().take(7).collect();
    assert_eq!(
        said_of(summaries, repository.path())["summary"]["head"],
        json!({ "kind": "detached", "commit": short }),
        "{answer}"
    );
    assert_eq!(
        said_of(summaries, &gone)["summary"]["head"],
        json!({ "kind": "missing" })
    );
}

#[test]
fn repo_worktree_summary__each_repository_asked_lists_its_worktrees_and_each_says_what_it_is_on() {
    let repository = repository();
    let login = worktree(&repository, "api-login", "topic/login");
    fs::write(login.join("new.txt"), "new\n").unwrap();
    let web = Repository::new();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let answer = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository), folder(&web)] }));
    let summaries = &answer["result"]["summaries"];
    assert_eq!(summaries.as_array().unwrap().len(), 3, "{answer}");
    let api = said_of(summaries, repository.path());
    let listed = api["worktrees"].as_array().unwrap();
    assert_eq!(listed.len(), 2, "{api}");
    assert_eq!(listed[0]["isMain"], json!(true));
    assert_eq!(listed[1]["branch"], json!("refs/heads/topic/login"));
    assert_eq!(api["summary"]["changed"], json!(0));
    let other = said_of(summaries, &login);
    assert_eq!(
        other["summary"]["head"],
        json!({ "kind": "branch", "name": "topic/login" })
    );
    assert_eq!(other["summary"]["changed"], json!(1));
    assert_eq!(other["worktrees"], Value::Null, "a worktree listed lists none itself");
    assert_eq!(
        said_of(summaries, web.path())["summary"]["head"],
        json!({ "kind": "noCommits" })
    );

    // Asked again, nothing is read again: what each said is answered at once.
    let started = Instant::now();
    let again = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository), folder(&web)] }));
    assert_eq!(again["result"], answer["result"]);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "no folder waits for its two seconds"
    );
}

#[test]
fn repo_worktree_summary__a_file_saved_in_a_folder_followed_is_said_with_its_new_count() {
    let repository = repository();
    let login = worktree(&repository, "api-login", "topic/login");
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let _answer = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository)] }));
    let _first = engine.notifications(SUMMARIES, Duration::from_millis(100));

    fs::write(login.join("a.txt"), "changed in the worktree\n").unwrap();
    let started = Instant::now();
    engine.notify(
        CHANGED,
        &json!({ "folder": login.to_string_lossy(), "paths": ["a.txt"] }),
    );
    let said = engine.notification_of(SUMMARIES);
    let summaries = &said["params"]["summaries"];
    assert_eq!(summaries.as_array().unwrap().len(), 1, "{said}");
    assert_eq!(said_of(summaries, &login)["summary"]["changed"], json!(1), "{said}");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "within the two seconds between two reads"
    );
}

#[test]
fn repo_worktree_summary__nothing_changed_in_a_folder_runs_no_git_there_and_says_nothing() {
    let repository = repository();
    // Beside the repository: a file of its git folder that grows would be a change of it.
    let trace = repository.path().with_file_name("trace.json");
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let _answer = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository)] }));
    let _first = engine.notifications(SUMMARIES, Duration::from_millis(100));
    let before = fs::read_to_string(&trace).unwrap().len();

    // A file of the git folder the watchers saw, while nothing a summary reads changed; what never asks for a read;
    // and the index of a worktree, which a commit there writes and which lists no worktree anew.
    for path in [
        ".git/HEAD",
        ".git/index.lock",
        ".git/objects/ab/cdef",
        ".git/worktrees/api-login/index",
    ] {
        engine.notify(CHANGED, &json!({ "folder": folder(&repository), "paths": [path] }));
    }
    let said = engine.notifications(SUMMARIES, Duration::from_millis(2800));
    assert!(said.is_empty(), "{said:?}");
    assert_eq!(commands_started(&trace, before), Vec::<String>::new());

    // A change with no path is the extension asking for the read whatever changed.
    engine.notify(CHANGED, &json!({ "folder": folder(&repository), "paths": [] }));
    let said = engine.notifications(SUMMARIES, Duration::from_millis(1500));
    assert!(said.is_empty(), "the same summary is not said again: {said:?}");
    assert_eq!(commands_started(&trace, before), ["status", "worktree"]);
}

#[test]
fn repo_worktree_summary__a_worktree_added_is_followed_once_its_repository_is_listed_again() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let answer = engine.ask(SUMMARY, &json!({ "folders": [folder(&repository)] }));
    assert_eq!(answer["result"]["summaries"].as_array().unwrap().len(), 1, "{answer}");
    let _first = engine.notifications(SUMMARIES, Duration::from_millis(100));

    let login = worktree(&repository, "api-login", "topic/login");
    engine.notify(
        CHANGED,
        &json!({ "folder": folder(&repository), "paths": [".git/worktrees/api-login/HEAD"] }),
    );
    let listed = engine.notification_of(SUMMARIES);
    let api = said_of(&listed["params"]["summaries"], repository.path());
    assert_eq!(api["worktrees"].as_array().unwrap().len(), 2, "{listed}");
    let read = engine.notification_of(SUMMARIES);
    let other = said_of(&read["params"]["summaries"], &login);
    assert_eq!(
        other["summary"]["head"],
        json!({ "kind": "branch", "name": "topic/login" }),
        "{read}"
    );
}
