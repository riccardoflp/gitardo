//! The JSON of what version 1.11 adds for milestone M31, Ask Gitardo in the Chat of VS Code: `read/help.ask`, its
//! answer, and the titles of the features sent with the question to the model.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::contract::FeatureId;
use model::help::{App, Exchange, HelpAnswer, guide};
use protocol::ai::{LanguageModelParams, ModelMessageJson, PurposeJson, RoleJson};
use protocol::help::{FeatureTitleJson, HelpAnswerJson, HelpAskParams};
use protocol::messages::{VERSION, is_compatible};
use serde_json::json;

#[test]
fn help_ask__version_1_11_is_spoken_by_an_engine_of_version_1() {
    assert_eq!(VERSION, "1.11");
    assert!(is_compatible("1.10"));
    assert!(is_compatible(VERSION));
}

#[test]
fn help_ask__the_question_is_read_with_the_earlier_ones_of_the_chat_and_the_model_chosen() {
    let params: HelpAskParams = serde_json::from_value(json!({
        "question": "how do I put my changes aside",
        "earlier": [{ "question": "what is a stash", "answer": "Changes set aside." }],
        "model": "copilot-fast"
    }))
    .unwrap();
    assert_eq!(params.model.as_deref(), Some("copilot-fast"));
    let input = params.input();
    assert_eq!(input.question, "how do I put my changes aside");
    assert_eq!(
        input.earlier,
        [Exchange {
            question: String::from("what is a stash"),
            answer: String::from("Changes set aside."),
        }]
    );
    let alone: HelpAskParams = serde_json::from_value(json!({ "question": "how do I fetch" })).unwrap();
    assert_eq!((alone.earlier.len(), alone.model), (0, None));
}

#[test]
fn help_ask__the_answer_is_written_with_its_markdown_the_features_it_names_and_its_model() {
    let answer = HelpAnswer {
        markdown: String::from("Run **Stash all changes**."),
        named: vec![FeatureId("stash.push")],
        model: String::from("Fast model"),
    };
    assert_eq!(
        serde_json::to_value(HelpAnswerJson::from(&answer)).unwrap(),
        json!({ "kind": "answered", "markdown": "Run **Stash all changes**.", "named": ["stash.push"], "model": "Fast model" })
    );
    let failed = HelpAnswerJson::Failed {
        message: String::from("The model refused."),
    };
    assert_eq!(
        serde_json::to_value(&failed).unwrap(),
        json!({ "kind": "failed", "message": "The model refused." })
    );
}

#[test]
fn help_ask__the_titles_of_the_features_go_with_the_question_and_are_left_out_of_any_other() {
    let guide = guide(App::Extension);
    let stash = guide.card("stash.push").unwrap();
    let params = LanguageModelParams {
        purpose: PurposeJson::Help,
        model: Some(String::from("copilot-fast")),
        effort: None,
        messages: vec![ModelMessageJson {
            role: RoleJson::User,
            text: String::from("You answer the user of Gitardo"),
        }],
        read: None,
        features: vec![FeatureTitleJson::from(stash)],
    };
    let written = serde_json::to_value(&params).unwrap();
    assert_eq!(written["purpose"], json!("help"));
    assert_eq!(
        written["features"],
        json!([{ "id": "stash.push", "title": "Stash all changes" }])
    );
    let other = LanguageModelParams {
        features: Vec::new(),
        ..params
    };
    assert!(serde_json::to_value(&other).unwrap().get("features").is_none());
}
