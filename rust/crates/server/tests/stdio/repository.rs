//! The connection and the reads of stage 1: `initialize`, the errors of the protocol, the end of the process, a
//! repository opened, read, changed and closed, and a read cancelled.

use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};

use protocol::messages::VERSION;
use serde_json::{Value, json};

use crate::harness::{Engine, commands_started, folder, repository};

#[test]
fn initialize_answers_the_version_of_the_protocol_and_a_major_version_that_differs_with_its_error_and_the_engines_version()
 {
    let mut engine = Engine::start();
    let answer = engine.initialize();
    assert_eq!(answer["result"]["protocol"], json!(VERSION), "{answer}");
    assert_eq!(answer["result"]["engine"]["version"], json!(env!("CARGO_PKG_VERSION")));
    assert!(
        answer["result"]["engine"]["git"]
            .as_str()
            .is_some_and(|said| said.starts_with("git version"))
    );

    let refused = engine.ask(
        "initialize",
        &json!({ "protocol": "2.0", "client": { "name": "gitardo-vscode", "version": "9.0.0" } }),
    );
    assert_eq!(refused["error"]["code"], json!(-32_602), "{refused}");
    assert_eq!(refused["error"]["data"]["protocol"], json!(VERSION));
    assert_eq!(
        refused["error"]["data"]["engine"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn an_unknown_method_answers_32601_and_the_engine_keeps_running() {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let unknown = engine.ask("graph/teleport", &json!({}));
    assert_eq!(unknown["error"]["code"], json!(-32_601), "{unknown}");
    let not_json = {
        engine
            .input
            .as_mut()
            .unwrap()
            .write_all(b"Content-Length: 5\r\n\r\n{ no ")
            .unwrap();
        let _none = engine.notifications("never", Duration::from_millis(300));
        engine.kept.pop().unwrap()
    };
    assert_eq!(not_json["error"]["code"], json!(-32_700), "{not_json}");
    assert_eq!(engine.ask("shutdown", &Value::Null)["result"], Value::Null);
    assert!(engine.exit().success(), "exit after shutdown ends with 0");
}

#[test]
fn the_input_closed_without_shutdown_ends_the_engine_with_1() {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    assert_eq!(engine.close_input().code(), Some(1));
}

#[test]
fn open_answers_the_state_of_the_repository_and_read_state_the_same_until_it_is_closed() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    let state = &opened["result"]["state"];
    assert_eq!(state["head"]["branch"], json!("refs/heads/main"), "{opened}");
    let branches: Vec<&str> = state["branches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|branch| branch["name"].as_str().unwrap())
        .collect();
    assert_eq!(branches, ["feature", "main"]);
    assert_eq!(state["tags"][0]["name"], json!("v1.0"));
    assert_eq!(state["workingTree"]["untracked"], json!([]));

    let read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    assert_eq!(&read["result"], state);

    assert_eq!(
        engine.ask("repository/close", &json!({ "repository": folder(&repository) }))["result"],
        Value::Null
    );
    let closed = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    assert_eq!(closed["error"]["code"], json!(1), "{closed}");
}

#[test]
fn ten_changes_in_a_burst_send_one_state() {
    let repository = repository();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let _ = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    for round in 0..10 {
        std::fs::write(repository.path().join("b.txt"), format!("{round}\n")).unwrap();
        engine.notify(
            "repository/changed",
            &json!({ "repository": folder(&repository), "paths": ["b.txt"] }),
        );
        thread::sleep(Duration::from_millis(20));
    }
    let states = engine.notifications("repository/state", Duration::from_secs(4));
    assert_eq!(states.len(), 1, "{states:?}");
    let state = &states[0]["params"];
    assert_eq!(state["reason"], json!("changed"));
    assert_eq!(state["state"]["workingTree"]["untracked"], json!(["b.txt"]));
}

#[test]
fn cancel_of_a_read_that_runs_answers_cancelled() {
    let repository = repository();
    // git asks this hook before `git status` lists the changes, and the hook takes ten seconds.
    let _ = repository.git(&["config", "core.fsmonitor", "sleep 10; true"]);
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let id = engine.send("repository/open", &json!({ "repository": folder(&repository) }));
    thread::sleep(Duration::from_millis(500));
    let started = Instant::now();
    engine.notify("$/cancel", &json!({ "id": id }));
    let answer = engine.answer(id);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the read stops without waiting for the hook"
    );
}

#[test]
fn another_worktree_is_counted_again_only_when_its_git_folder_changed_or_on_refresh() {
    let repository = repository();
    let other = repository.path().with_file_name("other");
    let other_text = other.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", "--detach", &other_text]);
    let trace = repository.path().join(".git").join("trace.json");
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    let count = |state: &Value| state["worktrees"][1]["uncommitted"].clone();
    assert_eq!(count(&opened["result"]["state"]), json!(0), "{opened}");
    let statuses = |from: usize| {
        commands_started(&trace, from)
            .iter()
            .filter(|command| *command == "status")
            .count()
    };
    let read_state = |engine: &mut Engine, refresh: bool| {
        let read = engine.ask(
            "read/repo.read-state",
            &json!({ "repository": folder(&repository), "refresh": refresh }),
        );
        read["result"].clone()
    };

    std::fs::write(
        repository.path().join("a.txt"),
        "changed here
",
    )
    .unwrap();
    let before = std::fs::read_to_string(&trace).unwrap().len();
    let _state = read_state(&mut engine, false);
    assert_eq!(statuses(before), 1, "git status here only");

    std::fs::write(
        other.join("b.txt"),
        "new there
",
    )
    .unwrap();
    let _added = repository.git(&["-C", &other_text, "add", "b.txt"]);
    let before = std::fs::read_to_string(&trace).unwrap().len();
    let state = read_state(&mut engine, false);
    assert_eq!(statuses(before), 2, "git add there changed its index");
    assert_eq!(count(&state), json!(1));

    let before = std::fs::read_to_string(&trace).unwrap().len();
    let _state = read_state(&mut engine, true);
    assert_eq!(statuses(before), 2, "Refresh counts every worktree");
}
