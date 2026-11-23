use std::path::PathBuf;

use model::ai::{AiModel, AiModels, AiProvider, CopilotNeed, Effort, Key};
use model::contract::{Answer, Answers, Context, FollowUp, Outcome, Query, QuestionKind, Step, Target};

use super::{
    AS_SET_UP, COPILOT_INSTALL, COPILOT_SIGN_IN, EFFORT, FIRST_MODEL, KEY, MODEL, MODELS, MODELS_WITH_KEY, SETTINGS,
    choose_model,
};
use crate::testing::{context, plan, question, state};

/// A context with the provider `anthropic` set up and the user's settings file.
fn set_up() -> Context {
    let mut context = context(state(), Target::Head);
    context.settings.ai.provider = Some(AiProvider::Anthropic);
    context.settings.user_settings_file = Some(PathBuf::from("settings.json"));
    context
}

/// Two models listed, the first named.
fn listed() -> Answer {
    Answer::AiModels(AiModels::Listed(vec![
        AiModel {
            id: String::from("model-a"),
            name: String::from("Model A"),
        },
        AiModel {
            id: String::from("model-b"),
            name: String::from("Model B"),
        },
    ]))
}

/// The settings file as written by a plan.
fn written(outcome: Outcome) -> String {
    let plan = plan(outcome);
    let [Step::Write { contents, .. }] = plan.steps.as_slice() else {
        panic!("writes the settings file: {plan:?}");
    };
    String::from_utf8(contents.clone()).unwrap()
}

#[test]
fn ai_choose_model__a_model_and_high_effort_are_written() {
    let context = set_up();
    let answers = Answers::new()
        .with(MODELS, listed())
        .with(MODEL, Answer::Choice(String::from("model:model-b")))
        .with(EFFORT, Answer::Choice(String::from("high")))
        .with(
            SETTINGS,
            Answer::Files(vec![Some(b"{\n  // Mine.\n  \"theme\": \"dark\"\n}\n".to_vec())]),
        );
    let text = written(choose_model(&context, &answers));
    let value = json::parse_with_comments(&text).unwrap();
    assert_eq!(value.get("ai.model").and_then(json::Value::as_str), Some("model-b"));
    assert_eq!(value.get("ai.effort").and_then(json::Value::as_str), Some("high"));
    assert!(text.contains("// Mine."), "{text}");
    let Outcome::Run(plan) = choose_model(&context, &answers) else {
        panic!("runs");
    };
    assert_eq!(plan.title.as_str(), "Resolve conflicts with Model B");
}

#[test]
fn ai_choose_model__the_first_model_and_as_set_up_remove_both_settings() {
    let mut context = set_up();
    context.settings.ai.model = String::from("model-b");
    context.settings.ai.effort = Some(Effort::High);
    let answers = Answers::new()
        .with(MODELS, listed())
        .with(MODEL, Answer::Choice(String::from(FIRST_MODEL)))
        .with(EFFORT, Answer::Choice(String::from(AS_SET_UP)))
        .with(
            SETTINGS,
            Answer::Files(vec![Some(
                b"{\n  \"theme\": \"dark\",\n  \"ai.model\": \"model-b\",\n  \"ai.effort\": \"high\"\n}\n".to_vec(),
            )]),
        );
    let text = written(choose_model(&context, &answers));
    assert!(!text.contains("ai.model") && !text.contains("ai.effort"), "{text}");
    assert!(text.contains("\"theme\": \"dark\""), "{text}");
}

#[test]
fn ai_choose_model__without_both_answers_nothing_is_written() {
    let mut context = set_up();
    context.settings.ai.model = String::from("model-b");
    let read = Answers::new().with(MODELS, listed());
    let asked = question(choose_model(&context, &read));
    assert_eq!(asked.key, MODEL);
    assert_eq!(asked.default, Some(Answer::Choice(String::from("model:model-b"))));
    let QuestionKind::Pick { choices } = &asked.kind else {
        panic!("a pick");
    };
    assert_eq!(choices.len(), 3);
    assert_eq!(choices[0].label.as_str(), "The first model of the provider");
    assert_eq!(
        choices[0].description.as_ref().map(model::contract::Text::as_str),
        Some("Model A")
    );
    let picked = read.with(MODEL, Answer::Choice(String::from(FIRST_MODEL)));
    let effort = question(choose_model(&context, &picked));
    assert_eq!(effort.key, EFFORT);
    assert_eq!(effort.default, Some(Answer::Choice(String::from(AS_SET_UP))));
}

#[test]
fn a_key_is_asked_when_the_provider_wants_one_and_kept_once_chosen() {
    let context = set_up();
    let first = choose_model(&context, &Answers::new());
    assert!(
        matches!(first, Outcome::Read(MODELS, Query::AiModels { key: None, .. })),
        "{first:?}"
    );
    let needed = Answers::new().with(MODELS, Answer::AiModels(AiModels::KeyNeeded));
    let asked = question(choose_model(&context, &needed));
    assert!(matches!(asked.kind, QuestionKind::Secret { .. }));
    let typed = needed.with(KEY, Answer::Text(String::from(" sk-test ")));
    let again = choose_model(&context, &typed);
    assert!(
        matches!(&again, Outcome::Read(MODELS_WITH_KEY, Query::AiModels { key: Some(key), .. }) if key.0 == "sk-test"),
        "{again:?}"
    );
    let answers = typed
        .with(MODELS_WITH_KEY, listed())
        .with(MODEL, Answer::Choice(String::from(FIRST_MODEL)))
        .with(EFFORT, Answer::Choice(String::from(AS_SET_UP)))
        .with(SETTINGS, Answer::Files(vec![None]));
    let plan = plan(choose_model(&context, &answers));
    assert_eq!(
        plan.after,
        [FollowUp::KeepAiKey(AiProvider::Anthropic, Key(String::from("sk-test")))]
    );
}

#[test]
fn no_model_listed_and_no_provider_are_refused() {
    let context = set_up();
    let none = Answers::new().with(MODELS, Answer::AiModels(AiModels::Listed(Vec::new())));
    let Outcome::Refuse(refusal) = choose_model(&context, &none) else {
        panic!("refused");
    };
    assert_eq!(
        refusal.message.as_str(),
        "No language model is available from the AI provider. Set one up in the settings."
    );
    let mut unset = set_up();
    unset.settings.ai.provider = None;
    assert!(matches!(choose_model(&unset, &none), Outcome::Refuse(_)));
}

#[test]
fn ai_choose_model__under_a_host_with_models_no_provider_is_needed_and_the_host_writes_the_two_settings() {
    let mut context = context(state(), Target::Head);
    context.settings.uses_host_models = true;
    let asked = choose_model(&context, &Answers::new());
    assert!(
        matches!(asked, Outcome::Read(key, Query::AiModels { .. }) if key == MODELS),
        "{asked:?}"
    );
    let answers = Answers::new()
        .with(MODELS, listed())
        .with(MODEL, Answer::Choice(String::from("model:model-b")))
        .with(EFFORT, Answer::Choice(String::from("high")));
    let plan = plan(choose_model(&context, &answers));
    assert!(plan.steps.is_empty(), "no settings file is written: {plan:?}");
    let written: Vec<_> = plan
        .after
        .iter()
        .map(|follow_up| match follow_up {
            FollowUp::WriteSetting(setting) => (setting.key.as_str(), setting.value.as_deref()),
            other => panic!("only settings written: {other:?}"),
        })
        .collect();
    assert_eq!(
        written,
        [("ai.model", Some("\"model-b\"")), ("ai.effort", Some("\"high\""))]
    );
    let first = Answers::new()
        .with(MODELS, listed())
        .with(MODEL, Answer::Choice(String::from(FIRST_MODEL)))
        .with(EFFORT, Answer::Choice(String::from(AS_SET_UP)));
    let removed = plan_of_settings(&choose_model(&context, &first));
    assert_eq!(
        removed,
        [(String::from("ai.model"), None), (String::from("ai.effort"), None)]
    );
}

/// The settings a plan has the host write.
fn plan_of_settings(outcome: &Outcome) -> Vec<(String, Option<String>)> {
    let Outcome::Run(plan) = outcome else {
        panic!("runs: {outcome:?}");
    };
    plan.after
        .iter()
        .filter_map(|follow_up| match follow_up {
            FollowUp::WriteSetting(setting) => Some((setting.key.clone(), setting.value.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn ai_choose_model__github_copilot_signed_out_or_not_installed_refuses_with_sign_in_or_how_to_install() {
    let mut context = set_up();
    context.settings.ai.provider = Some(AiProvider::Copilot);
    for (need, offered) in [
        (CopilotNeed::SignIn, (COPILOT_SIGN_IN, "Sign in")),
        (CopilotNeed::Install, (COPILOT_INSTALL, "How to install")),
        (CopilotNeed::Update, (COPILOT_INSTALL, "How to install")),
    ] {
        let answers = Answers::new().with(MODELS, Answer::AiModels(AiModels::CopilotNeeds(need)));
        let Outcome::Refuse(refusal) = choose_model(&context, &answers) else {
            panic!("refuses");
        };
        assert_eq!(refusal.message.as_str(), need.sentence());
        let (feature, label) = refusal.offer.unwrap();
        assert_eq!((feature, label.as_str()), offered);
    }
    // No key is asked of GitHub Copilot: the models are read as they are.
    assert!(matches!(
        choose_model(&context, &Answers::new()),
        Outcome::Read(MODELS, _)
    ));
}
