//! The reads of stage 5 that look at the conflicts and at what a repository holds besides its commits, a read
//! cancelled, reads that run at the same time, and the line each read leaves in the log.

use std::thread;
use std::time::{Duration, Instant};

use protocol::messages::VERSION;
use serde_json::{Value, json};
use testkit::{CONFLICT_FILE, Repository};

use crate::harness::{Engine, folder, repository};
use crate::reads::{bytes, opened, read, result};

/// An engine initialized with the host abilities and the log level given, with a repository opened.
fn opened_with(repository: &Repository, host: &Value, log_level: &str) -> Engine {
    let mut engine = Engine::start();
    let initialized = engine.ask(
        "initialize",
        &json!({
            "protocol": VERSION,
            "client": { "name": "gitardo-vscode", "version": "0.0.0" },
            "settings": {},
            "host": host,
            "logLevel": log_level
        }),
    );
    assert!(initialized.get("result").is_some(), "{initialized}");
    let _ = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

#[test]
fn a_merge_stopped_on_a_conflict_has_its_files_its_sides_their_names_and_what_the_rules_propose() {
    let mut repository = Repository::new();
    repository.conflict();
    let mut engine = opened(&repository);
    let files = result(&mut engine, &repository, "read/conflicts.read-files", Value::Null);
    assert_eq!(files[0]["path"], json!(CONFLICT_FILE), "{files}");
    assert!(
        files[0]["base"]["object"].is_string() && files[0]["current"].is_object() && files[0]["incoming"].is_object()
    );
    let sides = result(
        &mut engine,
        &repository,
        "read/merge-view.read-sides",
        json!({ "path": CONFLICT_FILE, "unsaved": null }),
    );
    assert_eq!(
        (
            bytes(&sides["base"]),
            bytes(&sides["current"]),
            bytes(&sides["incoming"])
        ),
        (b"base\n".to_vec(), b"ours\n".to_vec(), b"theirs\n".to_vec())
    );
    assert_eq!(sides["operation"], json!("merge"), "{sides}");
    assert_eq!(sides["hasBothSides"], json!(true));
    let names = result(&mut engine, &repository, "read/conflicts.side-names", Value::Null);
    assert_eq!(names["current"]["role"], json!("branchCheckedOut"), "{names}");
    assert_eq!(names["current"]["name"], json!("main"));
    assert_eq!(names["incoming"]["role"], json!("branchBeingMerged"));
    let proposals = result(
        &mut engine,
        &repository,
        "read/conflicts.resolve-by-rules",
        json!({ "paths": null }),
    );
    assert_eq!(proposals[0]["path"], json!(CONFLICT_FILE), "{proposals}");
    assert_eq!(proposals[0]["leftAlone"], Value::Null);
    assert_eq!(
        proposals[0]["conflicts"][0]["proposal"],
        Value::Null,
        "both sides changed the same line"
    );
    let none = result(
        &mut engine,
        &repository,
        "read/conflicts.resolve-by-rules",
        json!({ "paths": ["other.txt"] }),
    );
    assert_eq!(none, json!([]));
}

#[test]
fn the_reflog_the_remotes_the_identity_and_the_submodules() {
    let repository = repository();
    let _ = repository.git(&["remote", "add", "origin", "https://example.com/app.git"]);
    let mut engine = opened(&repository);
    let reflog = result(
        &mut engine,
        &repository,
        "read/reflog.read",
        json!({ "reference": "HEAD", "count": 1 }),
    );
    assert_eq!(reflog.as_array().map(Vec::len), Some(1), "{reflog}");
    assert_eq!(reflog[0]["selector"], json!("HEAD@{0}"));
    let remotes = result(&mut engine, &repository, "read/remote.read-remotes", Value::Null);
    assert_eq!(
        remotes,
        json!([{ "name": "origin", "url": "https://example.com/app.git" }])
    );
    let identity = result(&mut engine, &repository, "read/identity.read", Value::Null);
    assert!(identity["identity"]["name"].is_string(), "{identity}");
    let keys: Vec<&str> = identity["localKeys"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        keys.contains(&"user.name") && keys.contains(&"user.email"),
        "{identity}"
    );
    let submodules = result(&mut engine, &repository, "read/submodule.read-list", Value::Null);
    assert_eq!(submodules, json!([]));
}

/// A repository with a package committed, its packages installed in `node_modules` and a `.env`, both ignored.
fn with_packages() -> Repository {
    let mut repository = repository();
    let _ = repository.commit(
        "Add the package",
        &[
            (".gitignore", "node_modules/\n.env\n"),
            ("package.json", "{}\n"),
            ("package-lock.json", "{}\n"),
        ],
    );
    std::fs::create_dir_all(repository.path().join("node_modules").join("left-pad")).unwrap();
    std::fs::write(
        repository.path().join("node_modules").join("left-pad").join("index.js"),
        "x\n",
    )
    .unwrap();
    std::fs::write(repository.path().join(".env"), "KEY=1\n").unwrap();
    repository
}

#[test]
fn what_new_worktrees_get_is_what_git_ignores_and_what_the_patterns_take() {
    let repository = with_packages();
    let mut engine = opened(&repository);
    let worktree = folder(&repository);
    let ignored = result(
        &mut engine,
        &repository,
        "read/worktree-setup.read-ignored",
        json!({ "worktree": worktree }),
    );
    let paths: Vec<&str> = ignored
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["path"].as_str())
        .collect();
    assert_eq!(paths, [".env", "node_modules"], "{ignored}");
    assert_eq!(ignored[1]["kind"]["kind"], json!("folder"));
    let takes = result(
        &mut engine,
        &repository,
        "read/worktree-setup.read-what-patterns-take",
        json!({ "requests": [{ "folder": worktree, "link": ["node_modules"], "copy": [".env"], "commit": null }] }),
    );
    assert_eq!(takes[0]["linked"], json!(["node_modules"]), "{takes}");
    assert_eq!(takes[0]["copied"], json!([".env"]));
}

#[test]
fn the_commands_proposed_what_a_path_is_and_the_links_of_a_worktree() {
    let repository = with_packages();
    let mut engine = opened(&repository);
    let worktree = folder(&repository);
    let commands = result(
        &mut engine,
        &repository,
        "read/worktree-setup.propose-commands",
        Value::Null,
    );
    assert_eq!(
        commands["commands"],
        json!([{ "command": "npm ci", "file": "package-lock.json" }]),
        "{commands}"
    );
    let statuses = result(
        &mut engine,
        &repository,
        "read/worktree-setup.explain-change",
        json!({ "paths": ["node_modules", "a.txt"] }),
    );
    assert_eq!(
        statuses[0],
        json!({ "isFolder": true, "tracked": "nothing", "ignored": "whole" }),
        "{statuses}"
    );
    assert_eq!(
        statuses[1],
        json!({ "isFolder": false, "tracked": "it", "ignored": "nothing" })
    );
    let links = result(
        &mut engine,
        &repository,
        "read/worktree.read-folder-links",
        json!({ "worktree": worktree }),
    );
    assert_eq!(links["paths"], json!([]));
}

#[test]
fn the_pull_requests_are_not_on_github_or_have_no_token_and_the_token_is_asked_only_of_a_host_that_gives_it() {
    let repository = repository();
    let mut engine = opened(&repository);
    let none = result(
        &mut engine,
        &repository,
        "read/github.read-pull-requests",
        json!({ "remote": null }),
    );
    assert_eq!(none, json!({ "kind": "notOnGitHub" }));
    let _ = repository.git(&["remote", "add", "origin", "https://github.com/owner/app.git"]);
    let silent = result(
        &mut engine,
        &repository,
        "read/github.read-pull-requests",
        json!({ "remote": null }),
    );
    assert_eq!(
        silent,
        json!({ "kind": "noToken" }),
        "a host that did not say it gives the token is not asked"
    );

    let mut engine = opened_with(&repository, &json!({ "githubToken": true }), "info");
    let mut params = json!({ "remote": "origin", "repository": folder(&repository) });
    let id = engine.send("read/github.read-pull-requests", &params);
    let mut asked = Vec::new();
    let answer = engine.answer_asked(id, |request| {
        asked.push(request.clone());
        json!({ "token": null })
    });
    assert_eq!(answer["result"], json!({ "kind": "noToken" }), "{answer}");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0]["method"], json!("host/githubToken"));
    assert_eq!(asked[0]["params"], json!({ "host": "github.com", "ask": false }));
    params["remote"] = json!("upstream");
    let other = engine.ask("read/github.read-pull-requests", &params);
    assert_eq!(other["result"], json!({ "kind": "notOnGitHub" }), "{other}");
}

#[test]
fn a_read_of_a_repository_not_open_answers_1() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let answer = read(&mut engine, &repository, "read/remote.read-remotes", Value::Null);
    assert_eq!(answer["error"]["code"], json!(1), "{answer}");
}

#[test]
fn cancel_of_a_read_that_runs_answers_cancelled_while_other_reads_are_answered() {
    let repository = repository();
    let mut engine = opened(&repository);
    // git asks this hook before `git status` lists the changes, and the hook takes ten seconds.
    let _ = repository.git(&["config", "core.fsmonitor", "sleep 10; true"]);
    let slow = engine.send(
        "read/changes.read-working-tree",
        &json!({ "repository": folder(&repository) }),
    );
    thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let quick = result(&mut engine, &repository, "read/remote.read-remotes", Value::Null);
    assert_eq!(quick, json!([]));
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "a read does not wait for another"
    );
    engine.notify("$/cancel", &json!({ "id": slow }));
    let answer = engine.answer(slow);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "the read stops without waiting for the hook"
    );
}

#[test]
fn each_read_leaves_a_line_with_its_method_and_how_long_it_took_when_the_log_says_the_reads() {
    let repository = repository();
    let mut engine = opened_with(&repository, &json!({}), "debug");
    let _ = engine.notifications("log/line", Duration::from_millis(300));
    let _ = result(&mut engine, &repository, "read/remote.read-remotes", Value::Null);
    let lines = engine.notifications("log/line", Duration::from_millis(500));
    let line = lines
        .iter()
        .find(|line| {
            line["params"]["entry"]["line"]
                .as_str()
                .is_some_and(|said| said.starts_with("Read read/remote.read-remotes in "))
        })
        .unwrap_or_else(|| panic!("{lines:?}"));
    assert_eq!(line["params"]["entry"]["kind"]["kind"], json!("read"), "{line}");
    assert!(line["params"]["entry"]["kind"]["milliseconds"].is_u64(), "{line}");
}

#[test]
fn the_files_of_a_burst_that_ask_for_a_read_leave_out_what_git_ignores_and_nested_worktrees() {
    let repository = repository();
    std::fs::write(repository.path().join(".gitignore"), "target/\n").unwrap();
    let nested = repository.path().join(".claude").join("worktrees").join("feature");
    let nested_text = nested.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", "--detach", &nested_text]);
    let mut engine = opened(&repository);
    let asked = json!({ "paths": [
        "target/debug/app.exe",
        "src/main.rs",
        ".gitignore",
        ".claude/worktrees/feature/src/a.rs",
        ".claude/worktrees/feature-two/a.rs"
    ] });
    let found = result(&mut engine, &repository, "read/repo.watch-changes", asked);
    assert_eq!(
        found["paths"],
        json!([".gitignore", "src/main.rs", ".claude/worktrees/feature-two/a.rs"]),
        "{found}"
    );
}
