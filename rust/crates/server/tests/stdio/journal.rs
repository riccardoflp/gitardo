//! Undo across the two paths of stage 3: operations run by git as the TypeScript of the extension runs them, recorded
//! with `journal/record`, and operations run by the engine, all taken back by `feature/run` of `undo.last`.

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

/// What is checked out, as `journal/record` takes it.
fn head(repository: &Repository) -> Value {
    let commit = repository.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let branch = repository.git(&["symbolic-ref", "-q", "HEAD"]).trim().to_owned();
    json!({ "commit": commit, "branch": if branch.is_empty() { Value::Null } else { json!(branch) } })
}

/// Records an operation run outside the engine.
fn record(engine: &mut Engine, repository: &Repository, title: &str, record: &Value, before: &Value) -> Value {
    let answer = engine.ask(
        "journal/record",
        &json!({ "repository": folder(repository), "title": title, "record": record, "before": before, "after": head(repository) }),
    );
    assert!(answer.get("result").is_some(), "{answer}");
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
fn a_branch_deleted_outside_then_one_created_by_the_engine_are_taken_back_newest_first() {
    let repository = repository();
    let mut engine = opened(&repository);
    let before = head(&repository);
    let commit = repository.git(&["rev-parse", "feature"]).trim().to_owned();
    let _deleted = repository.git(&["branch", "-D", "feature"]);
    let recorded = record(
        &mut engine,
        &repository,
        "Delete branch feature",
        &json!({ "kind": "branchDeleted", "name": "feature", "commit": commit }),
        &before,
    );
    assert!(recorded["number"].is_number(), "{recorded}");

    let id = engine.send(
        "feature/run",
        &json!({
            "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" },
            "answers": { "name": { "kind": "text", "text": "spike" }, "mode": { "kind": "choice", "identifier": "only" } }
        }),
    );
    let created = engine.answer_asked(id, |request| panic!("asked {request}"));
    assert_eq!(created["result"]["kind"], json!("ran"), "{created}");

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
fn a_stash_dropped_then_a_commit_made_outside_are_taken_back_the_commit_first() {
    let mut repository = repository();
    let mut engine = opened(&repository);
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let _stashed = repository.git(&["stash", "push", "-m", "wip"]);
    let stash = repository.git(&["rev-parse", "stash@{0}"]).trim().to_owned();
    let before = head(&repository);
    let _dropped = repository.git(&["stash", "drop"]);
    let _ = record(
        &mut engine,
        &repository,
        "Drop stash",
        &json!({ "kind": "stashDropped", "message": "On main: wip", "commit": stash }),
        &before,
    );
    let first = head(&repository);
    let _commit = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _ = record(&mut engine, &repository, "Commit", &json!({ "kind": "reflog" }), &first);

    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(head(&repository), first, "the commit is taken back first");
    assert_eq!(undo(&mut engine, &repository)["kind"], json!("ran"));
    assert_eq!(repository.git(&["rev-parse", "stash@{0}"]).trim(), stash);
}

#[test]
fn undo_right_after_a_push_says_it_cannot_take_it_back_and_takes_back_nothing() {
    let repository = repository();
    let mut engine = opened(&repository);
    let before = head(&repository);
    let _ = record(
        &mut engine,
        &repository,
        "Push main to origin",
        &json!({ "kind": "notUndoable", "reason": "A push changes the remote, which Undo does not take back." }),
        &before,
    );
    let refused = undo(&mut engine, &repository);
    assert_eq!(refused["kind"], json!("refused"), "{refused}");
    assert_eq!(
        refused["message"],
        json!("\"Push main to origin\" cannot be undone. A push changes the remote, which Undo does not take back.")
    );
    assert_eq!(head(&repository), before);
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
