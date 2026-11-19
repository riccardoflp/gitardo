//! Milestone M31, Ask Gitardo in the Chat of VS Code: `read/help.ask` asks the two requests of a question of the model
//! chosen in the Chat with `host/languageModel`, the second under the id of the read, and answers with the answer read
//! with the guide of the extension. No repository is opened for it.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::time::Duration;

use protocol::messages::VERSION;
use serde_json::{Value, json};

use crate::harness::Engine;

/// An engine whose extension answers the questions to its language models.
fn engine() -> Engine {
    let mut engine = Engine::start();
    let initialized = engine.ask(
        "initialize",
        &json!({
            "protocol": VERSION,
            "client": { "name": "gitardo-vscode", "version": "0.0.0" },
            "settings": { "ai.model": "the-settings-model", "ai.effort": "high" },
            "host": { "languageModel": true },
            "logLevel": "info"
        }),
    );
    assert!(initialized.get("result").is_some(), "{initialized}");
    engine
}

/// The text of the one message of a question to the model.
fn prompt(asked: &Value) -> &str {
    asked["params"]["messages"][0]["text"].as_str().unwrap()
}

/// The first request has the question and the earlier ones of the Chat; the second the cards chosen, with the
/// places of the extension, and the question.
fn prompts_have_the_question_and_the_guide_of_the_extension(asked: &[Value]) {
    assert!(prompt(&asked[0]).contains("Question: how do I put my changes aside"));
    assert!(prompt(&asked[0]).contains("Question: what is a branch\nAnswer: A line of commits."));
    let second = prompt(&asked[1]);
    assert!(second.contains("Feature stash.push (Stash all changes)"), "{second}");
    assert!(
        second.contains("Source Control side bar"),
        "the places of the extension: {second}"
    );
    assert!(
        second.contains("<question>\nhow do I put my changes aside\n</question>"),
        "{second}"
    );
}

#[test]
fn help_ask__both_requests_go_to_the_model_chosen_in_the_chat_and_the_answer_is_read_with_the_extensions_guide() {
    let mut engine = engine();
    let id = engine.send(
        "read/help.ask",
        &json!({
            "question": "how do I put my changes aside",
            "earlier": [{ "question": "what is a branch", "answer": "A line of commits." }],
            "model": "chat-model"
        }),
    );
    let mut asked = Vec::new();
    let answer = engine.answer_asked(id, |request| {
        asked.push(request.clone());
        if asked.len() == 1 {
            json!({ "text": "stash.push\nsquash.commits", "model": "Chat model" })
        } else {
            json!({ "text": "Run [[stash.push]] from the Command Palette.", "model": "Chat model" })
        }
    });
    assert_eq!(
        answer["result"],
        json!({
            "kind": "answered",
            "markdown": "Run **Stash all changes** from the Command Palette.",
            "named": ["stash.push"],
            "model": "Chat model"
        }),
        "{answer}"
    );
    assert_eq!(asked.len(), 2, "{asked:?}");
    for question in &asked {
        let params = &question["params"];
        assert_eq!(question["method"], json!("host/languageModel"));
        assert_eq!(params["purpose"], json!("help"));
        // The model of the Chat, not the one of the settings, and no effort.
        assert_eq!(
            (&params["model"], &params["effort"]),
            (&json!("chat-model"), &Value::Null),
            "{params}"
        );
    }
    // The cards chosen are not shown in the Chat; the answer is, under the read, with the titles of the cards chosen.
    assert_eq!(asked[0]["params"]["read"], Value::Null);
    assert!(asked[0]["params"].get("features").is_none(), "{}", asked[0]);
    assert_eq!(asked[1]["params"]["read"], json!(id));
    assert_eq!(
        asked[1]["params"]["features"],
        json!([
            { "id": "stash.push", "title": "Stash all changes" },
            { "id": "squash.commits", "title": "Squash commits into one" }
        ])
    );
    prompts_have_the_question_and_the_guide_of_the_extension(&asked);
}

#[test]
fn help_ask__what_the_host_refuses_is_said_in_its_words_and_no_second_request_is_sent() {
    let mut engine = engine();
    let id = engine.send("read/help.ask", &json!({ "question": "how do I fetch" }));
    let refused = "No language model is available in VS Code.";
    let mut asked = 0;
    let answer = engine.answer_asked(id, |_| {
        asked += 1;
        json!({ "refused": refused })
    });
    assert_eq!(
        answer["result"],
        json!({ "kind": "failed", "message": refused }),
        "{answer}"
    );
    assert_eq!(asked, 1);
}

#[test]
fn help_ask__cancelling_the_read_while_the_cards_are_chosen_cancels_the_question_and_sends_no_second() {
    let mut engine = engine();
    let id = engine.send("read/help.ask", &json!({ "question": "how do I fetch" }));
    let question = engine.request_of("host/languageModel");
    engine.notify("$/cancel", &json!({ "id": id }));
    let answer = engine.answer(id);
    assert_eq!(answer["error"]["code"], json!(7), "{answer}");
    let cancels = engine.notifications("$/cancel", Duration::from_millis(300));
    assert_eq!(cancels.len(), 1, "{cancels:?}");
    assert_eq!(cancels[0]["params"]["id"], question["id"]);
    let later = engine.notifications("host/languageModel", Duration::from_millis(300));
    assert!(later.is_empty(), "{later:?}");
}
