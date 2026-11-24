//! The JSON of what version 1.6 adds for AI and GitHub (area 6 of stage 4): the host questions to the language models,
//! the two reads that ask them, the setting a feature leaves for the host to write, and the comparison a feature is
//! run on.

#![cfg(test)]

use engine::ai::{AiError, ConflictReply};
use model::ai::{Confidence, ConflictAnswer};
use model::compare::ComparisonPart;
use model::contract::{FollowUp, SettingTarget, SettingWritten, Target};
use protocol::ai::{
    AskConflictParams, AskedConflictJson, ConflictAnswerJson, DraftJson, DraftParams, LanguageModelParams,
    LanguageModelResult, LanguageModelsResult, ModelMessageJson, PurposeJson, RoleJson,
};
use protocol::feature::{FollowUpJson, TargetJson};
use protocol::rpc::RequestId;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

#[test]
fn a_question_to_a_language_model_is_written_with_its_purpose_model_effort_messages_and_read() {
    let params = LanguageModelParams {
        purpose: PurposeJson::Conflict,
        model: Some(String::from("copilot-fast")),
        effort: Some(String::from("high")),
        messages: vec![ModelMessageJson {
            role: RoleJson::User,
            text: String::from("Resolve one merge conflict"),
        }],
        read: Some(RequestId::Number(12)),
        features: Vec::new(),
    };
    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({
            "purpose": "conflict",
            "model": "copilot-fast",
            "effort": "high",
            "messages": [{ "role": "user", "text": "Resolve one merge conflict" }],
            "read": 12
        })
    );
    round_trip(&params);
}

#[test]
fn the_answer_of_a_language_model_is_its_text_and_model_or_why_the_host_refused() {
    let answered: LanguageModelResult = serde_json::from_value(json!({ "text": "Fix it", "model": "Fast" })).unwrap();
    assert_eq!(
        answered,
        LanguageModelResult::Answered {
            text: String::from("Fix it"),
            model: String::from("Fast")
        }
    );
    let refused: LanguageModelResult = serde_json::from_value(json!({ "refused": "No model." })).unwrap();
    assert_eq!(
        refused,
        LanguageModelResult::Refused {
            refused: String::from("No model.")
        }
    );
    let models: LanguageModelsResult =
        serde_json::from_value(json!({ "models": [{ "id": "a", "name": "A" }] })).unwrap();
    round_trip(&models);
}

#[test]
fn the_reads_of_the_draft_and_of_a_conflict_are_read_and_answered_by_kind() {
    let draft: DraftParams = serde_json::from_value(json!({})).unwrap();
    assert!(draft.subjects.is_empty(), "the subjects may be left out");
    round_trip(&DraftJson::Drafted {
        message: String::from("Change a"),
    });
    assert_eq!(
        serde_json::to_value(DraftJson::Failed {
            message: String::from("No.")
        })
        .unwrap(),
        json!({ "kind": "failed", "message": "No." })
    );
    round_trip(&AskConflictParams {
        path: String::from("a.txt"),
        lines: vec![String::from("<<<<<<< HEAD")],
        conflict: AskedConflictJson {
            start: 0,
            end: 5,
            current: vec![String::from("ours")],
            incoming: vec![String::from("theirs")],
            base: None,
        },
    });
}

#[test]
fn what_asking_about_a_conflict_came_to_is_the_resolution_or_why_not_and_whether_the_rest_stops() {
    let reply = ConflictReply {
        model: String::from("Fast"),
        answer: ConflictAnswer {
            lines: vec![String::from("both")],
            reason: String::from("Kept both."),
            confidence: Confidence::Low,
        },
    };
    assert_eq!(
        serde_json::to_value(ConflictAnswerJson::of(&Ok(reply))).unwrap(),
        json!({ "kind": "answered", "model": "Fast", "lines": ["both"], "reason": "Kept both.", "confidence": "low" })
    );
    let missing = ConflictAnswerJson::of(&Err(AiError::ModelMissing(String::from("gone"))));
    assert!(
        matches!(
            missing,
            ConflictAnswerJson::Failed {
                stops_the_rest: true,
                ..
            }
        ),
        "{missing:?}"
    );
    let refused = serde_json::to_value(ConflictAnswerJson::of(&Err(AiError::Host(String::from("No model."))))).unwrap();
    assert_eq!(
        refused,
        json!({ "kind": "failed", "message": "No model.", "stopsTheRest": false })
    );
}

#[test]
fn a_setting_to_write_has_its_key_and_its_value_or_null_to_remove_it() {
    let after = vec![
        FollowUp::WriteSetting(Box::new(SettingWritten {
            key: String::from("ai.model"),
            value: Some(String::from("\"two\"")),
            target: SettingTarget::User,
        })),
        FollowUp::WriteSetting(Box::new(SettingWritten {
            key: String::from("ai.effort"),
            value: None,
            target: SettingTarget::User,
        })),
        FollowUp::WriteSetting(Box::new(SettingWritten {
            key: String::from("worktrees.linkFolders"),
            value: Some(String::from("[\"/node_modules/\"]")),
            target: SettingTarget::WorkspaceFolder,
        })),
    ];
    assert_eq!(
        serde_json::to_value(FollowUpJson::all_of(&after)).unwrap(),
        json!([
            { "kind": "writeSetting", "key": "ai.model", "value": "two", "target": "user" },
            { "kind": "writeSetting", "key": "ai.effort", "value": null, "target": "user" },
            { "kind": "writeSetting", "key": "worktrees.linkFolders", "value": ["/node_modules/"], "target": "workspaceFolder" }
        ])
    );
}

#[test]
fn a_comparison_target_is_read_as_the_whole_comparison_of_its_two_sides() {
    let target: TargetJson = serde_json::from_value(json!({
        "kind": "comparison",
        "first": { "name": "main", "side": { "kind": "commit", "commit": "a".repeat(40) }, "isFollowed": true, "isGone": false },
        "second": { "name": "Working tree", "side": { "kind": "workingTree" }, "isFollowed": true, "isGone": false },
        "mode": "sinceDiverged",
        "path": null,
        "mergeBase": null
    }))
    .unwrap();
    let Target::Comparison(comparison) = Target::try_from(target).unwrap() else {
        panic!("not a comparison");
    };
    assert_eq!(
        (comparison.first.name.as_str(), comparison.second.name.as_str()),
        ("main", "Working tree")
    );
    assert_eq!(comparison.part, ComparisonPart::Whole);
}
