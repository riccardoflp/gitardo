//! `feature/run` of `branch.create`, its questions asked of the test as the extension would answer them, the name checked
//! with `question/validate`, and the errors of a feature that is not there or of a repository busy.

use std::time::Duration;

use serde_json::{Value, json};
use testkit::Repository;

use crate::harness::{Engine, folder, repository};

/// The engine started, initialized with these settings, and the repository opened.
fn opened(repository: &Repository, settings: &Value) -> Engine {
    let mut engine = Engine::start();
    let answer = engine.ask(
        "initialize",
        &json!({
            "protocol": protocol::messages::VERSION,
            "client": { "name": "gitardo-vscode", "version": "0.0.0" },
            "settings": settings,
            "host": {},
            "logLevel": "info"
        }),
    );
    assert!(answer.get("result").is_some(), "{answer}");
    let _state = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

/// The answers the extension gives to the questions of `branch.create`: the name, and whether to check it out.
fn answering(name: &str, mode: &str) -> impl FnMut(&Value) -> Value {
    let (name, mode) = (name.to_owned(), mode.to_owned());
    move |request| {
        assert_eq!(request["method"], json!("feature/ask"), "{request}");
        match request["params"]["question"]["key"].as_str() {
            Some("name") => json!({ "answer": { "kind": "text", "text": name } }),
            Some("mode") => json!({ "answer": { "kind": "choice", "identifier": mode } }),
            other => panic!("no answer for {other:?}"),
        }
    }
}

/// The lines of the log the engine sent.
fn logged(engine: &mut Engine) -> Vec<String> {
    engine
        .notifications("log/line", Duration::from_millis(300))
        .iter()
        .filter_map(|line| line["params"]["entry"]["line"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn branch_create_on_a_commit_asks_the_name_and_whether_to_check_it_out_runs_its_plan_and_sends_the_state_ran() {
    let repository = repository();
    let first = repository.git(&["rev-parse", "HEAD~1"]).trim().to_owned();
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "commit", "commit": first } }),
    );
    let mut asked = Vec::new();
    let mut answer = answering("topic", "checkout");
    let finished = engine.answer_asked(id, |request| {
        asked.push(request["params"]["question"].clone());
        answer(request)
    });
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    assert_eq!(asked[0]["kind"]["kind"], json!("text"), "{asked:?}");
    assert_eq!(asked[0]["kind"]["validation"]["kind"], json!("referenceName"));
    assert_eq!(
        asked[0]["kind"]["validation"]["takenMessage"],
        json!("A branch with that name already exists")
    );
    assert_eq!(repository.git(&["symbolic-ref", "--short", "HEAD"]).trim(), "topic");
    assert_eq!(repository.git(&["rev-parse", "HEAD"]).trim(), first);

    let follow_ups = engine.notifications("feature/followUps", Duration::from_millis(300));
    assert_eq!(follow_ups[0]["params"]["run"], json!(id));
    assert_eq!(follow_ups[0]["params"]["followUps"], json!([{ "kind": "selectHead" }]));
    let states = engine.notifications("repository/state", Duration::from_millis(300));
    assert_eq!(states[0]["params"]["reason"], json!("ran"), "{states:?}");
    assert_eq!(
        states[0]["params"]["state"]["head"]["branch"],
        json!("refs/heads/topic")
    );
    let lines = logged(&mut engine);
    assert!(
        lines.iter().any(|line| line.starts_with("git checkout -b topic")),
        "{lines:?}"
    );
}

#[test]
fn the_answers_given_with_feature_run_are_not_asked() {
    let repository = repository();
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "feature/run",
        &json!({
            "repository": folder(&repository),
            "feature": "branch.create",
            "target": { "kind": "head" },
            "answers": { "name": { "kind": "text", "text": "kept" }, "mode": { "kind": "choice", "identifier": "only" } }
        }),
    );
    let finished = engine.answer_asked(id, |request| panic!("asked {request}"));
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    assert_eq!(repository.git(&["symbolic-ref", "--short", "HEAD"]).trim(), "main");
    assert_eq!(repository.git(&["branch", "--list", "kept"]).trim(), "kept");
}

#[test]
fn question_validate_refuses_a_name_with_a_space_and_a_name_taken_with_the_engines_sentences() {
    let repository = repository();
    let mut engine = opened(&repository, &json!({}));
    let validation = json!({ "kind": "referenceName", "taken": ["main", "feature"], "takenMessage": "A branch with that name already exists" });
    let check = |engine: &mut Engine, text: &str| {
        let answer = engine.ask(
            "question/validate",
            &json!({ "repository": folder(&repository), "validation": validation, "text": text }),
        );
        answer["result"]["problem"].clone()
    };
    assert!(check(&mut engine, "feature x").is_string());
    assert_eq!(
        check(&mut engine, "main"),
        json!("A branch with that name already exists")
    );
    assert_eq!(check(&mut engine, "feature/x"), Value::Null);
}

#[test]
fn with_branches_format_set_the_name_is_proposed_from_the_format_and_the_inputs_are_remembered() {
    let repository = repository();
    let settings = json!({
        "branches.format": "${input:ticket}/${base}",
        "branches.inputs": [{ "id": "ticket", "type": "promptString", "description": "Ticket" }]
    });
    let mut engine = opened(&repository, &settings);
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" }, "inputs": { "ticket": "T-1" } }),
    );
    let mut proposed = Value::Null;
    let finished = engine.answer_asked(id, |request| {
        let question = &request["params"]["question"];
        match question["key"].as_str() {
            Some("way") => json!({ "answer": { "kind": "choice", "identifier": "format" } }),
            Some("name") => {
                proposed = question["default"].clone();
                json!({ "answer": question["default"] })
            }
            Some("mode") => json!({ "answer": { "kind": "choice", "identifier": "only" } }),
            _ => json!({ "answer": question["default"] }),
        }
    });
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    assert_eq!(proposed, json!({ "kind": "text", "text": "T-1/main" }));
    let follow_ups = engine.notifications("feature/followUps", Duration::from_millis(300));
    assert_eq!(
        follow_ups[0]["params"]["followUps"],
        json!([{ "kind": "rememberInputs", "values": { "ticket": "T-1" } }])
    );
}

#[test]
fn a_branch_checked_out_where_there_are_submodules_ends_with_their_update() {
    let mut inner = Repository::new();
    let _inner = inner.commit("Add b", &[("b.txt", "b\n")]);
    let mut repository = repository();
    let inner_folder = folder(&inner);
    let _added = repository.git(&[
        "-c",
        "protocol.file.allow=always",
        "submodule",
        "add",
        &inner_folder,
        "inner",
    ]);
    let _commit = repository.commit("Add the submodule", &[]);
    let mut engine = opened(&repository, &json!({ "submodules.updateOnCheckout": true }));
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" } }),
    );
    let finished = engine.answer_asked(id, answering("with-inner", "checkout"));
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    let steps = finished["result"]["steps"].as_array().cloned().unwrap_or_default();
    assert!(
        steps
            .iter()
            .any(|line| line.as_str().is_some_and(|line| line.contains("submodule update"))),
        "{steps:?}"
    );
}

#[test]
fn a_feature_that_is_not_there_answers_8_and_a_second_run_in_a_repository_busy_answers_6() {
    let repository = repository();
    let mut engine = opened(&repository, &json!({}));
    let unknown = engine.ask(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.teleport", "target": { "kind": "head" } }),
    );
    assert_eq!(unknown["error"]["code"], json!(8), "{unknown}");

    let first = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" } }),
    );
    let waiting = engine.request_of("feature/ask");
    let second = engine.ask(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "branch.create", "target": { "kind": "head" } }),
    );
    assert_eq!(second["error"]["code"], json!(6), "{second}");

    engine.notify("$/cancel", &json!({ "id": first }));
    let cancelled = engine.answer(first);
    assert_eq!(cancelled["error"]["code"], json!(7), "{cancelled} after {waiting}");
}
