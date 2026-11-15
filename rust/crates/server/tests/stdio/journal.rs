//! Undo of what the engine ran, kept in its journal, and of what was done outside it, read from the reflogs, all taken
//! back by `feature/run` of `undo.last`.

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};

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
