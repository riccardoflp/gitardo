//! The JSON of the messages of stage 2, `feature/run` and what it asks: a golden file of a question and a plan that a
//! person looked at, and every message read back as it was written.

#![cfg(test)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use model::contract::{
    Answer, AnswerKey, Danger, FollowUp, Plan, Question, QuestionKind, Step, Target, Text, Undo, Validation,
};
use protocol::feature::{
    AnswerJson, AskParams, AskResult, ConfirmParams, ConfirmResult, FeatureRunParams, FinishedJson, FollowUpJson,
    FollowUpsParams, PlanJson, QuestionJson, TargetJson, ValidateParams, ValidateResult, ValidationJson,
};
use protocol::rpc::RequestId;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use testkit::assert_golden;

/// The value with the fields of each object in the order of their names, as the golden file has them, whether
/// `serde_json` keeps the order they were written in or not: the library that lays out Mermaid asks it to, and a build
/// of this test has that or not depending on which crates are built with it.
fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(fields) => {
            let mut fields: Vec<_> = fields.into_iter().collect();
            fields.sort_by(|(left, _), (right, _)| left.cmp(right));
            serde_json::Value::Object(fields.into_iter().map(|(name, field)| (name, sorted(field))).collect())
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

/// Writes a value and reads it back, which must give the same value.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let read: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&read, value, "{text}");
}

/// The question of the name of `branch.create`, with the branches taken.
fn name_question() -> Question {
    Question {
        key: AnswerKey("name"),
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("feature/my-change")),
            validation: Validation::ReferenceName {
                taken: vec![String::from("main"), String::from("feature")],
                taken_message: Text::from("A branch with that name already exists"),
            },
        },
        title: Text::from("Create branch"),
        detail: None,
        default: Some(Answer::Text(String::from("T-1/main"))),
    }
}

/// The plan of `branch.create` checked out, with the inputs of the format to remember.
fn plan() -> Plan {
    Plan {
        title: Text::from("Create branch topic at main and check it out"),
        steps: vec![Step::Git {
            arguments: ["checkout", "-b", "topic", "main"].map(OsString::from).to_vec(),
            directory: PathBuf::from("/work/app"),
            input: None,
        }],
        danger: Danger::None,
        undo: Undo::ByReflog,
        after: vec![
            FollowUp::SelectHead,
            FollowUp::RememberInputs(BTreeMap::from([(String::from("ticket"), String::from("T-1"))])),
        ],
        on_failure: Vec::new(),
    }
}

#[test]
fn the_question_and_the_plan_of_branch_create_are_written_as_the_golden_file_says() {
    let message = json!({
        "ask": AskParams { run: RequestId::Number(4), question: QuestionJson::from(&name_question()) },
        "confirm": ConfirmParams { run: RequestId::Number(4), plan: PlanJson::from(&plan()) },
    });
    let mut text = serde_json::to_string_pretty(&sorted(message)).unwrap();
    text.push('\n');
    assert_golden(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/branch-create.json"),
        text,
    );
}

#[test]
fn every_message_of_feature_run_is_read_back_as_it_was_written() {
    let mut answers = BTreeMap::new();
    let _name = answers.insert(
        String::from("name"),
        AnswerJson::Text {
            text: String::from("topic"),
        },
    );
    let _mode = answers.insert(
        String::from("mode"),
        AnswerJson::Choice {
            identifier: String::from("checkout"),
        },
    );
    round_trip(&FeatureRunParams {
        repository: String::from("/work/app"),
        feature: String::from("branch.create"),
        target: TargetJson::Branch {
            name: String::from("main"),
        },
        answers,
        journal: true,
        inputs: BTreeMap::from([(String::from("ticket"), String::from("T-1"))]),
    });
    round_trip(&AskParams {
        run: RequestId::Number(4),
        question: QuestionJson::from(&name_question()),
    });
    round_trip(&AskResult {
        answer: Some(AnswerJson::Choices {
            identifiers: vec![String::from("a")],
        }),
    });
    round_trip(&ConfirmParams {
        run: RequestId::Number(4),
        plan: PlanJson::from(&plan()),
    });
    round_trip(&ConfirmResult { run: false });
    round_trip(&ValidateParams {
        repository: String::from("/work/app"),
        validation: ValidationJson::from(&Validation::Anything),
        text: String::from("x"),
    });
    round_trip(&ValidateResult { problem: None });
    round_trip(&FollowUpsParams {
        run: RequestId::Number(4),
        follow_ups: FollowUpJson::all_of(&plan().after),
    });
    let finished = engine_finished();
    round_trip(&finished);
}

/// How a run ended, as `feature/run` answers it.
fn engine_finished() -> FinishedJson {
    FinishedJson::Ran {
        plan: PlanJson::from(&plan()),
        steps: vec![String::from("git checkout -b topic main")],
        went_on_after: Vec::new(),
        waiting: Vec::new(),
    }
}

#[test]
fn a_target_and_an_answer_read_from_json_become_those_of_the_engine() {
    let target: TargetJson = serde_json::from_value(json!({ "kind": "commit", "commit": "a".repeat(40) })).unwrap();
    assert!(matches!(Target::try_from(target), Ok(Target::Commit(_))));
    let bad: TargetJson = serde_json::from_value(json!({ "kind": "commit", "commit": "nope" })).unwrap();
    assert!(Target::try_from(bad).is_err());
    let answer: AnswerJson = serde_json::from_value(json!({ "kind": "text", "text": "topic" })).unwrap();
    assert_eq!(Answer::from(answer), Answer::Text(String::from("topic")));
    let validation: ValidationJson =
        serde_json::from_value(json!({ "kind": "referenceName", "taken": ["main"], "takenMessage": "Taken" })).unwrap();
    assert_eq!(
        Validation::from(validation)
            .problem("main")
            .map(|text| text.to_string()),
        Some(String::from("Taken"))
    );
}

#[test]
fn an_operation_offered_crosses_with_its_target_and_answers_and_not_when_one_cannot() {
    let offered = |answer: Answer| {
        FollowUp::OfferRun(Box::new(model::contract::OfferedRun {
            label: Text::from("Delete anyway"),
            feature: model::contract::FeatureId("branch.delete"),
            target: Target::Branch(String::from("topic")),
            answers: model::contract::Answers::new().with(AnswerKey("merged"), answer),
            when_said: vec![String::from("not fully merged")],
        }))
    };
    let sent = FollowUpJson::all_of(&[offered(Answer::Flag(false))]);
    assert_eq!(
        serde_json::to_value(&sent).unwrap(),
        json!([{
            "kind": "offerRun",
            "label": "Delete anyway",
            "feature": "branch.delete",
            "target": { "kind": "branch", "name": "topic" },
            "answers": { "merged": { "kind": "flag", "value": false } },
        }])
    );
    let flag: AnswerJson = serde_json::from_value(json!({ "kind": "flag", "value": false })).unwrap();
    assert_eq!(Answer::from(flag), Answer::Flag(false));
    // A reading of the engine that cannot cross would make the operation run otherwise than offered.
    assert_eq!(FollowUpJson::all_of(&[offered(Answer::Commit(None))]), Vec::new());
}

#[test]
fn the_targets_of_the_conflicts_read_from_json_become_those_of_the_engine() {
    let resolutions = json!({
        "kind": "resolutions",
        "files": [{
            "path": "e.txt",
            "text": "a\n<<<<<<< HEAD\nb\n=======\nB\n>>>>>>> aside\nc\n",
            "conflicts": 1,
            "accepted": [{
                "source": { "kind": "ai", "model": "Chosen model", "reason": "Both", "confidence": "high" },
                "from": 1,
                "to": 6,
                "text": "b and B\n",
                "newLines": 1
            }]
        }]
    });
    let target: TargetJson = serde_json::from_value(resolutions).unwrap();
    round_trip(&target);
    let Ok(Target::Resolutions(read)) = Target::try_from(target) else {
        panic!("the resolutions")
    };
    let file = read.files.first().unwrap();
    assert_eq!(file.resolved().unwrap(), "a\nb and B\nc\n");
    assert_eq!(file.sources(), (0, 1, 0));

    let merge: TargetJson =
        serde_json::from_value(json!({ "kind": "mergeResult", "path": "b.txt", "unsaved": null, "marked": 1 }))
            .unwrap();
    round_trip(&merge);
    let Ok(Target::MergeResult(result)) = Target::try_from(merge) else {
        panic!("the file of a Merge page")
    };
    assert_eq!(
        (result.path.as_str(), result.unsaved, result.marked),
        ("b.txt", None, 1)
    );
}
