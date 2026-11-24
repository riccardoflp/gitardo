//! The risks of the extension's own code before the engine ran its operations (`docs/extension/13-risks.md`), each
//! tried against the engine as the extension runs it: a repository where the risk would bite, the feature run through
//! `feature/run`, and what is left in the repository after. Each test fails while its risk is true.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use serde_json::{Value, json};
use testkit::{CONFLICT_FILE, Repository};

use crate::harness::{Engine, folder, repository};

/// The engine started, initialized and the repository opened.
fn opened(repository: &Repository) -> Engine {
    let mut engine = Engine::start();
    let _ = engine.initialize();
    let _state = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

/// Runs a feature with its answers given up front, confirming its plan when asked, and gives how it ended with the
/// plans it was asked to confirm.
fn run(engine: &mut Engine, repository: &Repository, feature: &str, (target, answers): (&Value, &Value)) -> Ran {
    // The engine reads the repository again before each operation, as the extension's watchers have it do.
    let _read = engine.ask("read/repo.read-state", &json!({ "repository": folder(repository) }));
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(repository), "feature": feature, "target": target, "answers": answers }),
    );
    let mut confirmed = Vec::new();
    let answer = engine.answer_asked(id, |request| match request["method"].as_str() {
        Some("feature/confirm") => {
            confirmed.push(request["params"]["plan"].clone());
            json!({ "run": true })
        }
        _ => panic!("{feature} asks nothing but to confirm: {request}"),
    });
    Ran { answer, confirmed }
}

/// How a feature run ended, and the plans it was asked to confirm, in order.
struct Ran {
    answer: Value,
    confirmed: Vec<Value>,
}

impl Ran {
    /// How it ended: `ran`, `refused`, or the error.
    fn kind(&self) -> &Value {
        &self.answer["result"]["kind"]
    }

    /// The danger of the first plan it was asked to confirm.
    fn danger(&self) -> &Value {
        &self.confirmed[0]["danger"]
    }
}

/// Whether a merge is under way in the repository.
fn is_merging(repository: &Repository) -> bool {
    repository.path().join(".git").join("MERGE_HEAD").exists()
}

#[test]
fn changes_unstage_all__during_a_merge_with_its_conflicts_resolved_unstages_and_the_merge_goes_on() {
    let mut repository = repository();
    repository.conflict();
    std::fs::write(repository.path().join(CONFLICT_FILE), "both\n").unwrap();
    let _resolved = repository.git(&["add", CONFLICT_FILE]);
    let mut engine = opened(&repository);

    let ran = run(
        &mut engine,
        &repository,
        "changes.unstage-all",
        (&json!({ "kind": "changes" }), &json!({})),
    );
    assert_eq!(ran.kind(), &json!("ran"), "{}", ran.answer);
    assert!(is_merging(&repository), "the merge is not ended by `git reset`");
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "");
    assert_eq!(
        std::fs::read_to_string(repository.path().join(CONFLICT_FILE)).unwrap(),
        "both\n"
    );
}

#[test]
fn commit_create__with_nothing_staged_and_a_file_in_conflict_refuses_and_stages_nothing() {
    let mut repository = repository();
    repository.conflict();
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let mut engine = opened(&repository);

    let ran = run(
        &mut engine,
        &repository,
        "commit.create",
        (
            &json!({ "kind": "head" }),
            &json!({ "message": { "kind": "text", "text": "Merge" }, "stage-all": { "kind": "confirmed" } }),
        ),
    );
    assert_eq!(ran.kind(), &json!("refused"), "{}", ran.answer);
    let unmerged = repository.git(&["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(
        unmerged.trim(),
        CONFLICT_FILE,
        "the file in conflict is not staged with its markers"
    );
    assert!(is_merging(&repository));
}

/// A repository whose `topic` changes `f.txt` in two commits, `b` into `B` then `B` into `BB`, and whose `main`,
/// checked out, changes the same line to `main` when `main_changes_b`; the commits of `topic`, oldest first.
fn picking(main_changes_b: bool) -> (Repository, [String; 2]) {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the files", &[("f.txt", "a\nb\nc\n"), ("g.txt", "x\n")]);
    repository.branch("topic");
    let main = if main_changes_b { "a\nmain\nc\n" } else { "a\nb\nc\n" };
    let _main = repository.commit("Change them on main", &[("f.txt", main), ("g.txt", "y\n")]);
    repository.checkout("topic");
    let first = repository.commit("Make b B", &[("f.txt", "a\nB\nc\n")]);
    let second = repository.commit("Make B BB", &[("f.txt", "a\nBB\nc\n")]);
    repository.checkout("main");
    (repository, [first, second])
}

/// The commits of a selection as the timeline gives them, newest first.
fn selected([first, second]: &[String; 2]) -> Value {
    json!({ "kind": "commits", "commits": [second, first] })
}

#[test]
fn selection_cherry_pick__a_series_that_will_stop_on_a_conflict_says_so_naming_the_file() {
    let (repository, commits) = picking(true);
    let mut engine = opened(&repository);
    let ran = run(
        &mut engine,
        &repository,
        "selection.cherry-pick",
        (&selected(&commits), &json!({})),
    );
    assert_eq!(ran.confirmed.len(), 1, "it asks first: {}", ran.answer);
    assert_eq!(
        ran.danger(),
        &json!({ "kind": "confirm", "reason": "This will stop with conflicts in 1 file:\nf.txt" })
    );
}

#[test]
fn selection_cherry_pick__a_series_whose_later_pick_conflicts_only_without_the_one_before_runs_without_asking() {
    let (repository, commits) = picking(false);
    let mut engine = opened(&repository);
    let ran = run(
        &mut engine,
        &repository,
        "selection.cherry-pick",
        (&selected(&commits), &json!({})),
    );
    assert_eq!(ran.kind(), &json!("ran"), "{}", ran.answer);
    assert!(ran.confirmed.is_empty(), "{:?}", ran.confirmed);
    assert_eq!(
        std::fs::read_to_string(repository.path().join("f.txt")).unwrap(),
        "a\nBB\nc\n"
    );
}

#[test]
fn cherry_pick_commit__a_merge_commit_that_will_stop_on_a_conflict_says_so() {
    let (mut repository, _commits) = picking(true);
    // `topic` merged into `side`, a branch from the first commit that changes nothing of `f.txt`.
    let first = repository
        .git(&["rev-list", "--max-parents=0", "HEAD"])
        .trim()
        .to_owned();
    let _side = repository.git(&["checkout", "--quiet", "-b", "side", &first]);
    let _own = repository.commit("Add h", &[("h.txt", "h\n")]);
    let _merged = repository.git(&["merge", "--quiet", "--no-ff", "--no-edit", "topic"]);
    let merge = repository.git(&["rev-parse", "HEAD"]).trim().to_owned();
    repository.checkout("main");
    let mut engine = opened(&repository);
    let ran = run(
        &mut engine,
        &repository,
        "cherry-pick.commit",
        (&json!({ "kind": "commit", "commit": merge }), &json!({})),
    );
    assert_eq!(ran.confirmed.len(), 1, "it asks first: {}", ran.answer);
    assert_eq!(
        ran.danger(),
        &json!({ "kind": "confirm", "reason": "This will stop with conflicts in 1 file:\nf.txt" })
    );
}

/// A cherry-pick of `topic`'s commit stopped on a conflict in `f.txt`, which is then resolved and staged.
fn picked_and_resolved() -> Repository {
    let (repository, [_, second]) = picking(true);
    let output = repository
        .command("git")
        .args(["cherry-pick", &second])
        .output()
        .unwrap();
    assert!(!output.status.success(), "the pick stops on its conflict");
    std::fs::write(repository.path().join("f.txt"), "a\nresolved\nc\n").unwrap();
    let _staged = repository.git(&["add", "f.txt"]);
    repository
}

#[test]
fn conflicts_skip_step__with_what_was_resolved_staged_it_warns_first() {
    let repository = picked_and_resolved();
    let mut engine = opened(&repository);
    let ran = run(
        &mut engine,
        &repository,
        "conflicts.skip-step",
        (&json!({ "kind": "head" }), &json!({})),
    );
    assert_eq!(
        ran.danger(),
        &json!({ "kind": "destructive", "reason": "What was resolved in this step is thrown away." })
    );
}

#[test]
fn conflicts_take_side__a_file_with_lines_typed_in_it_warns_first() {
    let mut repository = repository();
    repository.conflict();
    let marked = std::fs::read_to_string(repository.path().join(CONFLICT_FILE)).unwrap();
    std::fs::write(
        repository.path().join(CONFLICT_FILE),
        format!("{marked}typed by hand\n"),
    )
    .unwrap();
    let mut engine = opened(&repository);
    let ran = run(
        &mut engine,
        &repository,
        "conflicts.take-side",
        (
            &json!({ "kind": "file", "path": CONFLICT_FILE }),
            &json!({ "side": { "kind": "choice", "identifier": "current" } }),
        ),
    );
    assert_eq!(ran.danger()["kind"], json!("destructive"), "{}", ran.danger());
}
