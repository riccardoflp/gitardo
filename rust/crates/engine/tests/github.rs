//! The GitHub token as the engine reads it for a feature (`github.token`): from its credential store, or from git's
//! credential helper, with what the log says of it; and, under a host, the host's token and models in their place. The
//! helper is the test's own, in a throwaway repository, and the store is in memory: neither the user's helper nor the
//! user's store is touched.

#![cfg(test)]
#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use engine::SecretStore as _;
use engine::ai::AiError;
use engine::{Asker, CommandLog, Engine, Finished, FixedClock, GITHUB_TOKEN, Host, LogKind, LogLevel, MemorySecrets};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::ai::{AiModel, AiModels, AiSettings};
use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FeatureId, FeatureSettings, FollowUp, Outcome, Plan, Query, Question,
    Target, Text, Undo,
};
use model::github::{HelperAsked, Token};
use testkit::Repository;

const TOKEN: AnswerKey = AnswerKey("token");

/// What the helper of the test prints for `get`.
const HELPER: &str = "!f() { echo username=ada; echo password=token-from-helper; }; f";

struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// A feature that reads the token, allowed to ask git's helper, and keeps the token it found.
fn read_and_keep(_context: &Context, answers: &Answers) -> Outcome {
    match answers.get(TOKEN) {
        Some(Answer::Token(token)) => Outcome::Run(Plan {
            title: Text::from("Keep the token"),
            steps: Vec::new(),
            danger: Danger::None,
            undo: Undo::Impossible(Text::from("A test plan.")),
            after: token.iter().cloned().map(FollowUp::KeepGitHubToken).collect(),
            on_failure: Vec::new(),
        }),
        _ => Outcome::Read(
            TOKEN,
            Query::GitHubToken {
                helper: HelperAsked::Allowed,
            },
        ),
    }
}

/// A repository whose only credential helper is the test's, and an engine for it that reads no configuration of the
/// system or of the user, logs at the debug level and keeps secrets in memory.
fn setup() -> (Repository, Engine, MemorySecrets, Context) {
    let repository = Repository::new();
    let _ = repository.git(&["config", "--add", "credential.helper", ""]);
    let _ = repository.git(&["config", "--add", "credential.helper", HELPER]);
    let global = repository.path().parent().unwrap().join("empty.gitconfig");
    fs::write(&global, "").unwrap();
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", global.as_os_str());
    let mut engine = Engine::new(
        git.clone(),
        CommandLog::new(LogLevel::Debug),
        Box::new(FixedClock(Timestamp(0))),
    );
    let secrets = MemorySecrets::default();
    engine.keep_secrets_in(Box::new(secrets.clone()));
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

fn lines(engine: &Engine) -> Vec<(LogKind, String)> {
    engine
        .log()
        .entries()
        .map(|entry| (entry.kind.clone(), entry.line.clone()))
        .collect()
}

#[test]
fn github_token__the_command_log_at_debug_shows_git_credential_fill_never_what_it_printed() {
    let (_repository, mut engine, secrets, context) = setup();
    let _ran = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    let logged = lines(&engine);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert!(matches!(logged[0].0, LogKind::Read(_)));
    assert_eq!(logged[0].1, "git credential fill");
    assert!(logged.iter().all(|(_, line)| !line.contains("token-from-helper")));
    assert_eq!(
        secrets.kept().get(GITHUB_TOKEN).map(String::as_str),
        Some("token-from-helper"),
        "the token found is kept"
    );
}

#[test]
fn github_token__a_token_kept_in_the_store_is_read_without_git() {
    let (_repository, mut engine, secrets, context) = setup();
    secrets.write(GITHUB_TOKEN, "kept-token").unwrap();
    let _ran = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    assert_eq!(lines(&engine), []);
    assert_eq!(secrets.kept().get(GITHUB_TOKEN).map(String::as_str), Some("kept-token"));
}

/// A host that gives a token, or none, and two models, and says what it was asked.
struct GivingHost {
    token: Option<&'static str>,
    asked: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl Host for GivingHost {
    fn github_token(&mut self, ask: bool) -> Option<Token> {
        self.asked.lock().unwrap().push(format!("token, ask {ask}"));
        self.token.map(|token| Token(token.to_owned()))
    }

    fn language_models(&mut self) -> Result<Vec<AiModel>, AiError> {
        self.asked.lock().unwrap().push(String::from("models"));
        Ok(["First", "Second"]
            .map(|name| AiModel {
                id: name.to_lowercase(),
                name: name.to_owned(),
            })
            .to_vec())
    }
}

/// A feature that reads the models, with no provider set up and no key.
fn read_models(_context: &Context, answers: &Answers) -> Outcome {
    match answers.get(TOKEN) {
        Some(Answer::AiModels(AiModels::Listed(models))) => Outcome::Run(Plan {
            title: Text::from(format!("Listed {}", models.len())),
            steps: Vec::new(),
            danger: Danger::None,
            undo: Undo::Impossible(Text::from("A test plan.")),
            after: vec![FollowUp::KeepAiKey(
                model::ai::AiProvider::Anthropic,
                model::ai::Key(String::from("typed")),
            )],
            on_failure: Vec::new(),
        }),
        _ => Outcome::Read(
            TOKEN,
            Query::AiModels {
                settings: AiSettings::default(),
                key: None,
            },
        ),
    }
}

#[test]
fn github_token__under_a_host_the_hosts_token_is_used_and_neither_git_nor_the_store_is() {
    let (_repository, mut engine, secrets, context) = setup();
    let asked = std::sync::Arc::default();
    engine.ask_host_with(Box::new(GivingHost {
        token: Some("host-token"),
        asked: std::sync::Arc::clone(&asked),
    }));
    let _ran = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    assert_eq!(*asked.lock().unwrap(), ["token, ask true"]);
    assert_eq!(lines(&engine), [], "git credential fill does not run");
    assert_eq!(
        secrets.kept(),
        BTreeMap::default(),
        "the host keeps its token, not the engine"
    );
}

#[test]
fn github_token__under_a_host_backing_out_of_signing_in_backs_out_of_the_feature() {
    let (_repository, mut engine, _secrets, context) = setup();
    engine.ask_host_with(Box::new(GivingHost {
        token: None,
        asked: std::sync::Arc::default(),
    }));
    let finished = engine
        .run_feature(FeatureId("github.token"), read_and_keep, &context, &mut RunsAll)
        .unwrap();
    assert_eq!(finished, Finished::BackedOut);
}

#[test]
fn ai_choose_model__under_a_host_its_models_are_listed_without_a_key_and_no_key_is_kept() {
    let (_repository, mut engine, secrets, context) = setup();
    let asked = std::sync::Arc::default();
    engine.ask_host_with(Box::new(GivingHost {
        token: None,
        asked: std::sync::Arc::clone(&asked),
    }));
    let finished = engine
        .run_feature(FeatureId("ai.choose-model"), read_models, &context, &mut RunsAll)
        .unwrap();
    let Finished::Ran { plan, .. } = finished else {
        panic!("it ran: {finished:?}");
    };
    assert_eq!(plan.title.as_str(), "Listed 2");
    assert_eq!(*asked.lock().unwrap(), ["models"]);
    assert_eq!(secrets.kept(), BTreeMap::default());
}
