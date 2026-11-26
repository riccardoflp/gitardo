//! The git folders of the repositories opened, watched by the engine itself (`repo.watch-git`, D93, version 1.15):
//! `repository/gitFolderChanged` saying what was written from outside, what is found by looking again when the watcher
//! gives nothing, a repository not shown compared once when `repository/shown` names it, and what a plan of the engine
//! writes not said at all.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};

/// The notification of the engine.
const CHANGED: &str = "repository/gitFolderChanged";
/// The request of the extension.
const SHOWN: &str = "repository/shown";

/// An engine with a repository opened.
fn opened(mut engine: Engine, repository: &Repository) -> Engine {
    let _ = engine.initialize();
    let answer = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    assert!(answer.get("result").is_some(), "{answer}");
    engine
}

/// The paths every notification of a list names.
fn paths_of(said: &[Value]) -> Vec<String> {
    said.iter()
        .flat_map(|notification| notification["params"]["paths"].as_array().cloned().unwrap_or_default())
        .filter_map(|path| path.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn repo_watch_git__a_reference_written_from_outside_is_said_with_its_path() {
    let repository = repository();
    let mut engine = opened(Engine::start(), &repository);
    let _made = repository.git(&["branch", "outside"]);
    let first = engine.notification_of(CHANGED);
    assert_eq!(first["params"]["repository"], json!(folder(&repository)), "{first}");
    assert_eq!(first["params"]["found"], json!(false), "{first}");
    let mut said = vec![first];
    said.extend(engine.notifications(CHANGED, Duration::from_millis(500)));
    let paths = paths_of(&said);
    assert!(
        paths.contains(&String::from(".git/refs/heads/outside")),
        "the branch is among {paths:?}"
    );
    assert!(
        paths
            .iter()
            .all(|path| path.starts_with(".git/") && !path.contains("index.lock")),
        "{paths:?}"
    );
}

#[test]
fn repo_watch_git__a_commit_in_a_worktree_is_said_to_its_repository_and_to_the_worktree_opened() {
    let repository = repository();
    let other = repository.path().with_file_name(format!(
        "{}-other",
        repository.path().file_name().unwrap().to_string_lossy()
    ));
    let other_text = other.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", "-b", "other", &other_text]);
    let mut engine = opened(Engine::start(), &repository);
    let answer = engine.ask("repository/open", &json!({ "repository": other_text }));
    assert!(answer.get("result").is_some(), "{answer}");
    let _quiet = engine.notifications(CHANGED, Duration::from_millis(300));
    std::fs::write(other.join("new.txt"), "new\n").unwrap();
    for arguments in [
        &["add", "new.txt"][..],
        &["commit", "--quiet", "-m", "Made in the worktree"],
    ] {
        let done = std::process::Command::new("git")
            .args(arguments)
            .current_dir(&other)
            .output()
            .unwrap();
        assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
    }
    let said = engine.notifications(CHANGED, Duration::from_millis(1_500));
    for name in [folder(&repository), other_text.clone()] {
        let of_it: Vec<Value> = said
            .iter()
            .filter(|notification| notification["params"]["repository"] == json!(name))
            .cloned()
            .collect();
        let paths = paths_of(&of_it);
        assert!(
            paths.contains(&String::from(".git/refs/heads/other")),
            "{name} is said the branch among {paths:?}"
        );
    }
    let _removed = repository.git(&["worktree", "remove", "--force", &other_text]);
}

#[test]
fn repo_watch_git__with_the_watcher_giving_nothing_a_commit_made_outside_is_found_within_three_seconds() {
    let mut repository = repository();
    let mut engine = opened(Engine::start_unwatched(), &repository);
    let shown = engine.ask(SHOWN, &json!({ "repositories": [folder(&repository)] }));
    assert_eq!(shown["result"], json!({ "changed": [] }), "{shown}");
    let _third = repository.commit("Made outside", &[("outside.txt", "outside\n")]);
    let committed = Instant::now();
    let found = engine.notification_of(CHANGED);
    assert!(
        committed.elapsed() < Duration::from_secs(3),
        "{:?}",
        committed.elapsed()
    );
    assert_eq!(
        found["params"],
        json!({ "repository": folder(&repository), "paths": [".git"], "found": true }),
        "{found}"
    );
    let again = engine.notifications(CHANGED, Duration::from_millis(2_500));
    assert_eq!(again, [] as [Value; 0], "it is found once");
}

#[test]
fn repo_watch_git__a_repository_not_shown_is_not_looked_at_again_and_is_compared_once_when_it_is_shown() {
    let repository = repository();
    let mut engine = opened(Engine::start_unwatched(), &repository);
    let _tagged = repository.git(&["tag", "outside"]);
    let said = engine.notifications(CHANGED, Duration::from_millis(2_500));
    assert_eq!(said, [] as [Value; 0], "no view shows it");
    let shown = engine.ask(SHOWN, &json!({ "repositories": [folder(&repository), "/not/open"] }));
    assert_eq!(shown["result"], json!({ "changed": [folder(&repository)] }), "{shown}");
    let again = engine.ask(SHOWN, &json!({ "repositories": [folder(&repository)] }));
    assert_eq!(again["result"], json!({ "changed": [] }), "{again}");
    let said = engine.notifications(CHANGED, Duration::from_millis(2_500));
    assert_eq!(said, [] as [Value; 0], "the answer said it");
    // Left and shown again with nothing written: nothing changed.
    let left = engine.ask(SHOWN, &json!({ "repositories": [] }));
    assert_eq!(left["result"], json!({ "changed": [] }), "{left}");
    let back = engine.ask(SHOWN, &json!({ "repositories": [folder(&repository)] }));
    assert_eq!(back["result"], json!({ "changed": [] }), "{back}");
}

#[test]
fn repo_watch_git__what_an_operation_of_the_engine_writes_is_not_said_and_what_is_written_after_it_is() {
    let repository = repository();
    let mut engine = opened(Engine::start(), &repository);
    let shown = engine.ask(SHOWN, &json!({ "repositories": [folder(&repository)] }));
    assert_eq!(shown["result"], json!({ "changed": [] }), "{shown}");
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" } }),
    );
    let finished = engine.answer_asked(id, |request| match request["params"]["question"]["key"].as_str() {
        Some("name") => json!({ "answer": { "kind": "text", "text": "topic" } }),
        Some("mode") => json!({ "answer": { "kind": "choice", "identifier": "checkout" } }),
        other => panic!("no answer for {other:?}"),
    });
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    assert_eq!(repository.git(&["symbolic-ref", "--short", "HEAD"]).trim(), "topic");
    let state = engine.notification_of("repository/state");
    assert_eq!(state["params"]["reason"], json!("ran"), "{state}");
    // Long enough for the watcher to say it and for the git folder to be looked at again.
    let said = engine.notifications(CHANGED, Duration::from_millis(3_000));
    assert_eq!(said, [] as [Value; 0], "the state sent after the plan says it");
    let _made = repository.git(&["branch", "outside"]);
    let after = engine.notification_of(CHANGED);
    assert_eq!(after["params"]["repository"], json!(folder(&repository)), "{after}");
}

#[test]
fn repo_watch_git__a_repository_closed_is_said_nothing_more() {
    let repository = repository();
    let mut engine = opened(Engine::start(), &repository);
    let closed = engine.ask("repository/close", &json!({ "repository": folder(&repository) }));
    assert!(closed.get("result").is_some(), "{closed}");
    let _made = repository.git(&["branch", "outside"]);
    let said = engine.notifications(CHANGED, Duration::from_millis(1_000));
    assert_eq!(said, [] as [Value; 0]);
}
