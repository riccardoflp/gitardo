//! Area 6 of stage 4, AI and GitHub: the draft of a commit message and the question about a conflict asked of the
//! language model of the extension with `host/languageModel`, a question cancelled with the read that asked it, and
//! `ai.choose-model` choosing among the extension's models and leaving the two settings for it to write.

use std::fs;
use std::time::Duration;

use protocol::messages::VERSION;
use serde_json::{Value, json};
use testkit::{CONFLICT_FILE, Repository};

use crate::harness::{Engine, folder, repository};

/// An engine whose extension answers the questions to its language models, with these settings, and the repository
/// opened.
fn opened(repository: &Repository, settings: &Value) -> Engine {
    let mut engine = Engine::start();
    let initialized = engine.ask(
        "initialize",
        &json!({
            "protocol": VERSION,
            "client": { "name": "gitardo-vscode", "version": "0.0.0" },
            "settings": settings,
            "host": { "languageModel": true, "githubToken": true },
            "logLevel": "info"
        }),
    );
    assert!(initialized.get("result").is_some(), "{initialized}");
    let _ = engine.ask("repository/open", &json!({ "repository": folder(repository) }));
    engine
}

/// A repository with `a.txt` committed, then changed and staged.
fn staged_change() -> Repository {
    let mut repository = repository();
    let _ = repository.commit("Add a", &[("a.txt", "one\n")]);
    fs::write(repository.path().join("a.txt"), "two\n").unwrap();
    let _ = repository.git(&["add", "a.txt"]);
    repository
}

#[test]
fn commit_draft_message_the_staged_diff_is_asked_of_the_hosts_first_model_and_its_answer_cleaned() {
    let repository = staged_change();
    let mut engine = opened(&repository, &json!({ "ai.model": "chosen", "ai.effort": "high" }));
    let id = engine.send(
        "read/commit.draft-message",
        &json!({ "repository": folder(&repository), "subjects": ["Add a"] }),
    );
    let mut asked = Vec::new();
    let answer = engine.answer_asked(id, |request| {
        asked.push(request.clone());
        json!({ "text": "```\nChange a\n```", "model": "Fake model" })
    });
    assert_eq!(
        answer["result"],
        json!({ "kind": "drafted", "message": "Change a" }),
        "{answer}"
    );
    assert_eq!(asked.len(), 1, "{asked:?}");
    let params = &asked[0]["params"];
    assert_eq!(asked[0]["method"], json!("host/languageModel"));
    assert_eq!(params["purpose"], json!("commitMessage"));
    // The draft is asked of VS Code's first model, as the extension asked it: the model chosen is for conflicts.
    assert_eq!(
        (&params["model"], &params["effort"]),
        (&Value::Null, &Value::Null),
        "{params}"
    );
    assert_eq!(params["read"], json!(id));
    let prompt = params["messages"][0]["text"].as_str().unwrap();
    assert_eq!(params["messages"][0]["role"], json!("user"));
    assert!(prompt.contains("+two") && prompt.contains("Add a"), "{prompt}");
}

#[test]
fn commit_draft_message_with_no_changes_nothing_is_asked() {
    let mut repository = repository();
    let _ = repository.commit("Add a", &[("a.txt", "one\n")]);
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "read/commit.draft-message",
        &json!({ "repository": folder(&repository) }),
    );
    let answer = engine.answer_asked(id, |request| panic!("asked {request}"));
    assert_eq!(
        answer["result"],
        json!({ "kind": "failed", "message": "There are no changes to describe." }),
        "{answer}"
    );
}

#[test]
fn commit_draft_message_what_the_host_refuses_is_said_in_its_words() {
    let repository = staged_change();
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "read/commit.draft-message",
        &json!({ "repository": folder(&repository) }),
    );
    let answer = engine.answer_asked(
        id,
        |_| json!({ "refused": "No language model is available in VS Code." }),
    );
    assert_eq!(
        answer["result"],
        json!({ "kind": "failed", "message": "No language model is available in VS Code." }),
        "{answer}"
    );
}

/// The parameters of the question about the conflict of [`Repository::conflict`], as the review of the extension has
/// it: the file's lines with two lines before the conflict and one after it.
fn conflict_params(repository: &Repository) -> Value {
    json!({
        "repository": folder(repository),
        "path": CONFLICT_FILE,
        "lines": ["start();", "before();", "<<<<<<< HEAD", "ours", "=======", "theirs", ">>>>>>> conflicting", "end();"],
        "conflict": { "start": 2, "end": 7, "current": ["ours"], "incoming": ["theirs"], "base": ["base"] }
    })
}

#[test]
fn ai_ask_conflict_the_question_goes_to_the_chosen_model_with_its_effort_and_the_answer_is_read_and_trimmed() {
    let mut repository = repository();
    repository.conflict();
    let mut engine = opened(&repository, &json!({ "ai.model": "chosen", "ai.effort": "high" }));
    let id = engine.send("read/ai.ask-conflict", &conflict_params(&repository));
    let mut asked = Vec::new();
    let answer = engine.answer_asked(id, |request| {
        asked.push(request.clone());
        json!({
            "text": "<resolution>\nstart();\nbefore();\nours\ntheirs\n</resolution>\n<reason>Kept both.</reason>\n<confidence>high</confidence>",
            "model": "Chosen model"
        })
    });
    assert_eq!(
        answer["result"],
        json!({ "kind": "answered", "model": "Chosen model", "lines": ["ours", "theirs"], "reason": "Kept both.", "confidence": "high" }),
        "{answer}"
    );
    let params = &asked[0]["params"];
    assert_eq!(params["purpose"], json!("conflict"));
    assert_eq!(
        (&params["model"], &params["effort"]),
        (&json!("chosen"), &json!("high"))
    );
    assert_eq!(params["read"], json!(id));
    let prompt = params["messages"][0]["text"].as_str().unwrap();
    assert!(
        prompt.contains(&format!("Resolve one merge conflict in the file {CONFLICT_FILE}.")),
        "{prompt}"
    );
    assert!(prompt.contains("<base>\nbase\n</base>"), "{prompt}");
    assert!(prompt.contains("main"), "the engine names the sides itself: {prompt}");
}

#[test]
fn ai_ask_conflict_a_model_that_is_not_there_fails_in_the_hosts_words_and_leaves_the_rest() {
    let mut repository = repository();
    repository.conflict();
    let mut engine = opened(&repository, &json!({ "ai.model": "gone" }));
    let id = engine.send("read/ai.ask-conflict", &conflict_params(&repository));
    let refused = "The model chosen to resolve conflicts, gone, is not available in VS Code: choose another one.";
    let answer = engine.answer_asked(id, |_| json!({ "refused": refused }));
    assert_eq!(
        answer["result"],
        json!({ "kind": "failed", "message": refused, "stopsTheRest": false }),
        "{answer}"
    );
}

#[test]
fn ai_ask_conflict_cancelling_the_read_cancels_the_question_to_the_model_in_the_extension() {
    let mut repository = repository();
    repository.conflict();
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send("read/ai.ask-conflict", &conflict_params(&repository));
    let question = engine.request_of("host/languageModel");
    engine.notify("$/cancel", &json!({ "id": id }));
    let answer = engine.answer(id);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    let cancels = engine.notifications("$/cancel", Duration::from_millis(300));
    assert_eq!(cancels.len(), 1, "{cancels:?}");
    assert_eq!(cancels[0]["params"]["id"], question["id"]);
}

#[test]
fn ai_choose_model_the_hosts_models_are_listed_and_the_two_settings_left_for_the_host_to_write() {
    let mut repository = repository();
    let _ = repository.commit("First", &[("a.txt", "1\n")]);
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "ai.choose-model", "target": { "kind": "head" } }),
    );
    let mut asked = Vec::new();
    let finished = engine.answer_asked(id, |request| {
        asked.push(request["method"].clone());
        match request["method"].as_str() {
            Some("host/languageModels") => {
                json!({ "models": [{ "id": "one", "name": "One" }, { "id": "two", "name": "Two" }] })
            }
            Some("feature/ask") if request["params"]["question"]["key"] == json!("model") => {
                json!({ "answer": { "kind": "choice", "identifier": "model:two" } })
            }
            Some("feature/ask") => json!({ "answer": { "kind": "choice", "identifier": "high" } }),
            Some("feature/confirm") => json!({ "run": true }),
            _ => panic!("asked {request}"),
        }
    });
    assert_eq!(finished["result"]["kind"], json!("ran"), "{finished}");
    assert_eq!(asked.first(), Some(&json!("host/languageModels")), "{asked:?}");
    let follow_ups = engine.notifications("feature/followUps", Duration::from_millis(300));
    assert_eq!(
        follow_ups[0]["params"]["followUps"],
        json!([
            { "kind": "writeSetting", "key": "ai.model", "value": "two", "target": "user" },
            { "kind": "writeSetting", "key": "ai.effort", "value": "high", "target": "user" }
        ]),
        "{follow_ups:?}"
    );
}

#[test]
fn ai_choose_model_with_no_model_in_the_host_it_refuses_in_the_words_of_vs_code() {
    let mut repository = repository();
    let _ = repository.commit("First", &[("a.txt", "1\n")]);
    let mut engine = opened(&repository, &json!({}));
    let id = engine.send(
        "feature/run",
        &json!({ "repository": folder(&repository), "feature": "ai.choose-model", "target": { "kind": "head" } }),
    );
    let finished = engine.answer_asked(id, |_| json!({ "models": [] }));
    assert_eq!(finished["result"]["kind"], json!("refused"), "{finished}");
    let said = finished["result"].to_string();
    assert!(said.contains("No language model is available in VS Code."), "{said}");
}
