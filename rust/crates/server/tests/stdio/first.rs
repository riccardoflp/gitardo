//! The state in two steps, version 1.14 of the protocol (`repo.read-first`): `repository/open` asked with `first` and
//! `read/repo.read-first` answer the first step, the rest follows in `repository/state`, and a read of the state
//! asked meanwhile waits for it. What ran is counted from the commands git traced.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, commands_started, folder, repository, traced_commands};

/// A repository with another worktree, `other`, that has three files not committed.
fn with_another_worktree() -> Repository {
    let repository = repository();
    let other = repository.path().with_file_name("other");
    let _added = repository.git(&["worktree", "add", "--quiet", "--detach", &other.to_string_lossy()]);
    for name in ["x.txt", "y.txt", "z.txt"] {
        std::fs::write(other.join(name), "new\n").unwrap();
    }
    repository
}

/// How many times a command of git is among those started.
fn times(commands: &[String], command: &str) -> usize {
    commands.iter().filter(|started| *started == command).count()
}

/// The count of what is not committed in the other worktree, in a state.
fn count_of_other(state: &Value) -> &Value {
    &state["worktrees"][1]["uncommitted"]
}

/// The file git's trace of an engine is written in, in the git folder of a repository.
fn trace_of(repository: &Repository) -> std::path::PathBuf {
    repository.path().join(".git").join("trace.json")
}

/// The length of the trace so far: what is started after it is what a test counts.
fn traced_so_far(trace: &Path) -> usize {
    std::fs::read_to_string(trace).unwrap_or_default().len()
}

#[test]
fn a_repository_opened_with_first_answers_the_first_step_and_sends_the_rest_as_a_state() {
    let repository = with_another_worktree();
    let trace = trace_of(&repository);
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let opened = engine.ask(
        "repository/open",
        &json!({ "repository": folder(&repository), "first": true }),
    );
    let first = &opened["result"];
    assert_eq!(first["first"], json!(true), "{opened}");
    assert_eq!(first["state"]["head"]["branch"], json!("refs/heads/main"));
    assert_eq!(first["state"]["branches"].as_array().map(Vec::len), Some(2));
    assert_eq!(first["state"]["worktrees"][0]["uncommitted"], json!(0));
    assert_eq!(
        count_of_other(&first["state"]),
        &Value::Null,
        "no count yet, not a count of zero"
    );
    assert_eq!(first["state"]["remotes"], json!([]));

    let rest = engine.notification_of("repository/state");
    assert_eq!(rest["params"]["reason"], json!("rest"), "{rest}");
    assert_eq!(rest["params"]["repository"], json!(folder(&repository)));
    assert_eq!(count_of_other(&rest["params"]["state"]), &json!(3));
    assert_eq!(rest["params"]["state"]["identity"]["name"], json!("Test Author"));
    let more = engine.notifications("repository/state", Duration::from_millis(300));
    assert!(more.is_empty(), "the rest is sent once: {more:?}");

    let commands = traced_commands(&trace);
    for once in ["for-each-ref", "symbolic-ref", "stash", "worktree"] {
        assert_eq!(times(&commands, once), 1, "`git {once}` ran once: {commands:?}");
    }
    assert_eq!(
        times(&commands, "status"),
        2,
        "here, then in the other worktree: {commands:?}"
    );
}

#[test]
fn a_repository_opened_without_first_answers_the_whole_state_and_sends_no_rest() {
    let repository = with_another_worktree();
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    assert!(opened["result"].get("first").is_none(), "{opened}");
    assert_eq!(count_of_other(&opened["result"]["state"]), &json!(3));
    let states = engine.notifications("repository/state", Duration::from_millis(500));
    assert!(states.is_empty(), "{states:?}");
}

#[test]
fn a_state_asked_while_the_rest_is_read_waits_for_it_and_reads_nothing_again() {
    let repository = with_another_worktree();
    // git asks this hook before `git status` lists the changes: the rest, which counts the other worktree, takes a
    // second, during which the graph and the state are asked.
    let _ = repository.git(&["config", "core.fsmonitor", "sleep 1; true"]);
    let trace = trace_of(&repository);
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let opened = engine.ask(
        "repository/open",
        &json!({ "repository": folder(&repository), "first": true }),
    );
    assert_eq!(count_of_other(&opened["result"]["state"]), &Value::Null, "{opened}");

    let page = engine.ask(
        "read/graph.read-commits",
        &json!({ "repository": folder(&repository), "pageSize": 100 }),
    );
    assert_eq!(page["result"]["rows"].as_array().map(Vec::len), Some(2), "{page}");
    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(last["result"]["entry"], Value::Null, "{last}");

    let state = engine.ask(
        "read/repo.read-state",
        &json!({ "repository": folder(&repository), "refresh": true }),
    );
    assert_eq!(count_of_other(&state["result"]), &json!(3), "the whole state: {state}");
    let commands = traced_commands(&trace);
    for once in ["for-each-ref", "symbolic-ref", "stash", "worktree"] {
        assert_eq!(times(&commands, once), 1, "`git {once}` ran once: {commands:?}");
    }
    assert_eq!(times(&commands, "status"), 2, "no state was read twice: {commands:?}");
    assert_eq!(
        engine.notification_of("repository/state")["params"]["reason"],
        json!("rest")
    );
}

#[test]
fn the_first_step_of_a_repository_opened_is_read_and_the_rest_keeps_the_counts_unless_refresh_is_asked() {
    let repository = with_another_worktree();
    let trace = trace_of(&repository);
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let _opened = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    std::fs::write(repository.path().join("b.txt"), "new here\n").unwrap();

    let from = traced_so_far(&trace);
    let first = engine.ask("read/repo.read-first", &json!({ "repository": folder(&repository) }));
    assert_eq!(first["result"]["workingTree"]["untracked"], json!(["b.txt"]), "{first}");
    assert_eq!(count_of_other(&first["result"]), &Value::Null);
    let rest = engine.notification_of("repository/state");
    assert_eq!(rest["params"]["reason"], json!("rest"), "{rest}");
    assert_eq!(count_of_other(&rest["params"]["state"]), &json!(3), "the count kept");
    let commands = commands_started(&trace, from);
    assert_eq!(times(&commands, "status"), 1, "git status here only: {commands:?}");
    assert_eq!(times(&commands, "for-each-ref"), 1, "{commands:?}");

    let from = traced_so_far(&trace);
    let _first = engine.ask(
        "read/repo.read-first",
        &json!({ "repository": folder(&repository), "refresh": true }),
    );
    let _rest = engine.notification_of("repository/state");
    let commands = commands_started(&trace, from);
    assert_eq!(
        times(&commands, "status"),
        2,
        "Refresh counts every worktree: {commands:?}"
    );

    let unknown = engine.ask("read/repo.read-first", &json!({ "repository": "/nowhere" }));
    assert!(
        unknown["error"]["code"].is_i64(),
        "a repository not opened is an error: {unknown}"
    );
}
