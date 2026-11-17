//! The reads of the graph, from stage 7: a page of the timeline and the next one with `skip`, the stashes among the
//! commits, the history of a file, where a commit is, what a search finds, and a page cancelled with `$/cancel`.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};
use crate::reads::{opened, read, result};

/// The full id of a revision.
fn id(repository: &Repository, revision: &str) -> String {
    repository.git(&["rev-parse", revision]).trim_end().to_owned()
}

/// The ids of the rows of a page, a stash as `stash@{<index>}`.
fn rows(page: &Value) -> Vec<String> {
    page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| match row["kind"].as_str() {
            Some("stash") => format!("stash@{{{}}}", row["index"]),
            _ => row["commit"]["id"].as_str().unwrap().to_owned(),
        })
        .collect()
}

/// The commits of `git log` over every branch, as the graph lists them.
fn listed(repository: &Repository) -> Vec<String> {
    repository
        .git(&[
            "log",
            "--format=%H",
            "--date-order",
            "--branches",
            "--remotes",
            "--tags",
            "HEAD",
            "--",
        ])
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_page_of_the_timeline_then_the_next_one_from_the_last_row_the_client_has() {
    let mut repository = repository();
    for number in 0..120 {
        let _ = repository.commit(&format!("Commit {number}"), &[]);
    }
    let mut engine = opened(&repository);
    let all = listed(&repository);
    assert_eq!(all.len(), 122);

    let first = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100 }),
    );
    assert_eq!(rows(&first), all[..100], "the newest hundred, newest first");
    assert_eq!(first["hasMore"], json!(true));
    let row = &first["rows"][0];
    assert_eq!(row["kind"], json!("commit"));
    assert_eq!(row["commit"]["parents"], json!([all[1]]));
    assert_eq!(row["commit"]["subject"], json!("Commit 119"));
    assert!(
        row["commit"]["authorName"].is_string() && row["commit"]["authorDate"].is_i64(),
        "{row}"
    );

    let next = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100, "pages": 2, "skip": 99 }),
    );
    assert_eq!(
        rows(&next),
        all[99..],
        "the last row the client has, then only the new ones"
    );
    assert_eq!(next["hasMore"], json!(false));
}

#[test]
fn the_stashes_are_among_the_commits_when_asked_and_never_in_the_history_of_a_file() {
    let mut repository = repository();
    std::fs::write(repository.path().join("a.txt"), "changed, not committed\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "-m", "Kept"]);
    let _ = repository.commit("Add b", &[("b.txt", "b\n")]);
    let mut engine = opened(&repository);
    let head = id(&repository, "HEAD");
    let base = id(&repository, "HEAD~1");

    let page = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100, "stashes": true }),
    );
    let ids = rows(&page);
    let stash = ids
        .iter()
        .position(|row| row == "stash@{0}")
        .unwrap_or_else(|| panic!("{ids:?}"));
    let base_row = ids.iter().position(|row| *row == base);
    assert!(
        base_row.is_some_and(|row| row > stash),
        "above the commit it was made on: {ids:?}"
    );
    let stash_row = page["rows"].get(stash).unwrap();
    assert_eq!(stash_row["commit"]["parents"], json!([base]), "only its first parent");

    let without = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100 }),
    );
    assert!(!rows(&without).contains(&String::from("stash@{0}")));

    let file = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({
            "pageSize": 100,
            "stashes": true,
            "narrowing": { "kind": "file", "path": "b.txt" }
        }),
    );
    assert_eq!(rows(&file), [head], "the history of b.txt, with no stash");
}

#[test]
fn the_branches_shown_and_the_first_parents_choose_the_commits_listed() {
    let mut repository = repository();
    repository.checkout("feature");
    let side = repository.commit("On feature", &[("f.txt", "f\n")]);
    repository.checkout("main");
    repository.merge("feature");
    let mut engine = opened(&repository);
    let merge = id(&repository, "HEAD");

    let first_parent = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100, "parents": "first", "shown": { "kind": "current" } }),
    );
    let ids = rows(&first_parent);
    assert_eq!(ids.first(), Some(&merge));
    assert!(!ids.contains(&side), "the merged commit is not followed: {ids:?}");
    assert_eq!(
        first_parent["rows"][0]["commit"]["parents"].as_array().map(Vec::len),
        Some(1)
    );

    let ticked = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100, "shown": { "kind": "references", "names": ["refs/heads/feature"] } }),
    );
    assert_eq!(rows(&ticked).first(), Some(&side), "the history of feature alone");
}

#[test]
fn the_position_of_a_commit_in_the_timeline_or_null_when_it_is_not_there() {
    let mut repository = repository();
    repository.checkout("feature");
    let side = repository.commit("On feature", &[("f.txt", "f\n")]);
    repository.checkout("main");
    let mut engine = opened(&repository);
    let all = listed(&repository);
    let expected = all.iter().position(|commit| *commit == side);

    let everything = json!({ "pageSize": 1000 });
    let found = result(
        &mut engine,
        &repository,
        "read/graph.read-position",
        json!({ "commit": side, "timeline": everything }),
    );
    assert_eq!(found["index"], json!(expected));

    let current = json!({ "pageSize": 1000, "shown": { "kind": "current" } });
    let missing = result(
        &mut engine,
        &repository,
        "read/graph.read-position",
        json!({ "commit": side, "timeline": current }),
    );
    assert_eq!(missing["index"], Value::Null, "not on what is checked out");

    let short = read(
        &mut engine,
        &repository,
        "read/graph.read-position",
        json!({ "commit": &side[..7], "timeline": everything }),
    );
    assert_eq!(short["error"]["code"], json!(-32_602), "{short}");
}

#[test]
fn a_search_finds_the_commits_of_the_timeline_and_a_pattern_git_refuses_is_its_failure() {
    let repository = repository();
    let mut engine = opened(&repository);
    let changed = id(&repository, "HEAD");
    let timeline = json!({ "pageSize": 1000 });
    let found = result(
        &mut engine,
        &repository,
        "read/search.read-matches",
        json!({ "text": "CHANGE", "mode": "message", "timeline": timeline }),
    );
    assert_eq!(found, json!({ "commits": [changed], "hasMore": false }));
    let in_changes = result(
        &mut engine,
        &repository,
        "read/search.read-matches",
        json!({ "text": "again", "mode": "changes", "timeline": timeline }),
    );
    assert_eq!(in_changes["commits"], json!([changed]));

    let refused = read(
        &mut engine,
        &repository,
        "read/search.read-matches",
        json!({ "text": "(", "mode": "changesPattern", "timeline": timeline }),
    );
    assert_eq!(refused["error"]["code"], json!(2), "{refused}");
}

#[test]
fn cancel_of_a_page_that_runs_answers_cancelled() {
    let repository = repository();
    // A search shown as a list reads the changes of each commit, and the conversion of the text takes ten seconds.
    std::fs::write(repository.path().join(".gitattributes"), "*.txt diff=slow\n").unwrap();
    let _ = repository.git(&["config", "diff.slow.textconv", "sleep 10; cat"]);
    let mut engine = opened(&repository);
    let slow = engine.send(
        "read/graph.read-commits",
        &json!({
            "repository": folder(&repository),
            "pageSize": 100,
            "narrowing": { "kind": "search", "mode": "changesPattern", "text": "again" }
        }),
    );
    thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    engine.notify("$/cancel", &json!({ "id": slow }));
    let answer = engine.answer(slow);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "the read stops without waiting for git"
    );
}

/// The commands git was run as, from a byte of a file of trace2 events on: not those git ran itself.
fn commands_started(trace: &std::path::Path, from: usize) -> Vec<String> {
    let text = std::fs::read_to_string(trace).unwrap_or_default();
    text.get(from..)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let event: Value = serde_json::from_str(line).ok()?;
            let is_started = event["event"] == "start" && !event["sid"].as_str()?.contains('/');
            is_started.then(|| command_of(event["argv"].as_array()?))?
        })
        .collect()
}

/// The command of git's arguments: the first one that is not an option of git itself, as `-c name=value`.
fn command_of(argv: &[Value]) -> Option<String> {
    let mut arguments = argv.iter().skip(1).filter_map(Value::as_str);
    while let Some(argument) = arguments.next() {
        match argument {
            "-c" | "-C" => {
                let _value = arguments.next();
            }
            option if option.starts_with('-') => {}
            command => return Some(command.to_owned()),
        }
    }
    None
}

#[test]
fn asked_right_after_repo_read_state_a_read_of_the_commits_runs_git_log_and_nothing_else() {
    let mut repository = repository();
    for number in 0..5 {
        let _ = repository.commit(&format!("Commit {number}"), &[]);
    }
    let trace = repository.path().join(".git").join("trace.json");
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    assert!(opened.get("result").is_some(), "{opened}");
    let _state = result(&mut engine, &repository, "read/repo.read-state", Value::Null);
    let before = std::fs::read_to_string(&trace).unwrap().len();
    let page = result(
        &mut engine,
        &repository,
        "read/graph.read-commits",
        json!({ "pageSize": 100, "stashes": true }),
    );
    assert_eq!(page["rows"].as_array().unwrap().len(), 7);
    assert_eq!(commands_started(&trace, before), ["log"]);
}
