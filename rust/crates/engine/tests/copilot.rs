//! The models of GitHub Copilot as the engine lists them for a feature (`ai.choose-model`, `ai.copilot`): through the
//! CLI of the window, a fake of `testkit`, with no key asked or kept.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::sync::Arc;

use engine::ai::{CopilotCli, CopilotStreams};
use engine::{Asker, CommandLog, Engine, Finished, FixedClock, LogLevel, MemorySecrets};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::ai::{AiModels, AiProvider, AiSettings, CopilotNeed};
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Query, Question, Refusal,
    Target, Text, Undo,
};
use testkit::{CopilotScript, FakeCopilot, Repository};

const MODELS: AnswerKey = AnswerKey("models");

struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A feature that lists the models of GitHub Copilot, and runs a plan named after them, or refuses with what Copilot
/// needs first.
fn list_copilot_models(_context: &Context, answers: &Answers) -> Outcome {
    match answers.get(MODELS) {
        Some(Answer::AiModels(AiModels::Listed(models))) => Outcome::Run(Plan {
            title: Text::from(
                models
                    .iter()
                    .map(|model| model.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            steps: Vec::new(),
            danger: Danger::None,
            undo: Undo::Impossible(Text::from("A test plan.")),
            after: Vec::new(),
            on_failure: Vec::new(),
        }),
        Some(Answer::AiModels(AiModels::CopilotNeeds(need))) => Outcome::Refuse(Refusal::saying(need.sentence())),
        _ => Outcome::Read(
            MODELS,
            Query::AiModels {
                settings: AiSettings {
                    provider: Some(AiProvider::Copilot),
                    ..AiSettings::default()
                },
                key: None,
            },
        ),
    }
}

/// An engine for a throwaway repository whose GitHub Copilot CLI is the fake.
fn setup(fake: &FakeCopilot) -> (Repository, Engine, MemorySecrets, Context) {
    let repository = Repository::new();
    let git = Git::new(repository.path());
    let mut engine = Engine::new(
        git.clone(),
        CommandLog::new(LogLevel::Debug),
        Box::new(FixedClock(Timestamp(0))),
    );
    let secrets = MemorySecrets::default();
    engine.keep_secrets_in(Box::new(secrets.clone()));
    let started = fake.clone();
    engine.ask_copilot_with(CopilotCli::over(move || {
        let (output, input) = started.start();
        Ok(CopilotStreams { output, input })
    }));
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let context = Context {
        repository: Arc::new(read_state(&git, &request).unwrap()),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    (repository, engine, secrets, context)
}

#[test]
fn ai_choose_model__with_github_copilot_the_models_of_the_subscription_are_listed_and_no_key_is_asked() {
    let fake = FakeCopilot::new(CopilotScript::default());
    let (_repository, mut engine, secrets, context) = setup(&fake);
    let finished = engine
        .run_feature(
            FeatureId("ai.choose-model"),
            list_copilot_models,
            &context,
            &mut RunsAll,
        )
        .unwrap();
    let Finished::Ran { plan, .. } = finished else {
        panic!("the models are not listed: {finished:?}");
    };
    assert_eq!(plan.title.as_str(), "GPT Test, Claude Test");
    assert_eq!(fake.methods(), ["connect", "auth.getStatus", "models.list"]);
    assert!(secrets.kept().is_empty(), "no key is kept for GitHub Copilot");
}

#[test]
fn ai_choose_model__with_github_copilot_signed_out_it_says_so() {
    let fake = FakeCopilot::new(CopilotScript {
        is_signed_in: false,
        ..CopilotScript::default()
    });
    let (_repository, mut engine, _secrets, context) = setup(&fake);
    let finished = engine
        .run_feature(
            FeatureId("ai.choose-model"),
            list_copilot_models,
            &context,
            &mut RunsAll,
        )
        .unwrap();
    let Finished::Refused(refusal) = finished else {
        panic!("it is not refused: {finished:?}");
    };
    assert_eq!(refusal.message.as_str(), CopilotNeed::SignIn.sentence());
}
