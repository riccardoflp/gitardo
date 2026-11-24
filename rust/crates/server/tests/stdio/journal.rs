//! Undo of what the engine ran, kept in its journal, and of what was done outside it, read from the reflogs, all taken
//! back by `feature/run` of `undo.last`; and the label of Undo, `journal/last`, checked against the state the last
//! read found.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository, traced_commands};

/// The engine started, initialized and the repository opened.
fn opened(repository: &Repository) -> Engine {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let _state = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

/// What is checked out: the commit and the branch.
fn head(repository: &Repository) -> Value {
    let commit = repository.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let branch = repository.git(&["symbolic-ref", "-q", "HEAD"]).trim().to_owned();
    json!({ "commit": commit, "branch": if branch.is_empty() { Value::Null } else { json!(branch) } })
}

/// Runs a feature, confirming its plan, and gives how it ended.
fn run(engine: &mut Engine, repository: &Repository, feature: &str, target: &Value, answers: &Value) -> Value {
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(repository), "feature": feature, "target": target, "answers": answers }),
    );
    let answer = engine.answer_asked(id, |request| match request["method"].as_str() {
        Some("feature/confirm") => json!({ "run": true }),
        _ => panic!("{feature} asks nothing but to confirm: {request}"),
    });
    assert_eq!(answer["result"]["kind"], json!("ran"), "{answer}");
    answer["result"].clone()
}

/// Runs `undo.last`, confirming its plan, and gives how it ended.
fn undo(engine: &mut Engine, repository: &Repository) -> Value {
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(repository), "feature": "undo.last", "target": { "kind": "head" } }),
    );
    let answer = engine.answer_asked(id, |request| match request["method"].as_str() {
        Some("feature/confirm") => json!({ "run": true }),
        _ => panic!("Undo asks nothing but to confirm: {request}"),
    });
    assert!(answer.get("result").is_some(), "{answer}");
    answer["result"].clone()
}

#[test]
fn a_branch_deleted_then_one_created_by_the_engine_are_taken_back_newest_first() {
    let repository = repository();
    let mut engine = opened(&repository);
    let commit = repository.git(&["rev-parse", "feature"]).trim().to_owned();
    let _deleted = run(
        &mut engine,
        &repository,
        "branch.delete",
        &json!({ "kind": "branch", "name": "feature" }),
        &json!({ "merged": { "kind": "flag", "value": false } }),
    );
    assert_eq!(repository.git(&["branch", "--list", "feature"]).trim(), "");
    let _created = run(
        &mut engine,
        &repository,
        "branch.create",
        &json!({ "kind": "head" }),
        &json!({ "name": { "kind": "text", "text": "spike" }, "mode": { "kind": "choice", "identifier": "only" } }),
    );

    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(
        last["result"]["entry"]["title"],
        json!("Create branch spike at HEAD"),
        "{last}"
    );
    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["branch", "--list", "spike"]).trim(), "");
    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["rev-parse", "feature"]).trim(), commit);
}

#[test]
fn a_stash_dropped_by_the_engine_is_stored_again_by_undo() {
    let repository = repository();
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let _stashed = repository.git(&["stash", "push", "-m", "wip"]);
    let stash = repository.git(&["rev-parse", "stash@{0}"]).trim().to_owned();
    let mut engine = opened(&repository);
    let _dropped = run(
        &mut engine,
        &repository,
        "stash.drop",
        &json!({ "kind": "stash", "index": 0 }),
        &json!({}),
    );
    assert_eq!(repository.git(&["stash", "list"]).trim(), "");

    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["rev-parse", "stash@{0}"]).trim(), stash);
}

#[test]
fn a_commit_made_outside_after_an_operation_of_the_engine_drops_the_journal_and_undo_goes_by_the_reflog() {
    let mut repository = repository();
    let mut engine = opened(&repository);
    let _created = run(
        &mut engine,
        &repository,
        "branch.create",
        &json!({ "kind": "head" }),
        &json!({ "name": { "kind": "text", "text": "spike" }, "mode": { "kind": "choice", "identifier": "only" } }),
    );
    let before = head(&repository);
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    // The extension reads the state again once its watchers saw the commit, and asks for the label of Undo after it.
    let _read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));

    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(last["result"]["entry"], Value::Null, "{last}");
    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(head(&repository), before, "the commit is taken back");
    assert_ne!(
        repository.git(&["branch", "--list", "spike"]).trim(),
        "",
        "the branch stays"
    );
}

#[test]
fn undo_during_a_merge_stopped_on_conflicts_refuses_with_the_engines_sentence() {
    let mut repository = repository();
    repository.conflict();
    let mut engine = opened(&repository);
    let refused = undo(&mut engine, &repository);
    assert_eq!(refused["kind"], json!("refused"), "{refused}");
    assert_eq!(refused["message"], json!("Finish or abort the merge first."));
}

#[test]
fn a_new_engine_has_an_empty_journal_and_undo_goes_by_the_reflog() {
    let mut repository = repository();
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    let parent = repository.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let mut engine = opened(&repository);
    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(last["result"]["entry"], Value::Null, "{last}");
    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["rev-parse", "HEAD"]).trim(), parent);
}

#[test]
fn undo_last__journal_last_asked_right_after_read_state_runs_no_git() {
    let repository = repository();
    let trace = repository.path().join(".git").join("trace.txt");
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let _state = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    let _created = run(
        &mut engine,
        &repository,
        "branch.create",
        &json!({ "kind": "head" }),
        &json!({ "name": { "kind": "text", "text": "spike" }, "mode": { "kind": "choice", "identifier": "only" } }),
    );
    let _read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    let before = traced_commands(&trace).len();

    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(
        last["result"]["entry"]["title"],
        json!("Create branch spike at HEAD"),
        "{last}"
    );
    let after = traced_commands(&trace);
    assert_eq!(after.len(), before, "no git ran: {:?}", after.get(before..));
}

#[test]
fn undo_last__journal_last_after_an_operation_takes_the_state_read_after_it_and_reads_no_more() {
    let repository = repository();
    let trace = repository.path().join(".git").join("trace.txt");
    let mut engine = Engine::start_traced(&trace);
    let _ = engine.initialize();
    let _state = engine.ask("repository/open", &json!({ "repository": folder(&repository) }));
    let before_operation = traced_commands(&trace).len();
    let _created = run(
        &mut engine,
        &repository,
        "branch.create",
        &json!({ "kind": "head" }),
        &json!({ "name": { "kind": "text", "text": "spike" }, "mode": { "kind": "choice", "identifier": "only" } }),
    );
    let ran = traced_commands(&trace);
    assert!(
        ran.len() > before_operation,
        "the operation ran, and the state was read after it"
    );

    let last = engine.ask("journal/last", &json!({ "repository": folder(&repository) }));
    assert_eq!(
        last["result"]["entry"]["title"],
        json!("Create branch spike at HEAD"),
        "{last}"
    );
    assert_eq!(
        traced_commands(&trace).len(),
        ran.len(),
        "the state read after the operation is the one checked"
    );
}

#[test]
fn undo_last__right_after_a_stash_it_says_why_it_cannot_rather_than_take_back_the_commit_before() {
    let mut repository = repository();
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    let before = head(&repository);
    let mut engine = opened(&repository);
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let _read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    let _stashed = run(
        &mut engine,
        &repository,
        "stash.push",
        &json!({ "kind": "head" }),
        &json!({ "message": { "kind": "text", "text": "wip" } }),
    );
    assert_ne!(repository.git(&["stash", "list"]).trim(), "");

    let refused = undo(&mut engine, &repository);
    assert_eq!(refused["kind"], json!("refused"), "{refused}");
    assert_eq!(head(&repository), before, "the commit before the stash stays");
}

#[test]
fn undo_last__right_after_a_push_it_says_why_it_cannot_rather_than_take_back_the_commit_pushed() {
    let mut repository = repository();
    let _remote = testkit::Remote::new(&repository);
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    let before = head(&repository);
    let mut engine = opened(&repository);
    let _pushed = run(
        &mut engine,
        &repository,
        "sync.push",
        &json!({ "kind": "head" }),
        &json!({}),
    );

    let refused = undo(&mut engine, &repository);
    assert_eq!(refused["kind"], json!("refused"), "{refused}");
    assert_eq!(
        refused["message"],
        json!(
            "\"Push main to origin/main\" cannot be undone. A push changes the remote, which Undo does not take back."
        )
    );
    assert_eq!(head(&repository), before, "the commit pushed stays");
}

#[test]
fn undo_last__right_after_a_tag_pushed_which_changes_nothing_here_it_says_why_it_cannot() {
    let mut repository = repository();
    let _remote = testkit::Remote::new(&repository);
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    let before = head(&repository);
    let mut engine = opened(&repository);
    let _pushed = run(
        &mut engine,
        &repository,
        "tag.push",
        &json!({ "kind": "tag", "name": "v1.0" }),
        &json!({}),
    );

    let refused = undo(&mut engine, &repository);
    assert_eq!(refused["kind"], json!("refused"), "{refused}");
    assert_eq!(head(&repository), before, "the commit before the push stays");
}

#[test]
fn undo_last__a_branch_deleted_then_a_file_staged_is_still_created_again() {
    let repository = repository();
    let commit = repository.git(&["rev-parse", "feature"]).trim().to_owned();
    let mut engine = opened(&repository);
    let _deleted = run(
        &mut engine,
        &repository,
        "branch.delete",
        &json!({ "kind": "branch", "name": "feature" }),
        &json!({ "merged": { "kind": "flag", "value": false } }),
    );
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let _read = engine.ask("read/repo.read-state", &json!({ "repository": folder(&repository) }));
    let _staged = run(
        &mut engine,
        &repository,
        "changes.stage",
        &json!({ "kind": "file", "path": "a.txt" }),
        &json!({}),
    );

    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["rev-parse", "feature"]).trim(), commit);
}
