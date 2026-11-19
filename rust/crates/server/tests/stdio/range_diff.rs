//! What a rewrite changed, as the extension asks it (1.11, M36): `range-diff.compare` run on a branch rebased, whose
//! follow-up opens the tab with the lines compared, `read/range-diff.read-pairs` given those lines, and **Compare First**
//! in the warning of a force push.

use std::time::Duration;

use serde_json::{Value, json};
use testkit::{Remote, Repository};

use crate::harness::{Engine, folder};
use crate::reads::{opened, result};

/// `feature` one commit above `main` and rebased in a terminal onto the commit `main` gained since, checked out.
fn rebased() -> Repository {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _b = repository.commit("Add b", &[("b.txt", "b\n")]);
    repository.checkout("main");
    let _c = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository.checkout("feature");
    let _rebased = repository.git(&["rebase", "--quiet", "main"]);
    repository
}

/// The follow-ups the engine sent once a feature ran.
fn follow_ups(engine: &mut Engine) -> Value {
    let sent = engine.notifications("feature/followUps", Duration::from_millis(300));
    assert_eq!(sent.len(), 1, "{sent:?}");
    sent[0]["params"]["followUps"].clone()
}

#[test]
fn range_diff_compare_on_a_branch_rebased_in_a_terminal_its_follow_up_opens_the_tab_with_the_lines_the_read_compares() {
    let repository = rebased();
    let mut engine = opened(&repository);
    let id = engine.send(
        "feature/run",
        &json!({
            "repository": folder(&repository),
            "feature": "range-diff.compare",
            "target": { "kind": "branch", "name": "feature" },
            "answers": {}
        }),
    );
    let finished = engine.answer_asked(id, |request| panic!("asked {request}"));
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    let follow_ups = follow_ups(&mut engine);
    let opens = &follow_ups[0];
    assert_eq!(opens["kind"], json!("openRangeDiff"), "{follow_ups}");
    assert_eq!(opens["title"], json!("What the rebase of feature changed"));
    assert_eq!(opens["branch"], json!("feature"));
    let read = result(
        &mut engine,
        &repository,
        "read/range-diff.read-pairs",
        json!({ "ranges": opens["ranges"].clone() }),
    );
    assert_eq!(read["counts"], json!("1 the same"), "{read}");
    assert_eq!(read["beforeCount"], json!(1));
    assert_eq!(read["pairs"][0]["kind"], json!("same"));
    assert_eq!(read["pairs"][0]["subject"], json!("Add b"));
}

#[test]
fn sync_push_force_with_lease_the_warning_of_a_branch_rebased_after_it_was_pushed_offers_compare_first() {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a\n")]);
    let _remote = Remote::new(&repository);
    repository.branch("feature");
    repository.checkout("feature");
    let _b = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _pushed = repository.git(&["push", "--quiet", "--set-upstream", "origin", "feature"]);
    let _amended = repository.git(&["commit", "--quiet", "--amend", "-m", "Add b, better"]);
    let mut engine = opened(&repository);
    let id = engine.send(
        "feature/run",
        &json!({
            "repository": folder(&repository),
            "feature": "sync.push-force-with-lease",
            "target": { "kind": "head" },
            "answers": {}
        }),
    );
    let mut offered = Value::Null;
    let finished = engine.answer_asked(id, |request| {
        offered = request["params"]["plan"]["after"].clone();
        json!({ "run": false })
    });
    assert_eq!(finished["result"]["kind"], json!("notConfirmed"), "{finished}");
    let compare = offered
        .as_array()
        .and_then(|after| {
            after
                .iter()
                .find(|follow_up| follow_up["kind"] == json!("offerBeforeRun"))
        })
        .unwrap_or_else(|| panic!("Compare first is offered: {offered}"));
    assert_eq!(compare["label"], json!("Compare first"));
    assert_eq!(compare["feature"], json!("range-diff.compare"));
    assert_eq!(compare["target"], json!({ "kind": "branch", "name": "feature" }));
}
